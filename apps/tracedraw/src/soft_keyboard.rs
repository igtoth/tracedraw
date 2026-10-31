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

/// How many frames in a row must take no typed text before the keyboard
/// hides: a frame or two without the canvas's caret (a tap's own frame,
/// the window resized for the keyboard) must not hide it and show it
/// again, which ends what the input method was composing (#1489).
pub const HIDE_AFTER_FRAMES: u8 = 3;

/// What the keyboard should do after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Show it and connect it to the editor afresh.
    Show,
    /// Show it again, still connected: the user hid it with the system's
    /// key and tapped into the text or field again (#1496).
    ShowAgain,
    Hide,
}

/// Follows the frames: shows the keyboard as soon as a frame takes typed
/// text, again when a tap lands while it does, and hides it once
/// [`HIDE_AFTER_FRAMES`] frames in a row took none.
#[derive(Debug, Default)]
pub struct Follower {
    shown: bool,
    unwanted: u8,
}

impl Follower {
    /// What to ask of the keyboard after a frame that took typed text
    /// (`want`) and had a pointer press (`tapped`), or not.
    pub fn step(&mut self, want: bool, tapped: bool) -> Option<Ask> {
        if want {
            self.unwanted = 0;
            if !self.shown {
                self.shown = true;
                return Some(Ask::Show);
            }
            return tapped.then_some(Ask::ShowAgain);
        }
        if !self.shown {
            return None;
        }
        self.unwanted = self.unwanted.saturating_add(1);
        if self.unwanted < HIDE_AFTER_FRAMES {
            return None;
        }
        self.shown = false;
        self.unwanted = 0;
        Some(Ask::Hide)
    }

    /// Whether frames that take no text are being counted towards a
    /// hide: more frames must come for it.
    pub fn hiding(&self) -> bool {
        self.shown && self.unwanted > 0
    }

    /// The last ask could not reach the activity: the next frame asks
    /// again.
    pub fn failed(&mut self, ask: Ask) {
        match ask {
            Ask::Show => self.shown = false,
            Ask::Hide => self.shown = true,
            Ask::ShowAgain => {}
        }
    }
}

/// The emulator check's typing test (`scripts/android/emulator_check.sh`
/// starts the app with `--es tracedraw.check typing`): the app opens a
/// drawing with the Text tool, the check taps the canvas and types
/// through a test input method and as key presses, and the app writes
/// what the text being edited reads to the system log after each change.
/// Nothing of this runs in a normal start.
pub mod check {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    static ON: AtomicBool = AtomicBool::new(false);

    #[derive(Default)]
    struct State {
        /// The drawing, the tool and the screen are ready for the tap.
        ready: bool,
        /// A text was started (by the check's tap).
        started: bool,
        /// What the log said last: the text and its composition.
        said: Option<(String, String)>,
    }

    static STATE: Mutex<Option<State>> = Mutex::new(None);

    /// Start the check (the app was started for it).
    pub fn start() {
        ON.store(true, Ordering::Release);
        log::info!("typing check: on");
    }

    /// After each frame: until the check's tap starts a text, a drawing
    /// is open with the Text tool and nothing in front of it; then each
    /// change of the text being edited is logged.
    pub fn frame(app: &mut crate::app::App) {
        if !ON.load(Ordering::Acquire) {
            return;
        }
        let Ok(mut guard) = STATE.lock() else {
            return;
        };
        let state = guard.get_or_insert_with(State::default);
        if let Some(line) = step(app, state) {
            log::info!("{line}");
        }
    }

    /// One frame of the check; the line to log, if any.
    fn step(app: &mut crate::app::App, state: &mut State) -> Option<String> {
        if !state.started {
            if app.text_edit.is_some() {
                state.started = true;
            } else {
                if !app.has_document() {
                    app.new_document();
                }
                app.show_welcome = false;
                app.settings.update_check_asked = true;
                app.dialog = crate::ui::dialogs::Dialog::None;
                if app.tool != crate::tools::Tool::Text {
                    app.set_tool(crate::tools::Tool::Text);
                }
                if !state.ready {
                    state.ready = true;
                    return Some("typing check: ready, the Text tool is on".into());
                }
                return None;
            }
        }
        let now = (app.edit_text(), app.ime_preedit.clone());
        if state.said.as_ref() == Some(&now) {
            return None;
        }
        let line = if app.text_edit.is_some() {
            format!("typing check: text {:?}, composing {:?}", now.0, now.1)
        } else {
            "typing check: no text is being edited".to_string()
        };
        state.said = Some(now);
        Some(line)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn the_check_readies_the_text_tool_then_logs_the_text() {
            let mut app = crate::app::App::headless();
            app.settings.autocorrect.enabled = false;
            app.show_welcome = true;
            app.dialog = crate::ui::dialogs::Dialog::UpdatePolicy;
            let mut state = State::default();
            assert_eq!(
                step(&mut app, &mut state).as_deref(),
                Some("typing check: ready, the Text tool is on")
            );
            assert_eq!(app.tool, crate::tools::Tool::Text);
            assert!(!app.show_welcome);
            assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::None));
            assert_eq!(step(&mut app, &mut state), None);
            app.start_text(tracedraw_core::geometry::Point::new(10.0, 10.0), None);
            assert_eq!(
                step(&mut app, &mut state).as_deref(),
                Some("typing check: text \"\", composing \"\"")
            );
            assert_eq!(step(&mut app, &mut state), None);
            app.ime_preedit = "ni".into();
            assert_eq!(
                step(&mut app, &mut state).as_deref(),
                Some("typing check: text \"\", composing \"ni\"")
            );
            app.ime_preedit.clear();
            app.text_insert("你好");
            assert_eq!(
                step(&mut app, &mut state).as_deref(),
                Some("typing check: text \"你好\", composing \"\"")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, ImeEvent, Key};

    /// The tests that queue what the keyboard typed take turns: the
    /// queue is one for the process.
    static QUEUE_TURN: Mutex<()> = Mutex::new(());

    fn queue_turn() -> std::sync::MutexGuard<'static, ()> {
        QUEUE_TURN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

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

    /// The keyboard shows with the first frame that takes text, comes
    /// back on a tap while it does (the system's key may have hidden it),
    /// and hides only after a few frames in a row take none: a lone frame
    /// without the caret does not hide it and show it again, which would
    /// end what the input method composes (#1489, #1496).
    #[test]
    fn the_keyboard_follows_the_frames_without_flicker() {
        let mut f = Follower::default();
        assert_eq!(f.step(false, false), None);
        assert_eq!(f.step(false, true), None);
        assert_eq!(f.step(true, true), Some(Ask::Show));
        assert_eq!(f.step(true, false), None);
        assert_eq!(f.step(true, true), Some(Ask::ShowAgain));
        // One or two frames without: still shown.
        assert_eq!(f.step(false, false), None);
        assert_eq!(f.step(true, false), None);
        assert!(!f.hiding());
        for _ in 1..HIDE_AFTER_FRAMES {
            assert_eq!(f.step(false, false), None);
            assert!(f.hiding(), "the frames that let it hide are asked for");
        }
        assert_eq!(f.step(false, false), Some(Ask::Hide));
        assert!(!f.hiding());
        assert_eq!(f.step(false, false), None);
        // An ask that did not reach the activity is made again.
        assert_eq!(f.step(true, false), Some(Ask::Show));
        f.failed(Ask::Show);
        assert_eq!(f.step(true, false), Some(Ask::Show));
    }

    /// The Android app's first start (the Touch workspace on a phone's
    /// screen): a finger's tap with the Text tool starts a text and the
    /// frames ask for the keyboard from then on; what it composes and
    /// commits goes into that text (#1496, #1489).
    #[test]
    fn a_tap_with_the_text_tool_starts_a_text_that_takes_the_keyboard() {
        use egui::{Event, PointerButton, Pos2, TouchPhase};
        let _turn = queue_turn();
        let ctx = crate::theme::ui_context();
        let mut app = crate::app::App::headless();
        app.settings.autocorrect.enabled = false;
        app.show_welcome = false;
        app.set_workspace(crate::app::Workspace::Touch);
        let follower = std::cell::RefCell::new(Follower::default());
        let asks = std::cell::RefCell::new(Vec::new());
        let frame = |app: &mut crate::app::App, events: Vec<Event>| {
            let mut raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(412.0, 860.0),
                )),
                events,
                ..Default::default()
            };
            // As the window's raw input hook does (lib.rs).
            take_into(&mut raw);
            crate::pen::filter_raw_input(&ctx, &mut raw);
            crate::interaction::release_chorded_left(&ctx, &mut raw);
            let mut out = ctx.run_ui(raw, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
            let tapped = ctx.input(|i| i.pointer.any_pressed());
            if let Some(ask) = follower.borrow_mut().step(wanted(&out), tapped) {
                asks.borrow_mut().push(ask);
            }
            wanted(&out)
        };
        for _ in 0..3 {
            frame(&mut app, Vec::new());
        }
        app.zoom_to_page();
        app.set_tool(crate::tools::Tool::Text);
        for _ in 0..3 {
            assert!(!frame(&mut app, Vec::new()));
        }
        let at = app.canvas_rect.center();
        let touch = |phase| Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(0),
            phase,
            pos: at,
            force: None,
        };
        let button = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        // The windowing layer turns the finger into the pointer as well,
        // and lets the pointer go with the finger: the release comes with
        // PointerGone, so the frame of the click has no hover position.
        frame(
            &mut app,
            vec![
                touch(TouchPhase::Start),
                Event::PointerMoved(at),
                button(true),
            ],
        );
        frame(
            &mut app,
            vec![touch(TouchPhase::End), button(false), Event::PointerGone],
        );
        assert!(app.text_edit.is_some(), "the tap started no text");
        let mut wanted_frames = 0;
        for _ in 0..5 {
            wanted_frames += frame(&mut app, Vec::new()) as usize;
        }
        assert_eq!(wanted_frames, 5, "every frame asks for the keyboard");
        assert_eq!(asks.borrow().first(), Some(&Ask::Show));
        assert!(!asks.borrow().contains(&Ask::Hide));
        push(Typed::Compose("ni".into()));
        frame(&mut app, Vec::new());
        assert_eq!(app.ime_preedit, "ni");
        push(Typed::Commit("你好".into()));
        frame(&mut app, Vec::new());
        assert_eq!(app.edit_text(), "你好");
        assert!(app.ime_preedit.is_empty());
        frame(&mut app, vec![Event::Text("abc".into())]);
        assert_eq!(app.edit_text(), "你好abc");
        assert!(!asks.borrow().contains(&Ask::Hide));
    }

    /// Chinese composed and committed on the keyboard, a Tamil letter
    /// changed by deleting it and committing its new form: the text being
    /// edited gets them as typed.
    #[test]
    fn a_text_being_edited_takes_what_the_keyboard_types() {
        let _turn = queue_turn();
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
