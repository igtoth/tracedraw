//! Rich tips for the bars: the button's name in bold on the first line
//! and what it does under it, like the standard bar's.
//!
//! A tip is the translated name, optionally followed by "\n" and the
//! description. When only the name comes, the description is found by
//! the name: every key `k` with a `k_desc` gives the pair (tr(k),
//! tr(k_desc)), so the bars keep passing `tr(key)` (or the name with its
//! shortcut, "Name (Ctrl+Q)") and still show the description.

use crate::i18n::tr;
use egui::{Response, Ui};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// The descriptions by name, for the interface language that was used to
/// build them.
fn by_name() -> &'static Mutex<(String, HashMap<String, String>)> {
    static MAP: OnceLock<Mutex<(String, HashMap<String, String>)>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new((String::new(), HashMap::new())))
}

/// The description of the button called `name` in the current language.
pub fn description_of(name: &str) -> Option<String> {
    let lang = crate::i18n::language();
    let mut guard = by_name().lock().ok()?;
    if guard.0 != lang || guard.1.is_empty() {
        let mut map = HashMap::new();
        let mut keys = crate::i18n::english_keys();
        // A fixed order, so the first of two keys with one name wins.
        keys.sort();
        for key in keys {
            if let Some(base) = key.strip_suffix("_desc") {
                map.entry(tr(base)).or_insert_with(|| tr(&key));
            }
        }
        *guard = (lang, map);
    }
    let name = name.trim();
    if let Some(d) = guard.1.get(name) {
        return Some(d.clone());
    }
    // "Name (Ctrl+Q)": the name without its shortcut.
    let bare = name
        .strip_suffix(')')
        .and_then(|n| n.rsplit_once(" ("))
        .map(|(n, _)| n)?;
    guard.1.get(bare).cloned()
}

/// The title and description a tip shows.
pub fn parts(tip: &str) -> (String, Option<String>) {
    match tip.split_once('\n') {
        Some((t, d)) if !d.trim().is_empty() => (t.to_string(), Some(d.to_string())),
        Some((t, _)) => (t.to_string(), description_of(t)),
        None => (tip.to_string(), description_of(tip)),
    }
}

/// Draw a tip: the title in bold, the description under it.
pub fn show(ui: &mut Ui, tip: &str) {
    #[cfg(test)]
    record(tip);
    let (title, desc) = parts(tip);
    // A menu name's "..." says a dialog opens; the tip names the command.
    let title = title.trim_end_matches("...").trim_end_matches('\u{2026}');
    ui.set_max_width(280.0);
    // A tip that is already a sentence (no name above it) reads as one.
    if desc.is_none() && title.chars().count() > 45 {
        ui.label(egui::RichText::new(title).size(12.0));
        return;
    }
    ui.label(egui::RichText::new(title).font(crate::theme::bold(12.0)));
    if let Some(d) = desc {
        ui.label(egui::RichText::new(d).size(12.0));
    }
}

/// Rich tips on a response, in place of `on_hover_text`.
pub trait RichTip {
    fn rich_tip(self, tip: impl AsRef<str>) -> Response;
    fn rich_disabled_tip(self, tip: impl AsRef<str>) -> Response;
}

impl RichTip for Response {
    fn rich_tip(self, tip: impl AsRef<str>) -> Response {
        let tip = tip.as_ref().to_string();
        #[cfg(test)]
        record(&tip);
        self.on_hover_ui(|ui| show(ui, &tip))
    }

    fn rich_disabled_tip(self, tip: impl AsRef<str>) -> Response {
        let tip = tip.as_ref().to_string();
        #[cfg(test)]
        record(&tip);
        self.on_disabled_hover_ui(|ui| show(ui, &tip))
    }
}

/// The tip of `key`: its name, and the description kept under
/// `key_desc` below it.
pub fn keyed(key: &str) -> String {
    format!("{}\n{}", tr(key), tr(&format!("{key}_desc")))
}

/// A tip named by `title_key` and described by `desc_key`.
pub fn pair(title_key: &str, desc_key: &str) -> String {
    format!("{}\n{}", tr(title_key), tr(desc_key))
}

/// Draw `add`, then give everything it drew one rich tip: a field with
/// its spin arrows, a picture and the field after it, a row of options.
/// The controls inside keep working; one that shows a tip of its own
/// keeps it, and this tip stays away.
pub fn tip_over<R>(ui: &mut Ui, tip: impl AsRef<str>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let inner = ui.scope(add);
    cover(
        ui,
        inner.response.rect,
        inner.response.id.with("tip_over"),
        tip,
    );
    inner.inner
}

/// A rich tip over `rect`, already drawn: see [`tip_over`].
pub fn cover(ui: &mut Ui, rect: egui::Rect, id: egui::Id, tip: impl AsRef<str>) {
    // A control inside already showed its own tip this frame.
    let own_tip = ui.rect_contains_pointer(rect)
        && ui
            .ctx()
            .viewport(|v| !v.this_pass.tooltips.widget_tooltips.is_empty());
    if !own_tip {
        ui.interact(rect, id, egui::Sense::hover()).rich_tip(tip);
    }
}

#[cfg(test)]
thread_local! {
    static SEEN: std::cell::RefCell<Option<std::collections::BTreeSet<String>>> =
        const { std::cell::RefCell::new(None) };
}

/// Tests: the tips the bars asked for while recording.
#[cfg(test)]
fn record(tip: &str) {
    SEEN.with(|s| {
        if let Some(set) = s.borrow_mut().as_mut() {
            set.insert(tip.to_string());
        }
    });
}

#[cfg(test)]
pub fn start_recording() {
    SEEN.with(|s| *s.borrow_mut() = Some(Default::default()));
}

#[cfg(test)]
pub fn recorded() -> std::collections::BTreeSet<String> {
    SEEN.with(|s| s.borrow_mut().take().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_find_their_descriptions() {
        crate::i18n::set_language("en");
        let name = tr("toolbar.options");
        let (t, d) = parts(&name);
        assert_eq!(t, name);
        assert_eq!(d, Some(tr("toolbar.options_desc")));
        // With a shortcut after the name.
        let (_, d) = parts(&format!("{name} (Ctrl+J)"));
        assert_eq!(d, Some(tr("toolbar.options_desc")));
        // A given description wins; an unknown name has none.
        let (_, d) = parts("Anything\nIts words");
        assert_eq!(d.as_deref(), Some("Its words"));
        assert_eq!(parts("No such button here").1, None);
    }
}
