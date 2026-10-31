//! What the on-screen keyboard of Android types (#1309, #1271).
//!
//! The activity is a `NativeActivity`, which offers input methods no text
//! field: a keyboard could only send the key presses of Latin letters,
//! so Chinese, Japanese, Korean, Tamil and every script typed by
//! composing never reached the editor. The activity now keeps an
//! invisible view the keyboard types into (`TextInputView.java`); what
//! it composes, commits and deletes comes here through the activity's
//! native methods (`android.rs`), is queued, and joins the next frame's
//! input as an input method's events, the way the windowing layer hands
//! them over on the desktop. The keyboard is shown while something takes
//! typed text: a text or table cell being edited, a field with the
//! focus ([`wanted`]).
//!
//! Off Android only the queue's frame side runs (nothing is queued).

#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use std::sync::Mutex;

/// What the keyboard did, in the order it did it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Typed {
    /// Text being composed (an empty one ends the composition).
    Compose(String),
    /// Text committed: typed at the caret.
    Commit(String),
    /// Characters deleted before and after the caret.
    Delete { before: usize, after: usize },
}

/// A key pressed and let go.
fn key_events(key: egui::Key) -> [egui::Event; 2] {
    let ev = |pressed| egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    [ev(true), ev(false)]
}

/// The frame events of what the keyboard did: compositions and commits as
/// an input method's events, line ends committed as Enter, deletions as
/// Backspace and Delete presses (what a text being edited and a field
/// both follow).
pub fn events_of(typed: &Typed) -> Vec<egui::Event> {
    use egui::{Event, ImeEvent, Key};
    match typed {
        Typed::Compose(text) => vec![Event::Ime(ImeEvent::Preedit {
            text: text.clone(),
            active_range_chars: None,
        })],
        Typed::Commit(text) => {
            let mut out = Vec::new();
            let text = text.replace("\r\n", "\n").replace('\r', "\n");
            for (i, part) in text.split('\n').enumerate() {
                if i > 0 {
                    out.extend(key_events(Key::Enter));
                }
                // The first part ends the composition even when empty.
                if i == 0 || !part.is_empty() {
                    out.push(Event::Ime(ImeEvent::Commit(part.to_string())));
                }
            }
            out
        }
        Typed::Delete { before, after } => {
            let mut out = Vec::new();
            for _ in 0..*before {
                out.extend(key_events(Key::Backspace));
            }
            for _ in 0..*after {
                out.extend(key_events(Key::Delete));
            }
            out
        }
    }
}

/// Events waiting for the next frame.
static QUEUE: Mutex<Vec<egui::Event>> = Mutex::new(Vec::new());

/// Queue what the keyboard did for the next frame.
pub fn push(typed: Typed) {
    if let Ok(mut q) = QUEUE.lock() {
        q.extend(events_of(&typed));
    }
}

/// Add what the keyboard did since the last frame to its input.
pub fn take_into(raw: &mut egui::RawInput) {
    let events = match QUEUE.lock() {
        Ok(mut q) if !q.is_empty() => std::mem::take(&mut *q),
        _ => return,
    };
    raw.events.extend(events);
}

/// Whether the frame just drawn takes typed text: the canvas while a
/// text is edited and focused fields ask for the input method.
pub fn wanted(output: &egui::FullOutput) -> bool {
    output.platform_output.ime.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, ImeEvent, Key};

    fn keys(events: &[Event]) -> Vec<(Key, bool)> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Key { key, pressed, .. } => Some((*key, *pressed)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_keyboard_s_text_becomes_input_method_events() {
        assert_eq!(
            events_of(&Typed::Compose("ni".into())),
            vec![Event::Ime(ImeEvent::Preedit {
                text: "ni".into(),
                active_range_chars: None,
            })]
        );
        assert_eq!(
            events_of(&Typed::Commit("你好".into())),
            vec![Event::Ime(ImeEvent::Commit("你好".into()))]
        );
        // A line end committed is Enter.
        let e = events_of(&Typed::Commit("a\nb".into()));
        assert_eq!(e.first(), Some(&Event::Ime(ImeEvent::Commit("a".into()))));
        assert_eq!(keys(&e), vec![(Key::Enter, true), (Key::Enter, false)]);
        assert_eq!(e.last(), Some(&Event::Ime(ImeEvent::Commit("b".into()))));
        let e = events_of(&Typed::Delete {
            before: 2,
            after: 1,
        });
        assert_eq!(
            keys(&e),
            vec![
                (Key::Backspace, true),
                (Key::Backspace, false),
                (Key::Backspace, true),
                (Key::Backspace, false),
                (Key::Delete, true),
                (Key::Delete, false)
            ]
        );
    }

    /// The keyboard is wanted while a text is edited on the canvas, not
    /// while the Pick tool has the selection.
    #[test]
    fn the_keyboard_is_wanted_while_text_is_edited() {
        let ctx = crate::theme::ui_context();
        let mut app = crate::app::App::headless();
        app.show_welcome = false;
        let frame = |app: &mut crate::app::App| {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(raw, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
            wanted(&out)
        };
        for _ in 0..3 {
            frame(&mut app);
        }
        app.zoom_to_page();
        assert!(!frame(&mut app));
        app.set_tool(crate::tools::Tool::Text);
        app.start_text(tracedraw_core::geometry::Point::new(30.0, 50.0), None);
        assert!(frame(&mut app));
        app.finish_text();
        app.set_tool(crate::tools::Tool::Pick);
        assert!(!frame(&mut app));
    }

    /// Chinese composed and committed on the keyboard, a Tamil letter
    /// changed by deleting it and committing its new form: the text being
    /// edited gets them as typed.
    #[test]
    fn a_text_being_edited_takes_what_the_keyboard_types() {
        let mut app = crate::app::App::headless();
        app.settings.autocorrect.enabled = false;
        app.start_text(tracedraw_core::geometry::Point::new(0.0, 0.0), None);
        let ctx = crate::theme::ui_context();
        let frame = |app: &mut crate::app::App, typed: Vec<Typed>| {
            for t in typed {
                push(t);
            }
            let mut raw = egui::RawInput::default();
            take_into(&mut raw);
            let mut out = ctx.run_ui(raw, |ui| app.text_keyboard(ui.ctx()));
            out.textures_delta.clear();
        };
        frame(&mut app, vec![Typed::Compose("ni".into())]);
        assert_eq!(app.ime_preedit, "ni");
        assert_eq!(app.edit_text(), "");
        frame(&mut app, vec![Typed::Commit("你".into())]);
        assert_eq!(app.edit_text(), "你");
        assert!(app.ime_preedit.is_empty());
        frame(
            &mut app,
            vec![
                Typed::Commit("க\u{0BCD}".into()),
                Typed::Delete {
                    before: 1,
                    after: 0,
                },
                Typed::Commit("\u{0BBF}".into()),
            ],
        );
        assert_eq!(app.edit_text(), "你கி");
    }
}
