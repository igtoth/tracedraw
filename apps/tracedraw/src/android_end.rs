//! The wait at the end of the Android activity (`android::ending`): the
//! activity's thread waits until the editor's thread has given up the
//! window (which it does after backing the drawings up) before the
//! process ends, never longer than a bound. On its own, without Android
//! types, so that its tests run on every platform.

use std::time::{Duration, Instant};

/// Look at `done` every `step` until it holds or `timeout` has passed;
/// whether it held. It is looked at once more at the end, so a condition
/// that turned true during the last step counts.
pub fn wait_until(done: impl Fn() -> bool, timeout: Duration, step: Duration) -> bool {
    let start = Instant::now();
    loop {
        if done() {
            return true;
        }
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            return false;
        }
        std::thread::sleep(step.min(timeout - elapsed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A window already gone (or never there) ends the wait at once; one
    /// that goes while waiting ends it then; one that stays is given up
    /// on after the bound, so the activity's end is never held for good.
    #[test]
    fn the_end_waits_for_the_window_within_a_bound() {
        let start = Instant::now();
        assert!(wait_until(
            || true,
            Duration::from_secs(10),
            Duration::from_millis(10)
        ));
        assert!(start.elapsed() < Duration::from_secs(1));

        let looks = AtomicUsize::new(0);
        assert!(wait_until(
            || looks.fetch_add(1, Ordering::SeqCst) >= 3,
            Duration::from_secs(10),
            Duration::from_millis(1)
        ));
        assert_eq!(looks.load(Ordering::SeqCst), 4);

        let start = Instant::now();
        assert!(!wait_until(
            || false,
            Duration::from_millis(50),
            Duration::from_millis(10)
        ));
        let waited = start.elapsed();
        assert!(waited >= Duration::from_millis(50));
        assert!(waited < Duration::from_secs(5));
    }
}
