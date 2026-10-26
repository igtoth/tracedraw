//! Effects on text objects: blends between texts (and a text and a
//! shape), extrusions of text, and Break Apart of artistic text.

use crate::app::App;
use tracedraw_core::geometry::Shape as _;
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
