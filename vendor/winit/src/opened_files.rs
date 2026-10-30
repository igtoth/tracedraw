//! TraceDraw: files the system asks the application to open (macOS sends
//! the files opened from Finder, by a double click or Open With, to the
//! running application as an "open documents" event, never on the command
//! line; see `vendor/README.md`). The application delegate hands them
//! here; the program takes them with a handler it sets before the event
//! loop runs. Files that arrive before a handler is set wait for it, so
//! the files a start was launched with are not lost. The file has no
//! dependency on the rest of the crate, so that TraceDraw's tests can
//! include and run it.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

type Handler = Box<dyn Fn(Vec<PathBuf>) + Send>;

struct State {
    handler: Option<Handler>,
    waiting: Vec<PathBuf>,
}

static STATE: Mutex<State> = Mutex::new(State {
    handler: None,
    waiting: Vec::new(),
});

fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Set the handler that receives the files the system asks to open; the
/// files that came before it are handed to it at once.
pub fn set_handler(handler: Handler) {
    let mut s = state();
    let waiting = std::mem::take(&mut s.waiting);
    if !waiting.is_empty() {
        handler(waiting);
    }
    s.handler = Some(handler);
}

/// Files the system asked to open: to the handler, or kept until one is set.
pub fn deliver(files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }
    let mut s = state();
    match &s.handler {
        Some(h) => h(files),
        None => s.waiting.extend(files),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn files_before_the_handler_wait_for_it_and_later_ones_go_straight_to_it() {
        deliver(vec![PathBuf::from("/a.cdr")]);
        deliver(Vec::new());
        deliver(vec![PathBuf::from("/b.tdraw"), PathBuf::from("/c.cdr")]);
        let got = Arc::new(Mutex::new(Vec::<Vec<PathBuf>>::new()));
        let sink = got.clone();
        set_handler(Box::new(move |f| sink.lock().unwrap().push(f)));
        deliver(vec![PathBuf::from("/d.cdr")]);
        let got = got.lock().unwrap();
        assert_eq!(
            *got,
            vec![
                vec![
                    PathBuf::from("/a.cdr"),
                    PathBuf::from("/b.tdraw"),
                    PathBuf::from("/c.cdr")
                ],
                vec![PathBuf::from("/d.cdr")],
            ]
        );
    }
}
