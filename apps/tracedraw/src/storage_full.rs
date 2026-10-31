//! What the browser could not keep (#1414). The browser version keeps the
//! shelf, the settings and the auto-backups in the page's storage
//! (`web.rs`), which a browser may refuse: its quota for the page is
//! full, or storage is turned off for the site (some private windows).
//! Every write that fails is reported here; the editor tells the user
//! once per kind and session with a message, and what could not be kept
//! stays in memory for as long as the tab is open. See
//! `docs/behavior/web.md`, Storage.

// Only the browser build writes to a browser's storage.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::cell::RefCell;

/// What the browser keeps for the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    Shelf,
    Settings,
    Backup,
}

/// A write the browser refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    pub what: Kept,
    /// The page's storage quota is full (rather than storage refused).
    pub full: bool,
}

thread_local! {
    /// Refusals not looked at yet.
    static REFUSED: RefCell<Vec<Refusal>> = const { RefCell::new(Vec::new()) };
    /// The kinds the user was told about in this session.
    static TOLD: RefCell<Vec<Kept>> = const { RefCell::new(Vec::new()) };
}

/// Whether a browser error name means the storage quota is full: the
/// standard `QuotaExceededError`, and the names older Firefox gave it.
pub fn is_quota_error(name: &str) -> bool {
    matches!(
        name,
        "QuotaExceededError" | "NS_ERROR_DOM_QUOTA_REACHED" | "NS_ERROR_FILE_NO_DEVICE_SPACE"
    )
}

/// The browser refused to keep `what`.
pub fn report(what: Kept, full: bool) {
    log::warn!(
        "the browser did not keep {what:?}{}",
        if full { " (storage full)" } else { "" }
    );
    REFUSED.with(|r| r.borrow_mut().push(Refusal { what, full }));
}

/// The first refusal of a kind the user was not told about yet in this
/// session (marked as told); the others are dropped.
pub fn take_new() -> Option<Refusal> {
    let refused: Vec<Refusal> = REFUSED.with(|r| std::mem::take(&mut *r.borrow_mut()));
    TOLD.with(|t| {
        let mut told = t.borrow_mut();
        let first = refused.into_iter().find(|r| !told.contains(&r.what))?;
        told.push(first.what);
        Some(first)
    })
}

/// Whether a refusal is waiting to be looked at.
pub fn pending() -> bool {
    REFUSED.with(|r| !r.borrow().is_empty())
}

/// The message telling the user about `r`: what was not kept, why, and
/// that it stays in this tab.
pub fn message(r: Refusal) -> String {
    use crate::i18n::tr;
    let title = match r.what {
        Kept::Shelf => tr("storage.shelf_not_kept"),
        Kept::Settings => tr("storage.settings_not_kept"),
        Kept::Backup => tr("storage.backup_not_kept"),
    };
    let why = if r.full {
        tr("storage.full")
    } else {
        tr("storage.refused")
    };
    format!("{title}\n\n{why}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #1414: each kind is told once per session, the full quota and a
    /// refused storage apart; refusals of a kind already told are
    /// dropped.
    #[test]
    fn each_kind_is_told_once() {
        while take_new().is_some() {}
        assert!(is_quota_error("QuotaExceededError"));
        assert!(is_quota_error("NS_ERROR_DOM_QUOTA_REACHED"));
        assert!(!is_quota_error("SecurityError"));
        assert!(!is_quota_error("InvalidStateError"));
        report(Kept::Shelf, true);
        report(Kept::Shelf, true);
        assert!(pending());
        assert_eq!(
            take_new(),
            Some(Refusal {
                what: Kept::Shelf,
                full: true
            })
        );
        assert!(!pending());
        report(Kept::Shelf, true);
        assert_eq!(take_new(), None, "the shelf was told about");
        report(Kept::Settings, false);
        let r = take_new().expect("settings");
        assert_eq!(r.what, Kept::Settings);
        assert!(!r.full);
        assert!(message(r).contains(&crate::i18n::tr("storage.refused")));
        assert!(message(Refusal {
            what: Kept::Backup,
            full: true
        })
        .contains(&crate::i18n::tr("storage.full")));
    }
}
