//! The text direction keys while text is typed: Right Ctrl with Right
//! Shift makes the paragraphs at the caret run right to left, Left Ctrl
//! with Left Shift left to right. Both keys of one side go down together
//! and come up with no other key pressed in between (so Ctrl+Shift+Arrow
//! still selects by word). Which side's keys are held comes from the
//! system: the windowing layer reports Ctrl and Shift without their side.

use tracedraw_core::TextDirection;

/// The side of the keyboard both Ctrl and Shift are held on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum State {
    #[default]
    Idle,
    /// Ctrl and Shift of one side are held, nothing else yet.
    Armed(Side),
    /// Another key came while they were held: no direction this time.
    Spoiled,
}

/// Follows the keys frame by frame (see [`DirectionGesture::step`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct DirectionGesture {
    state: State,
}

impl DirectionGesture {
    /// One frame: `ctrl_shift` says whether Ctrl and Shift are both held,
    /// `side` which side holds both (`None` when the system cannot tell
    /// or the keys are mixed), `other_key` whether another key went down
    /// in this frame. Gives the direction to set when the keys come up.
    pub fn step(
        &mut self,
        ctrl_shift: bool,
        side: Option<Side>,
        other_key: bool,
    ) -> Option<TextDirection> {
        if ctrl_shift {
            self.state = match self.state {
                _ if other_key => State::Spoiled,
                State::Idle => side.map_or(State::Spoiled, State::Armed),
                State::Armed(s) if side.is_some_and(|now| now != s) => State::Spoiled,
                keep => keep,
            };
            return None;
        }
        let done = match self.state {
            State::Armed(Side::Right) if !other_key => Some(TextDirection::Rtl),
            State::Armed(Side::Left) if !other_key => Some(TextDirection::Ltr),
            _ => None,
        };
        self.state = State::Idle;
        done
    }
}

/// Which side holds both Ctrl and Shift right now, asked of the system.
#[cfg(windows)]
pub fn held_side() -> Option<Side> {
    #[link(name = "user32")]
    extern "system" {
        fn GetAsyncKeyState(key: i32) -> i16;
    }
    const VK_LSHIFT: i32 = 0xA0;
    const VK_RSHIFT: i32 = 0xA1;
    const VK_LCONTROL: i32 = 0xA2;
    const VK_RCONTROL: i32 = 0xA3;
    // SAFETY: GetAsyncKeyState only reads the keyboard state; any key
    // code is allowed.
    let down = |k: i32| unsafe { GetAsyncKeyState(k) } < 0;
    let (ls, rs, lc, rc) = (
        down(VK_LSHIFT),
        down(VK_RSHIFT),
        down(VK_LCONTROL),
        down(VK_RCONTROL),
    );
    match (lc && ls, rc && rs) {
        (false, true) if !lc && !ls => Some(Side::Right),
        (true, false) if !rc && !rs => Some(Side::Left),
        _ => None,
    }
}

/// Elsewhere the side is not known: the text bar's buttons set the
/// direction.
#[cfg(not(windows))]
pub fn held_side() -> Option<Side> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_keys_of_one_side_set_the_direction_when_let_go() {
        let mut g = DirectionGesture::default();
        // Right Ctrl + Right Shift, held a few frames, then let go.
        assert_eq!(g.step(true, Some(Side::Right), false), None);
        assert_eq!(g.step(true, Some(Side::Right), false), None);
        assert_eq!(g.step(false, None, false), Some(TextDirection::Rtl));
        // Left Ctrl + Left Shift.
        assert_eq!(g.step(true, Some(Side::Left), false), None);
        assert_eq!(g.step(false, None, false), Some(TextDirection::Ltr));
        // Nothing held: nothing.
        assert_eq!(g.step(false, None, false), None);
    }

    #[test]
    fn another_key_or_mixed_sides_cancel_it() {
        let mut g = DirectionGesture::default();
        // Ctrl+Shift+Arrow selects by word: no direction.
        assert_eq!(g.step(true, Some(Side::Right), false), None);
        assert_eq!(g.step(true, Some(Side::Right), true), None);
        assert_eq!(g.step(false, None, false), None);
        // Left Ctrl with Right Shift: the system says no side.
        assert_eq!(g.step(true, None, false), None);
        assert_eq!(g.step(false, None, false), None);
        // Starting on one side and adding the other: cancelled.
        assert_eq!(g.step(true, Some(Side::Left), false), None);
        assert_eq!(g.step(true, Some(Side::Right), false), None);
        assert_eq!(g.step(false, None, false), None);
    }
}
