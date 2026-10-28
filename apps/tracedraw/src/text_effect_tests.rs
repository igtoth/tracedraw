//! Effects on text objects: blends between texts (and a text and a
//! shape), extrusions of text, and Break Apart of artistic text.

use crate::app::App;
use tracedraw_core::geometry::Shape as _;
use tracedraw_core::geometry::Vec2;
use tracedraw_core::{
    document::{spans_text, ShapeKind},
    geometry::{Point, Rect},
    live::Effect,
    Color, Fill, ShapeId,
};

fn text(app: &mut App, at: Point, s: &str) -> ShapeId {
    // Letters' outlines come from the text engine, as in the app.
    tracedraw_text::install();
    app.start_text(at, None);
    app.text_insert(s);
    let id = app.text_edit.as_ref().expect("editing").shape;
    app.finish_text();
    app.run(tracedraw_core::Command::SetFill {
        shapes: vec![id],
        fill: Fill::Solid(Color::rgb8(200, 30, 30)),
    });
    id
}

fn center(app: &App, id: ShapeId) -> Point {
    app.doc().find_shape(id).expect("shape").bounds().center()
}

#[test]
fn the_blend_tool_blends_two_texts_and_a_text_with_a_shape() {
    let mut app = App::headless();
    let a = text(&mut app, Point::new(20.0, 200.0), "ABC");
    let b = text(&mut app, Point::new(120.0, 120.0), "XYZ");
    // A press on a letter of the first text, a release on the second.
    app.selection = vec![a, b];
    let (sa, sb) = (center(&app, a), center(&app, b));
    assert!(
        app.hit_test_inside(sa).is_some(),
        "the text is hit at {sa:?}"
    );
    app.tool = crate::tools::Tool::Blend;
    app.drag = crate::app::Drag::Connector {
        from: app.hit_test_inside(sa).expect("start"),
        start: sa,
        current: sb,
    };
    app.end_drag();
    let blended = app.doc().find_shape(a).expect("a").clone();
    assert!(
        blended
            .effects
            .iter()
            .any(|e| matches!(e, Effect::Blend { .. })),
        "{:?}",
        blended.effects
    );
    let ev = tracedraw_core::live::evaluate(&blended);
    let steps: Vec<_> = ev.below.iter().chain(&ev.above).collect();
    assert!(!steps.is_empty(), "the blend draws its steps");
    // Each step keeps the letters apart (one outline part or more per
    // letter) and about the texts' size, not a tangle of one outline.
    let width = |id: ShapeId| app.doc().find_shape(id).expect("text").bounds().width();
    let (wa, wb) = (width(a), width(b));
    for s in steps {
        let path = s.page_path();
        let r = path.bounding_box();
        assert!(
            tracedraw_core::effects::subpaths(&path).len() >= 3,
            "{} parts",
            tracedraw_core::effects::subpaths(&path).len()
        );
        assert!(r.width() > wa.min(wb) * 0.7, "{r:?} vs {wa} {wb}");
    }
    // A text and a rectangle.
    let r = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(150.0, 30.0, 180.0, 60.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.selection = vec![b, r];
    app.blend_selection(5, 0.0, 0.0, 0.0);
    let s = app.doc().find_shape(b).expect("b").clone();
    let ev = tracedraw_core::live::evaluate(&s);
    assert_eq!(ev.below.len() + ev.above.len(), 5);
}

#[test]
fn text_can_be_extruded() {
    let mut app = App::headless();
    app.tool = crate::tools::Tool::Extrude;
    let id = text(&mut app, Point::new(40.0, 150.0), "3D");
    let c = center(&app, id);
    assert!(app.begin_extrude(c), "the extrusion starts on the text");
    app.drag_extrude(Point::new(c.x + 30.0, c.y + 20.0));
    app.extrude_drag = None;
    let s = app.doc().find_shape(id).expect("text").clone();
    assert!(
        s.effects
            .iter()
            .any(|e| matches!(e, Effect::Extrude { .. })),
        "{:?}",
        s.effects
    );
    let ev = tracedraw_core::live::evaluate(&s);
    assert!(!ev.below.is_empty(), "the sides are drawn behind the text");
}

#[test]
fn break_apart_splits_text_into_lines_words_then_letters() {
    let mut app = App::headless();
    let texts = |app: &App| -> Vec<(String, Point)> {
        app.selection
            .iter()
            .filter_map(|id| app.doc().find_shape(*id))
            .filter_map(|s| match &s.kind {
                ShapeKind::Text { spans, origin, .. } => Some((spans_text(spans), *origin)),
                _ => None,
            })
            .collect()
    };
    // Two lines: one object per line, the second below the first.
    let id = text(&mut app, Point::new(20.0, 200.0), "one two\nthree");
    app.select(vec![id]);
    app.break_apart();
    let got = texts(&app);
    assert_eq!(
        got.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(),
        ["one two", "three"]
    );
    assert!(got[1].1.y < got[0].1.y, "{got:?}");
    assert_eq!(app.engine.undo_label(), Some("Break Apart"));
    // One line: words, side by side, keeping the fill.
    let first = app.selection[0];
    app.select(vec![first]);
    app.break_apart();
    let got = texts(&app);
    assert_eq!(
        got.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert!(got[1].1.x > got[0].1.x + 1.0);
    assert!((got[1].1.y - got[0].1.y).abs() < 1e-6);
    for id in &app.selection {
        let s = app.doc().find_shape(*id).expect("piece");
        assert_eq!(s.fill, Fill::Solid(Color::rgb8(200, 30, 30)));
    }
    // One word: letters; the first keeps the object's place and id.
    let word = app.selection[0];
    app.select(vec![word]);
    app.break_apart();
    let got = texts(&app);
    assert_eq!(
        got.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(),
        ["o", "n", "e"]
    );
    assert!(app.selection.contains(&word));
    // Undo takes it back in one step.
    app.undo();
    assert!(app.doc().find_shape(word).is_some_and(
        |s| matches!(&s.kind, ShapeKind::Text { spans, .. } if spans_text(spans) == "one")
    ));
    // A single letter stays as it is.
    let one = text(&mut app, Point::new(60.0, 60.0), "x");
    app.select(vec![one]);
    app.break_apart();
    assert_eq!(texts(&app).len(), 1);
}

#[test]
fn text_pieces_follow_lines_words_and_characters() {
    use crate::ops::text_pieces;
    assert_eq!(text_pieces("ab cd\nef"), [(0, 5), (6, 8)]);
    assert_eq!(text_pieces("ab  cd"), [(0, 2), (4, 6)]);
    assert_eq!(text_pieces("中文字"), [(0, 1), (1, 2), (2, 3)]);
    assert!(text_pieces("").is_empty());
}

#[test]
fn an_input_method_composes_then_commits_cjk_text() {
    let mut app = App::headless();
    app.start_text(Point::new(30.0, 100.0), None);
    let ctx = crate::theme::ui_context();
    let send = |app: &mut App, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            app.text_keyboard(ui.ctx());
        });
        out.textures_delta.clear();
    };
    send(
        &mut app,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "zhong".into(),
            active_range_chars: None,
        })],
    );
    assert_eq!(app.ime_preedit, "zhong");
    assert_eq!(app.edit_text(), "", "nothing typed while composing");
    // Keys go to the input method while it composes.
    send(
        &mut app,
        vec![egui::Event::Key {
            key: egui::Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    send(
        &mut app,
        vec![egui::Event::Ime(egui::ImeEvent::Commit("中文".into()))],
    );
    assert!(app.ime_preedit.is_empty());
    assert_eq!(app.edit_text(), "中文");
    // The text keeps its letters; their outlines come from a font that
    // has them when the text's own font lacks them.
    let id = app.text_edit.as_ref().expect("editing").shape;
    app.finish_text();
    let s = app.doc().find_shape(id).expect("text").clone();
    if tracedraw_text::fonts()
        .families()
        .iter()
        .any(|f| f.contains("CJK"))
    {
        let r = s.page_path().bounding_box();
        assert!(r.width() > 1.0 && r.height() > 1.0, "{r:?}");
    }
}

#[test]
fn the_canvas_asks_for_the_input_method_while_text_is_typed() {
    let mut app = App::headless();
    app.start_text(Point::new(30.0, 100.0), None);
    let ctx = crate::theme::ui_context();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| {
        let painter = ui.painter().clone();
        crate::canvas::ime_at_caret(
            &app,
            &painter,
            egui::pos2(100.0, 90.0),
            egui::pos2(100.0, 110.0),
        );
    });
    out.textures_delta.clear();
    let ime = out
        .platform_output
        .ime
        .expect("the input method is allowed");
    assert!(ime.cursor_rect.contains(egui::pos2(100.0, 100.0)));
}

/// Bounds of `ids` together, as they are drawn on the page.
fn drawn_bounds(app: &App, ids: &[ShapeId]) -> Rect {
    ids.iter()
        .filter_map(|id| app.doc().shape_on_page(*id))
        .map(|s| s.visual_bounds())
        .reduce(|a, b| a.union(b))
        .expect("drawn")
}

fn near_rect(a: Rect, b: Rect, tol: f64) -> bool {
    (a.x0 - b.x0).abs() < tol
        && (a.y0 - b.y0).abs() < tol
        && (a.x1 - b.x1).abs() < tol
        && (a.y1 - b.y1).abs() < tol
}

/// Convert to Curves of an enveloped text gives the curve drawn as the
/// text was, its envelope part of it (#647).
#[test]
fn convert_to_curves_bakes_the_envelope_of_text() {
    let mut app = App::headless();
    let id = text(&mut app, Point::new(20.0, 100.0), "ABC");
    app.select(vec![id]);
    app.apply_envelope_preset(crate::envelope_edit::Preset::Arch);
    let drawn = drawn_bounds(&app, &[id]);
    app.convert_to_curves();
    let s = app.doc().shape_on_page(id).expect("shape");
    assert!(matches!(s.kind, ShapeKind::Path { .. }), "{:?}", s.kind);
    assert!(s.effects.is_empty(), "{:?}", s.effects);
    assert_eq!(s.fill, Fill::Solid(Color::rgb8(200, 30, 30)));
    assert!(
        near_rect(s.bounds(), drawn, 0.05),
        "{:?} {drawn:?}",
        s.bounds()
    );
}

/// An enveloped text with letters of their own colour keeps the colours:
/// it is drawn as one warped curve a colour, together as warped as the
/// text in one colour is (#912).
#[test]
fn an_envelope_keeps_the_colours_of_the_letters() {
    let mut app = App::headless();
    let id = text(&mut app, Point::new(20.0, 100.0), "ABC");
    let blue = Fill::Solid(Color::rgb8(0, 0, 255));
    let mut kind = app.doc().find_shape(id).expect("text").kind.clone();
    if let ShapeKind::Text { spans, .. } = &mut kind {
        let first = spans[0].clone();
        let mut b = first.clone();
        b.text = "B".into();
        b.fill = Some(blue.clone());
        let mut c = first.clone();
        c.text = "C".into();
        spans.clear();
        spans.push(tracedraw_core::TextSpan {
            text: "A".into(),
            ..first
        });
        spans.push(b);
        spans.push(c);
    }
    app.run(tracedraw_core::Command::SetShapeKind {
        shape: id,
        kind: kind.clone(),
    });
    app.select(vec![id]);
    app.apply_envelope_preset(crate::envelope_edit::Preset::Arch);
    let s = app.doc().find_shape(id).expect("text").clone();
    let ev = tracedraw_core::live::evaluate(&s);
    let ShapeKind::Group { children } = &ev.main.kind else {
        panic!("a curve a colour: {:?}", ev.main.kind);
    };
    let fills: Vec<&Fill> = children.iter().map(|c| &c.fill).collect();
    assert_eq!(fills, [&Fill::Solid(Color::rgb8(200, 30, 30)), &blue]);
    assert!(children
        .iter()
        .all(|c| matches!(c.kind, ShapeKind::Path { .. }) && c.effects.is_empty()));
    // The same warp as the text in one colour: the curves together cover
    // what it covers, and the blue "B" sits between the others.
    let mut plain = s.clone();
    if let ShapeKind::Text { spans, .. } = &mut plain.kind {
        spans.iter_mut().for_each(|sp| sp.fill = None);
    }
    let one = tracedraw_core::live::evaluate(&plain).main;
    assert!(matches!(one.kind, ShapeKind::Path { .. }));
    assert!(
        near_rect(ev.main.bounds(), one.bounds(), 1e-6),
        "{:?} {:?}",
        ev.main.bounds(),
        one.bounds()
    );
    let mut b = children[1].clone();
    b.transform_by(ev.main.transform);
    let mut rest = children[0].clone();
    rest.transform_by(ev.main.transform);
    let (bb, rb) = (b.bounds(), rest.bounds());
    assert!(bb.x0 > rb.x0 && bb.x1 < rb.x1, "{bb:?} {rb:?}");
    // The page draws the blue letter.
    let doc = app.doc();
    let pm = tracedraw_render::render_page_image(doc, doc.pages[0].id, 60.0).expect("page");
    let blue_px = pm
        .pixels()
        .iter()
        .filter(|p| p.blue() > 200 && p.red() < 60 && p.green() < 60)
        .count();
    assert!(blue_px > 10, "{blue_px}");
}

/// Break Apart of an enveloped text gives one curve per letter, each
/// warped as the whole text was and keeping its holes (#647); one undo
/// brings the text and its envelope back.
#[test]
fn break_apart_bakes_the_envelope_into_letter_curves() {
    let mut app = App::headless();
    let id = text(&mut app, Point::new(20.0, 100.0), "OBO");
    app.select(vec![id]);
    app.apply_envelope_preset(crate::envelope_edit::Preset::Arch);
    let drawn = drawn_bounds(&app, &[id]);
    app.break_apart();
    let pieces = app.selection.clone();
    assert_eq!(pieces.len(), 3, "{pieces:?}");
    assert!(pieces.contains(&id));
    let mut spans = Vec::new();
    for p in &pieces {
        let s = app.doc().shape_on_page(*p).expect("piece");
        let ShapeKind::Path { path, .. } = &s.kind else {
            panic!("a curve: {:?}", s.kind);
        };
        assert!(s.effects.is_empty(), "{:?}", s.effects);
        assert_eq!(s.fill, Fill::Solid(Color::rgb8(200, 30, 30)));
        let parts = tracedraw_core::effects::subpaths(path);
        assert!(parts.len() >= 2, "the holes stay: {}", parts.len());
        spans.push((s.bounds().x0, s.bounds().x1));
    }
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    for w in spans.windows(2) {
        assert!(w[1].0 > w[0].1 - 0.5, "{spans:?}");
    }
    let got = drawn_bounds(&app, &pieces);
    assert!(near_rect(got, drawn, 0.05), "{got:?} {drawn:?}");
    assert_eq!(app.engine.undo_label(), Some("Break Apart"));
    app.undo();
    let s = app.doc().find_shape(id).expect("text");
    assert!(matches!(s.kind, ShapeKind::Text { .. }));
    assert!(s
        .effects
        .iter()
        .any(|e| matches!(e, Effect::Envelope { .. })));
    assert!(app.doc().find_shape(pieces[2]).is_none() || pieces[2] == id);
}

/// The node count of `ids`' curves.
fn node_count(app: &App, ids: &[ShapeId]) -> usize {
    ids.iter()
        .filter_map(|id| app.doc().shape_on_page(*id))
        .map(|s| match &s.kind {
            ShapeKind::Path { path, .. } => tracedraw_core::nodes::nodes(path).len(),
            _ => 0,
        })
        .sum()
}

/// Convert to Curves and Break Apart of an enveloped text give smooth
/// curves with about as many nodes as the text's own outline, not the
/// warp's dense lines, still drawn where the text was (#647).
#[test]
fn baked_envelopes_are_fitted_with_few_curves() {
    let mut app = App::headless();
    let plain = text(&mut app, Point::new(20.0, 100.0), "ABC");
    app.select(vec![plain]);
    app.convert_to_curves();
    let own = node_count(&app, &[plain]);
    assert!(own > 10, "{own}");
    for break_apart in [false, true] {
        let id = text(&mut app, Point::new(20.0, 160.0), "ABC");
        app.select(vec![id]);
        app.apply_envelope_preset(crate::envelope_edit::Preset::Arch);
        let drawn = drawn_bounds(&app, &[id]);
        if break_apart {
            app.break_apart();
        } else {
            app.convert_to_curves();
        }
        let pieces = app.selection.clone();
        let n = node_count(&app, &pieces);
        assert!(
            n <= own * 3,
            "break apart {break_apart}: {n} nodes for {own} unwarped"
        );
        let curves = pieces
            .iter()
            .filter_map(|id| app.doc().shape_on_page(*id))
            .flat_map(|s| match &s.kind {
                ShapeKind::Path { path, .. } => path.elements().to_vec(),
                _ => Vec::new(),
            })
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::CurveTo(..)))
            .count();
        assert!(curves > 0, "curves, not only lines");
        let got = drawn_bounds(&app, &pieces);
        assert!(near_rect(got, drawn, 0.05), "{got:?} {drawn:?}");
        app.delete_selection();
    }
}

/// Break Apart of one enveloped letter makes it one warped curve with its
/// hole kept; it used to split it into filled parts (#647).
#[test]
fn break_apart_keeps_the_hole_of_a_single_warped_letter() {
    let mut app = App::headless();
    let id = text(&mut app, Point::new(20.0, 100.0), "O");
    app.select(vec![id]);
    app.apply_envelope_preset(crate::envelope_edit::Preset::Arch);
    app.break_apart();
    assert_eq!(app.selection, vec![id]);
    let s = app.doc().find_shape(id).expect("piece");
    let ShapeKind::Path { path, .. } = &s.kind else {
        panic!("a curve: {:?}", s.kind);
    };
    assert!(s.effects.is_empty());
    assert!(tracedraw_core::effects::subpaths(path).len() >= 2);
}

/// Several warped texts broken apart at once keep the stacking order:
/// each text's letters stay right above it, under what was above it.
#[test]
fn breaking_several_warped_texts_keeps_the_stacking_order() {
    let mut app = App::headless();
    let low = text(&mut app, Point::new(20.0, 100.0), "ABC");
    let between = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 200.0, 200.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    let high = text(&mut app, Point::new(20.0, 160.0), "DE");
    for id in [low, high] {
        app.select(vec![id]);
        app.apply_envelope_preset(crate::envelope_edit::Preset::Arch);
    }
    app.select(vec![low, high]);
    app.break_apart();
    assert_eq!(app.selection.len(), 5, "{:?}", app.selection);
    let order: Vec<ShapeId> = app
        .doc()
        .pages
        .iter()
        .flat_map(|p| &p.layers)
        .flat_map(|l| &l.shapes)
        .map(|s| s.id)
        .collect();
    let at = |id: ShapeId| order.iter().position(|o| *o == id).expect("placed");
    let rect_at = at(between);
    for id in &app.selection {
        let y = app
            .doc()
            .find_shape(*id)
            .expect("piece")
            .bounds()
            .center()
            .y;
        if y < 130.0 {
            assert!(
                at(*id) < rect_at,
                "a letter of the low text above the rectangle"
            );
        } else {
            assert!(
                at(*id) > rect_at,
                "a letter of the high text under the rectangle"
            );
        }
    }
}

/// How long Convert to Curves takes on long texts, and the drawing's
/// look kept (#800): `cargo test --release -p tracedraw -- --ignored
/// frame_times_converting_long_text --nocapture`. Uses a Chinese font
/// when one is installed.
#[test]
#[ignore]
fn frame_times_converting_long_text() {
    for (font, line) in [
        ("WenQuanYi Zen Hei", "镂空问题汉字转曲线测试中文字体"),
        (
            "DejaVu Sans",
            "The quick brown fox jumps over the lazy dog. ",
        ),
    ] {
        let mut app = App::headless();
        app.text_font = font.into();
        let body: String = (0..40).map(|_| format!("{}\n", line.repeat(4))).collect();
        let id = text(&mut app, Point::new(20.0, 100.0), &body);
        app.select(vec![id]);
        let t = std::time::Instant::now();
        app.convert_to_curves();
        let s = app.doc().shape_on_page(id).expect("shape");
        let n = s.local_path().elements().len();
        eprintln!(
            "{font}: {} characters to curves in {:?}, {n} path elements",
            body.chars().count(),
            t.elapsed()
        );
    }
}

/// One frame of the whole window with `events`.
fn frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
    out.textures_delta.clear();
}

fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

fn click(ctx: &egui::Context, app: &mut App, at: egui::Pos2) {
    frame(ctx, app, vec![egui::Event::PointerMoved(at)]);
    frame(ctx, app, vec![press(at, true)]);
    frame(ctx, app, vec![press(at, false)]);
    // Past the double-click time, so the next click is one of its own.
    for _ in 0..20 {
        frame(ctx, app, Vec::new());
    }
}

fn drag(ctx: &egui::Context, app: &mut App, from: egui::Pos2, to: egui::Pos2) {
    frame(ctx, app, vec![egui::Event::PointerMoved(from)]);
    frame(ctx, app, vec![press(from, true)]);
    for i in 1..=6 {
        let p = from + (to - from) * (i as f32 / 6.0);
        frame(ctx, app, vec![egui::Event::PointerMoved(p)]);
    }
    frame(ctx, app, vec![press(to, false)]);
    frame(ctx, app, Vec::new());
}

fn blend_of(app: &App, id: ShapeId) -> Option<Effect> {
    app.doc()
        .find_shape(id)?
        .effects
        .iter()
        .find(|e| matches!(e, Effect::Blend { .. }))
        .cloned()
}

/// The Blend tool on a blend of two texts, in the window: a click on
/// either text picks the blend and shows its controls on the canvas;
/// dragging the object slider toward the start bunches the steps there,
/// the colour slider stays, the end stays the end, and each drag is one
/// undo step (#965).
#[test]
fn the_blend_tool_picks_a_text_blend_and_its_sliders_tune_it() {
    use crate::blend_tool::{screen_normal, BlendHandle, SLIDER_OFFSET_PX};
    let ctx = crate::theme::ui_context();
    let mut app = App::headless();
    app.show_welcome = false;
    let a = text(&mut app, Point::new(30.0, 150.0), "A");
    let b = text(&mut app, Point::new(150.0, 150.0), "A");
    app.run(tracedraw_core::Command::SetFill {
        shapes: vec![b],
        fill: Fill::Solid(Color::rgb8(0, 160, 80)),
    });
    app.selection = vec![a, b];
    app.blend_selection(20, 0.0, 0.0, 0.0);
    assert!(blend_of(&app, a).is_some());
    app.set_tool(crate::tools::Tool::Blend);
    app.select(Vec::new());
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new());
    }
    app.zoom_to_page();
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new());
    }
    assert_eq!(app.blend_controls(), None, "nothing selected");

    // A click on the end text picks the blend.
    let on_b = app.view.to_screen(center(&app, b));
    click(&ctx, &mut app, on_b);
    assert_eq!(app.selection, vec![b]);
    let c = app.blend_controls().expect("the blend's controls");
    assert_eq!(c.shape, a);
    // A click on empty space lets go; one on the start text picks it.
    let empty = app.view.to_screen(Point::new(100.0, 40.0));
    click(&ctx, &mut app, empty);
    assert_eq!(app.blend_controls(), None);
    let on_a = app.view.to_screen(center(&app, a));
    click(&ctx, &mut app, on_a);
    assert_eq!(app.selection, vec![a]);
    let c = app.blend_controls().expect("the blend's controls");
    assert!((c.start - center(&app, a)).hypot() < 1e-6);
    assert!((c.end - center(&app, b)).hypot() < 1e-6);

    // The object slider, above the line, dragged to 30 % of the way.
    let (sa, sb) = (app.view.to_screen(c.start), app.view.to_screen(c.end));
    let n = screen_normal(sa, sb) * SLIDER_OFFSET_PX;
    let grab = app.view.to_screen(c.slider_point(BlendHandle::Objects)) + n;
    assert_eq!(
        app.blend_handle_at(app.view.to_page(grab)),
        Some(BlendHandle::Objects)
    );
    let undo_before = app.engine.history_labels().0.len();
    drag(&ctx, &mut app, grab, sa + (sb - sa) * 0.3 + n);
    let Some(Effect::Blend {
        accel_objects,
        accel_colors,
        end_id,
        steps,
        ..
    }) = blend_of(&app, a)
    else {
        panic!("still a blend");
    };
    assert!(accel_objects > 0.3, "{accel_objects}");
    assert_eq!(accel_colors, 0.0);
    assert_eq!((end_id, steps), (Some(b), 20));
    assert_eq!(app.selection, vec![a], "the blend stays selected");
    assert_eq!(app.engine.history_labels().0.len(), undo_before + 1);
    // The bar shows the new value.
    assert_eq!(app.blend_accel_objects, accel_objects);
    // The steps follow: the middle one now nearer the start.
    let s = app.doc().find_shape(a).expect("a").clone();
    let ev = tracedraw_core::live::evaluate(&s);
    let mut xs: Vec<f64> = ev
        .below
        .iter()
        .chain(&ev.above)
        .map(|s| s.page_path().bounding_box().center().x)
        .collect();
    xs.sort_by(f64::total_cmp);
    let middle = xs[xs.len() / 2];
    assert!(middle < (c.start.x + c.end.x) / 2.0 - 5.0, "{middle}");

    // The colour slider, below the line, toward the end.
    let c = app.blend_controls().expect("controls");
    let grab = app.view.to_screen(c.slider_point(BlendHandle::Colors)) - n;
    assert_eq!(
        app.blend_handle_at(app.view.to_page(grab)),
        Some(BlendHandle::Colors)
    );
    drag(&ctx, &mut app, grab, sa + (sb - sa) * 0.7 - n);
    let Some(Effect::Blend {
        accel_objects: o2,
        accel_colors,
        ..
    }) = blend_of(&app, a)
    else {
        panic!("still a blend");
    };
    assert!(accel_colors < -0.3, "{accel_colors}");
    assert_eq!(o2, accel_objects, "the object slider stays");
    assert_eq!(app.engine.history_labels().0.len(), undo_before + 2);
}

/// The squares at the ends of a text blend's line, with the Blend tool
/// in the window (#965, the reporter's second request): dragging the end
/// square to empty space moves the end text, so the blend changes its
/// direction, in one undo step; dropped on another text it makes that
/// text the end; the start square dropped on a text moves the blend to
/// it, the old start staying a plain text.
#[test]
fn the_blend_tool_s_end_squares_move_and_pick_the_end_objects() {
    use crate::blend_tool::BlendHandle;
    let ctx = crate::theme::ui_context();
    let mut app = App::headless();
    app.show_welcome = false;
    let a = text(&mut app, Point::new(30.0, 150.0), "A");
    let b = text(&mut app, Point::new(150.0, 150.0), "A");
    let c = text(&mut app, Point::new(150.0, 60.0), "A");
    let d = text(&mut app, Point::new(30.0, 60.0), "A");
    app.selection = vec![a, b];
    app.blend_selection(20, 0.0, 0.0, 0.0);
    app.set_tool(crate::tools::Tool::Blend);
    app.select(Vec::new());
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new());
    }
    app.zoom_to_page();
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new());
    }
    // A click on the text picks the blend and its controls.
    let on_a = app.view.to_screen(center(&app, a));
    click(&ctx, &mut app, on_a);
    let ctl = app.blend_controls().expect("the blend's controls");
    assert_eq!(ctl.shape, a);
    let end_before = center(&app, b);
    let grab = app.view.to_screen(ctl.end);
    assert_eq!(app.blend_handle_at(ctl.end), Some(BlendHandle::End));
    assert_eq!(app.blend_handle_at(ctl.start), Some(BlendHandle::Start));

    // The end square dragged 40 mm to the right, onto empty page: the end
    // text moves with it and the line follows.
    let undo_before = app.engine.history_labels().0.len();
    let to = app.view.to_screen(ctl.end + Vec2::new(40.0, 0.0));
    drag(&ctx, &mut app, grab, to);
    let moved = center(&app, b);
    assert!((moved.x - end_before.x - 40.0).abs() < 1.0, "{moved:?}");
    assert!((moved.y - end_before.y).abs() < 1.0, "{moved:?}");
    let ctl = app.blend_controls().expect("controls");
    assert!((ctl.end - moved).hypot() < 1e-6);
    assert_eq!(app.engine.history_labels().0.len(), undo_before + 1);
    assert!(matches!(blend_of(&app, a), Some(Effect::Blend { end_id: Some(id), .. }) if id == b));

    // The end square dropped on the third text: that text becomes the
    // end, the second text stays where it was.
    let grab = app.view.to_screen(ctl.end);
    let on_c = app.view.to_screen(center(&app, c));
    drag(&ctx, &mut app, grab, on_c);
    let Some(Effect::Blend { end_id, steps, .. }) = blend_of(&app, a) else {
        panic!("still a blend");
    };
    assert_eq!((end_id, steps), (Some(c), 20));
    assert!(
        (center(&app, b) - moved).hypot() < 1e-6,
        "the old end stays"
    );
    assert!((center(&app, c) - Point::new(150.0, 60.0)).hypot() > 0.0);
    let ctl = app.blend_controls().expect("controls");
    assert!((ctl.end - center(&app, c)).hypot() < 1e-6);
    assert_eq!(app.selection, vec![a]);

    // The start square dropped on the fourth text: the blend moves to it.
    let grab = app.view.to_screen(ctl.start);
    let on_d = app.view.to_screen(center(&app, d));
    drag(&ctx, &mut app, grab, on_d);
    assert!(blend_of(&app, a).is_none(), "the old start is plain text");
    let Some(Effect::Blend { end_id, .. }) = blend_of(&app, d) else {
        panic!("the blend is on the fourth text");
    };
    assert_eq!(end_id, Some(c));
    assert_eq!(app.selection, vec![d]);
    let ctl = app.blend_controls().expect("controls");
    assert_eq!(ctl.shape, d);
    assert!((ctl.start - center(&app, d)).hypot() < 1e-6);
    // Undo takes the blend back to the first text.
    app.undo();
    assert!(blend_of(&app, a).is_some());
    assert!(blend_of(&app, d).is_none());
}

/// Copy Envelope Properties from a shape with no envelope fits the text
/// into that shape (#993): text taking a circle's outline fills the
/// ellipse inside its own box and leaves the box's corners empty.
#[test]
fn text_takes_the_shape_of_an_object_as_its_envelope() {
    let mut app = App::headless();
    let id = text(&mut app, Point::new(20.0, 100.0), "HHHHHH");
    let circle = app
        .new_shape(ShapeKind::Ellipse {
            rect: Rect::new(150.0, 50.0, 190.0, 90.0),
            arc: None,
        })
        .expect("circle");
    app.select(vec![id]);
    let before = tracedraw_core::live::evaluate(app.doc().find_shape(id).expect("text"))
        .main
        .bounds();
    app.start_copy_envelope();
    app.copy_envelope_from(circle);
    assert!(app.envelope_of(id).is_some(), "{}", app.status);
    let s = app.doc().find_shape(id).expect("text").clone();
    let drawn = s.transform * tracedraw_core::live::evaluate(&s).main.local_path();
    let b = drawn.bounding_box();
    // The text keeps its place and size.
    assert!(
        (b.center() - before.center()).hypot() < 0.5,
        "{b:?} {before:?}"
    );
    assert!((b.width() - before.width()).abs() < 0.5, "{b:?} {before:?}");
    // Every point drawn lies in the ellipse of the box.
    let (c, rx, ry) = (b.center(), b.width() / 2.0, b.height() / 2.0);
    let mut worst = 0.0f64;
    tracedraw_core::geometry::flatten(&drawn, 0.01, &mut |el| {
        if let tracedraw_core::geometry::PathEl::LineTo(p)
        | tracedraw_core::geometry::PathEl::MoveTo(p) = el
        {
            let d = ((p.x - c.x) / rx).powi(2) + ((p.y - c.y) / ry).powi(2);
            worst = worst.max(d);
        }
    });
    assert!(worst < 1.02, "{worst}");
}
