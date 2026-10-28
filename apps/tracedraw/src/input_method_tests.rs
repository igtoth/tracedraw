//! Typing through input methods (#1151): what the system's keyboard
//! programs send (composition events, plain characters) reaches the
//! artistic and paragraph text being edited.

use crate::app::App;
use crate::tools::Tool;
use egui::{Event, Pos2};
use tracedraw_core::geometry::{Point, Size};

fn frame(ctx: &egui::Context, app: &mut App, events: Vec<Event>) -> egui::FullOutput {
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
    out
}

fn setup() -> (egui::Context, App) {
    let ctx = crate::theme::ui_context();
    let mut app = App::headless();
    app.show_welcome = false;
    app.settings.autocorrect.enabled = false;
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new());
    }
    app.zoom_to_page();
    frame(&ctx, &mut app, Vec::new());
    (ctx, app)
}

/// Starts an artistic text (or a paragraph text in a frame) with the
/// Text tool, as a click (or a drag) does.
fn start(ctx: &egui::Context, app: &mut App, paragraph: bool) {
    app.set_tool(Tool::Text);
    let frame_size = paragraph.then(|| Size::new(80.0, 40.0));
    app.start_text(Point::new(30.0, 50.0), frame_size);
    frame(ctx, app, Vec::new());
}

const TAMIL: &str = "தமிழ் கொடு";

/// The keyboard messages of an input method that posts each character
/// with no key press before it (and one key it took for itself).
fn posted_messages(text: &str) -> Vec<(u32, usize)> {
    use crate::system_chars::*;
    let mut msgs = Vec::new();
    for (i, c) in text.chars().enumerate() {
        if i == 3 {
            msgs.push((WM_KEYDOWN, VK_PROCESSKEY as usize));
        } else {
            msgs.push((WM_KEYUP, 0x41));
        }
        msgs.push((WM_CHAR, c as usize));
    }
    msgs
}

/// A frame whose input carries what the message hook kept, as the
/// window's raw input hook adds it.
fn frame_after_messages(ctx: &egui::Context, app: &mut App, msgs: &[(u32, usize)]) {
    let mut tracker = crate::system_chars::CharTracker::default();
    for (m, w) in msgs {
        tracker.message(*m, *w);
    }
    crate::system_chars::hand_over(&mut tracker);
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };
    crate::system_chars::take_into(&mut input);
    let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
    out.textures_delta.clear();
}

#[test]
fn characters_posted_by_an_input_method_type_into_artistic_and_paragraph_text() {
    for paragraph in [false, true] {
        let (ctx, mut app) = setup();
        start(&ctx, &mut app, paragraph);
        frame_after_messages(&ctx, &mut app, &posted_messages(TAMIL));
        assert_eq!(app.edit_text(), TAMIL, "paragraph {paragraph}");
        // Then typed one letter per frame.
        frame_after_messages(&ctx, &mut app, &posted_messages(" கை"));
        assert_eq!(app.edit_text(), format!("{TAMIL} கை"));
    }
}

#[test]
fn an_input_method_types_tamil_by_text_or_by_composition() {
    for paragraph in [false, true] {
        let (ctx, mut app) = setup();
        start(&ctx, &mut app, paragraph);
        // The canvas asks for the input method while the text is edited.
        let out = frame(&ctx, &mut app, Vec::new());
        assert!(out.platform_output.ime.is_some(), "paragraph {paragraph}");
        // Letters as plain text, one character at a time.
        for c in "தமிழ்".chars() {
            frame(&ctx, &mut app, vec![Event::Text(c.to_string())]);
        }
        // A letter composed, then committed.
        frame(
            &ctx,
            &mut app,
            vec![Event::Ime(egui::ImeEvent::Preedit {
                text: "கொ".into(),
                active_range_chars: None,
            })],
        );
        assert_eq!(app.edit_text(), "தமிழ்");
        frame(
            &ctx,
            &mut app,
            vec![Event::Ime(egui::ImeEvent::Commit(" கொ".into()))],
        );
        assert_eq!(app.edit_text(), "தமிழ் கொ");
        // The letters are drawn (from a face that has them when the
        // text's own font lacks them).
        use tracedraw_core::geometry::Shape as _;
        tracedraw_text::install();
        let id = app.text_edit.as_ref().expect("editing").shape;
        app.finish_text();
        let has_tamil = tracedraw_text::fonts()
            .families()
            .iter()
            .any(|f| f.contains("Tamil") || f.contains("FreeSerif") || f.contains("Lohit"));
        if has_tamil {
            let s = app.doc().find_shape(id).expect("text").clone();
            let r = s.page_path().bounding_box();
            assert!(r.width() > 5.0 && r.height() > 2.0, "{r:?}");
        }
    }
}
