//! Characters that keyboard programs send on Windows and the windowing
//! layer loses (#1151).
//!
//! Input methods for Indic scripts (Tamil, Hindi, Bengali and others)
//! often type by sending the window plain character messages: some post
//! `WM_CHAR` with no key press before it, some answer a key they took
//! (reported as the "processed by an input method" key) with the
//! characters of the letter. The windowing layer reads a `WM_CHAR` only
//! as the text of the key press just before it, and drops the text of a
//! processed key while the input method is allowed (which it is while a
//! text is edited), so nothing was typed.
//!
//! A message hook reads the keyboard messages as they come, in the order
//! the windowing layer reads them, and keeps the characters it will
//! lose. At the start of each frame they are added to the frame's input
//! as typed text ([`merge`]), less any of them that did arrive (so a
//! character is never typed twice).

// Off Windows only the frame side runs (nothing is ever kept).
#![cfg_attr(not(windows), allow(dead_code))]

use std::sync::Mutex;

pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_KEYUP: u32 = 0x0101;
pub const WM_CHAR: u32 = 0x0102;
pub const WM_DEADCHAR: u32 = 0x0103;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_SYSKEYUP: u32 = 0x0105;
pub const WM_SYSCHAR: u32 = 0x0106;
pub const WM_SYSDEADCHAR: u32 = 0x0107;
/// The key an input method took for itself.
pub const VK_PROCESSKEY: u16 = 0xE5;

/// What the hook has seen of the keyboard messages, and the characters
/// to add to the next frame.
#[derive(Default, Debug)]
pub struct CharTracker {
    /// The key press whose characters may follow (its virtual key).
    key: Option<u16>,
    /// The first half of a character outside the basic plane.
    high: Option<u16>,
    /// Ctrl and Alt held, as their key presses and releases tell: both
    /// at once are AltGr, whose characters the windowing layer drops as
    /// the side effects of a Ctrl shortcut (#1271).
    ctrl: bool,
    alt: bool,
    /// Characters the windowing layer loses, in the order typed.
    pub lost: String,
}

impl CharTracker {
    /// One message of the window's queue. Only the keyboard messages
    /// matter: the windowing layer looks only at those to tie a
    /// character to its key.
    pub fn message(&mut self, msg: u32, wparam: usize) {
        match msg {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let vk = (wparam & 0xFFFF) as u16;
                self.held(vk, true);
                self.key = Some(vk);
                self.high = None;
            }
            WM_KEYUP | WM_SYSKEYUP => {
                self.held((wparam & 0xFFFF) as u16, false);
                self.key = None;
                self.high = None;
            }
            WM_DEADCHAR | WM_SYSDEADCHAR => {
                self.key = None;
                self.high = None;
            }
            WM_CHAR => {
                // Lost: no key press before it, or the text of a key an
                // input method took. The composing input methods of
                // Chinese, Japanese and Korean hand their text over as
                // a commit the windowing layer does deliver, often a
                // frame later than the message: their scripts are not
                // taken from a processed key, or they would type twice.
                let processed = self.key == Some(VK_PROCESSKEY);
                // A character of AltGr (Ctrl and Alt held): Polish ą ę ł
                // ś ż, German @ € { } and many more are dropped too.
                let altgr = self.ctrl && self.alt;
                let lost = self.key.is_none() || processed || altgr;
                let unit = (wparam & 0xFFFF) as u16;
                let ch = match unit {
                    0xD800..=0xDBFF => {
                        self.high = Some(unit);
                        return;
                    }
                    0xDC00..=0xDFFF => {
                        let Some(high) = self.high.take() else {
                            return;
                        };
                        char::decode_utf16([high, unit]).next().and_then(Result::ok)
                    }
                    _ => {
                        self.high = None;
                        char::from_u32(wparam as u32)
                    }
                };
                if let Some(ch) =
                    ch.filter(|c| lost && typed_char(*c) && !(processed && composed_script(*c)))
                {
                    self.lost.push(ch);
                }
            }
            // Alt with a key (the menus' access keys) types nothing.
            WM_SYSCHAR => {}
            _ => {}
        }
    }
}

impl CharTracker {
    /// Follow Ctrl and Alt (either side) going down or up.
    fn held(&mut self, vk: u16, down: bool) {
        match vk {
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => self.ctrl = down,
            VK_MENU | VK_LMENU | VK_RMENU => self.alt = down,
            _ => {}
        }
    }
}

pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;
pub const VK_LCONTROL: u16 = 0xA2;
pub const VK_RCONTROL: u16 = 0xA3;
pub const VK_LMENU: u16 = 0xA4;
pub const VK_RMENU: u16 = 0xA5;

/// A character of the scripts typed with a composing input method
/// (Chinese, Japanese, Korean, and their full-width forms), which comes
/// through the input method's commit.
fn composed_script(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x11FF
        | 0x2E80..=0x2FFF
        | 0x3000..=0x30FF
        | 0x3130..=0x318F
        | 0x31A0..=0x9FFF
        | 0xA960..=0xA97F
        | 0xAC00..=0xD7FF
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFFEF
        | 0x20000..=0x3FFFF)
}

/// A character that types (as the windowing layer filters them): no
/// control characters, nothing of the private use areas.
fn typed_char(c: char) -> bool {
    let private = ('\u{e000}'..='\u{f8ff}').contains(&c)
        || ('\u{f0000}'..='\u{ffffd}').contains(&c)
        || ('\u{100000}'..='\u{10fffd}').contains(&c);
    !c.is_control() && !private
}

/// Add the `lost` characters to a frame's `events` as typed text, less
/// those the frame already has as text or as an input method's commit.
pub fn merge(lost: &str, events: &mut Vec<egui::Event>) {
    if lost.is_empty() {
        return;
    }
    let mut arrived: Vec<char> = events
        .iter()
        .filter_map(|e| match e {
            egui::Event::Text(t) => Some(t.as_str()),
            egui::Event::Ime(egui::ImeEvent::Commit(t)) => Some(t.as_str()),
            _ => None,
        })
        .flat_map(str::chars)
        .collect();
    let mut add = String::new();
    for c in lost.chars() {
        if let Some(i) = arrived.iter().position(|a| *a == c) {
            arrived.remove(i);
        } else {
            add.push(c);
        }
    }
    if !add.is_empty() {
        events.push(egui::Event::Text(add));
    }
}

/// The characters the hook kept for the next frame.
static LOST: Mutex<String> = Mutex::new(String::new());
/// The window's context, to ask for a frame when a character is kept
/// (nothing else wakes the window for it).
static CONTEXT: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();

/// The window's context, once it exists.
pub fn set_context(ctx: &egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
}

/// Add the characters kept since the last frame to its input.
pub fn take_into(raw: &mut egui::RawInput) {
    let lost = match LOST.lock() {
        Ok(mut l) => std::mem::take(&mut *l),
        Err(_) => return,
    };
    merge(&lost, &mut raw.events);
}

/// Keep what `tracker` found lost and ask for a frame to type it.
pub(crate) fn hand_over(tracker: &mut CharTracker) {
    if tracker.lost.is_empty() {
        return;
    }
    if let Ok(mut l) = LOST.lock() {
        l.push_str(&tracker.lost);
    }
    tracker.lost.clear();
    if let Some(ctx) = CONTEXT.get() {
        ctx.request_repaint();
    }
}

/// Read the window's messages before the windowing layer does.
#[cfg(windows)]
pub fn install(builder: &mut eframe::EventLoopBuilder<eframe::UserEvent>) {
    use winit::platform::windows::EventLoopBuilderExtWindows as _;
    let mut tracker = CharTracker::default();
    builder.with_msg_hook(move |msg| {
        let msg = msg as *const windows_sys::Win32::UI::WindowsAndMessaging::MSG;
        // SAFETY: the windowing layer hands the hook the message it just
        // took from the queue, valid for the call.
        if let Some(m) = unsafe { msg.as_ref() } {
            tracker.message(m.message, m.wParam);
            hand_over(&mut tracker);
        }
        // The message goes on to the windowing layer as before.
        false
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A composing input method's text (Chinese here) after a processed
    /// key is left to the commit the windowing layer delivers, even when
    /// that commit comes in a later frame, so it is never typed twice;
    /// Tamil after a processed key is still kept (#1151).
    #[test]
    fn composed_scripts_are_left_to_the_commit() {
        let mut t = CharTracker::default();
        t.message(WM_KEYDOWN, VK_PROCESSKEY as usize);
        t.message(WM_CHAR, '中' as usize);
        t.message(WM_CHAR, '\u{3042}' as usize);
        t.message(WM_CHAR, '\u{d55c}' as usize);
        assert_eq!(t.lost, "");
        t.message(WM_KEYUP, VK_PROCESSKEY as usize);
        t.message(WM_KEYDOWN, VK_PROCESSKEY as usize);
        t.message(WM_CHAR, '\u{0b95}' as usize);
        assert_eq!(t.lost, "\u{0b95}");
        // With no key before it, any script is kept.
        t.lost.clear();
        t.message(WM_KEYUP, VK_PROCESSKEY as usize);
        t.message(WM_CHAR, '中' as usize);
        assert_eq!(t.lost, "中");
    }

    fn feed(msgs: &[(u32, usize)]) -> String {
        let mut t = CharTracker::default();
        for (m, w) in msgs {
            t.message(*m, *w);
        }
        t.lost
    }

    fn ch(c: char) -> usize {
        c as usize
    }

    #[test]
    fn characters_of_a_key_press_arrive_and_are_not_kept() {
        // A key and its character, a key with two characters (a dead key
        // that could not combine), a key sending a character by itself
        // (VK_PACKET, as keyboard programs inject them).
        let lost = feed(&[
            (WM_KEYDOWN, 0x41),
            (WM_CHAR, ch('a')),
            (WM_KEYUP, 0x41),
            (WM_KEYDOWN, 0x42),
            (WM_CHAR, ch('`')),
            (WM_CHAR, ch('b')),
            (WM_KEYUP, 0x42),
            (WM_KEYDOWN, 0xE7),
            (WM_CHAR, ch('த')),
            (WM_KEYUP, 0xE7),
        ]);
        assert_eq!(lost, "");
    }

    #[test]
    fn characters_with_no_key_press_or_after_a_processed_key_are_kept() {
        // Posted characters (the u sign after a consonant, as an input
        // method types தமிழ்), then a key an input method took, answered
        // with the characters of a letter.
        let lost = feed(&[
            (WM_CHAR, ch('த')),
            (WM_CHAR, ch('ம')),
            (WM_CHAR, ch('\u{0BBF}')),
            (WM_KEYDOWN, 0x5A),
            (WM_KEYUP, 0x5A),
            (WM_CHAR, ch('ழ')),
            (WM_KEYDOWN, VK_PROCESSKEY as usize),
            (WM_CHAR, ch('\u{0BCD}')),
            (WM_KEYUP, 0x4C),
        ]);
        assert_eq!(lost, "தமிழ்");
    }

    #[test]
    fn control_characters_menu_keys_and_dead_keys_are_not_kept() {
        let lost = feed(&[
            // Ctrl+A, Backspace and Enter as posted characters.
            (WM_CHAR, 0x01),
            (WM_CHAR, 0x08),
            (WM_CHAR, 0x0D),
            // Alt+F: a menu's access key.
            (WM_SYSKEYDOWN, 0x46),
            (WM_SYSCHAR, ch('f')),
            (WM_SYSKEYUP, 0x46),
            // A private use character.
            (WM_CHAR, 0xF728),
        ]);
        assert_eq!(lost, "");
    }

    /// #1271: the characters of AltGr (Windows holds Ctrl and Alt for
    /// it) are kept, since the windowing layer drops the text of keys
    /// pressed with Ctrl; once AltGr is let go, keys type as before.
    #[test]
    fn characters_typed_with_altgr_are_kept() {
        let lost = feed(&[
            // AltGr down: Ctrl, then the right Alt.
            (WM_KEYDOWN, VK_CONTROL as usize),
            (WM_KEYDOWN, VK_MENU as usize),
            // Polish: AltGr+A, AltGr+L, AltGr+Z.
            (WM_KEYDOWN, 0x41),
            (WM_CHAR, ch('ą')),
            (WM_KEYUP, 0x41),
            (WM_KEYDOWN, 0x4C),
            (WM_CHAR, ch('ł')),
            (WM_KEYUP, 0x4C),
            (WM_KEYDOWN, 0x5A),
            (WM_CHAR, ch('ż')),
            (WM_KEYUP, 0x5A),
            (WM_KEYUP, VK_MENU as usize),
            (WM_KEYUP, VK_CONTROL as usize),
            // A plain key after it arrives by itself.
            (WM_KEYDOWN, 0x42),
            (WM_CHAR, ch('b')),
            (WM_KEYUP, 0x42),
            // Ctrl alone: a shortcut's control character is not kept.
            (WM_KEYDOWN, VK_CONTROL as usize),
            (WM_KEYDOWN, 0x53),
            (WM_CHAR, 0x13),
            (WM_KEYUP, 0x53),
            (WM_KEYUP, VK_CONTROL as usize),
        ]);
        assert_eq!(lost, "ąłż");
    }

    #[test]
    fn a_character_outside_the_basic_plane_comes_in_two_halves() {
        let mut units = [0u16; 2];
        '𑌕'.encode_utf16(&mut units);
        let lost = feed(&[(WM_CHAR, units[0] as usize), (WM_CHAR, units[1] as usize)]);
        assert_eq!(lost, "𑌕");
        // A second half alone is no character.
        assert_eq!(feed(&[(WM_CHAR, units[1] as usize)]), "");
    }

    #[test]
    fn kept_characters_join_the_frame_less_those_that_arrived() {
        let mut events = vec![egui::Event::Text("க".into())];
        merge("கொ", &mut events);
        assert_eq!(
            events,
            [
                egui::Event::Text("க".into()),
                egui::Event::Text("\u{0BCA}".into())
            ]
        );
        // All arrived (as text or an input method's commit): nothing more.
        let mut events = vec![egui::Event::Ime(egui::ImeEvent::Commit("中文".into()))];
        merge("中文", &mut events);
        assert_eq!(events.len(), 1);
        let mut none = Vec::new();
        merge("", &mut none);
        assert!(none.is_empty());
    }
}
