//! Text > Writing Tools: one dialog with three tabs (Spell Checker,
//! Grammar, Thesaurus), "?" and Close; and Text > Custom Optional
//! Hyphens. See docs/behavior/writing-tools.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::window;
use crate::ui::tips::RichTip;
use egui::{Context, Ui};
use tracedraw_core::{document::ShapeKind, ShapeId};

/// What the spell checker and the grammar checker look at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CheckScope {
    /// Every text object of the page.
    #[default]
    Document,
    /// The selected text objects.
    Selection,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellTab {
    /// Started: the unknown words are listed.
    pub started: bool,
    pub unknown: Vec<(ShapeId, String)>,
    pub cursor: usize,
    pub replacement: String,
    pub dictionary_words: usize,
    /// Words skipped for this check.
    pub skipped: Vec<String>,
    /// Replacements made, newest last, for Undo.
    pub done: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GrammarTab {
    pub started: bool,
    pub findings: Vec<(ShapeId, crate::grammar::Finding)>,
    pub cursor: usize,
    pub skipped_rules: Vec<crate::grammar::Rule>,
    pub done: usize,
    /// "Grammar check completed. Close the grammar checker?"
    pub ask_close: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThesaurusTab {
    pub word: String,
    /// Words looked up, newest first (the word list).
    pub history: Vec<String>,
    pub original: String,
    pub target: Option<ShapeId>,
    pub looked_up: Option<String>,
    pub meanings: Vec<crate::thesaurus::Meaning>,
    pub selected: Option<String>,
    pub started: bool,
    /// Close the dialog after Insert.
    pub auto_close: bool,
}

/// Text > Writing Tools.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WritingToolsState {
    /// 0 Spell Checker, 1 Grammar, 2 Thesaurus.
    pub tab: usize,
    pub scope: CheckScope,
    pub spell: SpellTab,
    pub grammar: GrammarTab,
    pub thesaurus: ThesaurusTab,
}

impl WritingToolsState {
    pub fn on_tab(tab: usize) -> Self {
        WritingToolsState {
            tab,
            ..Default::default()
        }
    }
}

/// The text objects a check looks at.
fn scope_ids(app: &App, scope: CheckScope) -> Vec<ShapeId> {
    match scope {
        CheckScope::Selection => app.text_shapes().iter().map(|s| s.id).collect(),
        CheckScope::Document => app
            .doc()
            .page(app.page)
            .map(|p| {
                p.layers
                    .iter()
                    .flat_map(|l| &l.shapes)
                    .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// The unknown words of the text objects `ids`, in order, each place once.
pub fn unknown_words(app: &App, ids: &[ShapeId], tab: &SpellTab) -> Vec<(ShapeId, String)> {
    let prefs = &app.settings.spelling;
    let mut out = Vec::new();
    for id in ids {
        let Some(s) = app.doc().find_shape(*id) else {
            continue;
        };
        let ShapeKind::Text { spans, .. } = &s.kind else {
            continue;
        };
        let text: String = spans.iter().map(|x| x.text.as_str()).collect();
        for w in crate::spell::unknown_words(&text) {
            if tab.skipped.contains(&w)
                || (prefs.ignore_digits && w.chars().any(|c| c.is_ascii_digit()))
                || (prefs.ignore_capitals && w.chars().all(|c| !c.is_lowercase()))
            {
                continue;
            }
            out.push((*id, w));
        }
    }
    out
}

/// A spelling correction as an Autocorrect replacement (Options > Text >
/// Spelling > Add corrections to Autocorrect): `word` to `with`, the
/// row for `word` replaced if there is one; nothing for an empty or
/// unchanged word, or one with spaces.
pub fn add_correction(prefs: &mut crate::autocorrect::AutocorrectPrefs, word: &str, with: &str) {
    if word.is_empty() || word == with || word.contains(char::is_whitespace) {
        return;
    }
    match prefs.replacements.iter_mut().find(|(f, _)| f == word) {
        Some(row) => row.1 = with.to_string(),
        None => prefs
            .replacements
            .push((word.to_string(), with.to_string())),
    }
}

/// A right-hand column button, 140 wide.
fn side_button(ui: &mut Ui, key: &str, enabled: bool) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(tr(key)).min_size(egui::vec2(140.0, 24.0)),
    )
    .clicked()
}

/// A read-only status box.
fn status_box(ui: &mut Ui, text: &str) {
    let mut t = text.to_string();
    ui.add(
        egui::TextEdit::singleline(&mut t)
            .desired_width(230.0)
            .interactive(false),
    );
}

/// Draw the dialog.
pub fn dialog(app: &mut App, ctx: &Context, st: &mut WritingToolsState, close: &mut bool) {
    let mut help = false;
    window(ctx, tr("writing.title")).show(ctx, |ui| {
        ui.set_min_width(700.0);
        ui.horizontal(|ui| {
            for (i, key) in [
                "writing.spell_checker",
                "writing.grammar",
                "writing.thesaurus",
            ]
            .iter()
            .enumerate()
            {
                if ui.selectable_label(st.tab == i, tr(key)).clicked() {
                    st.tab = i;
                }
            }
        });
        ui.separator();
        egui::Frame::new()
            .inner_margin(egui::Margin::same(6))
            .show(ui, |ui| {
                ui.set_min_height(210.0);
                match st.tab {
                    0 => spell_tab(app, ui, st),
                    1 => grammar_tab(app, ui, st, close),
                    _ => thesaurus_tab(app, ui, st, close),
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            help = ui
                .add_sized([27.0, 27.0], egui::Button::new("?"))
                .rich_tip(tr("dialog.help"))
                .clicked();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.close")))
                    .clicked()
                {
                    *close = true;
                }
            });
        });
    });
    if help {
        app.open_dialog_help();
    }
}

fn spell_tab(app: &mut App, ui: &mut Ui, st: &mut WritingToolsState) {
    let scope = st.scope;
    let t = &mut st.spell;
    let current = t.unknown.get(t.cursor).cloned();
    let mut start = false;
    let mut replace_one = false;
    let mut replace_all = false;
    let mut skip_once = false;
    let mut skip_all = false;
    let mut add = false;
    let mut undo = false;
    ui.horizontal_top(|ui| {
        egui::Grid::new("spell_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("writing.replace_with"));
                ui.add_enabled(
                    current.is_some(),
                    egui::TextEdit::singleline(&mut t.replacement).desired_width(230.0),
                );
                ui.end_row();
                ui.label(tr("writing.replacements"));
                egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(230.0, 110.0));
                        egui::ScrollArea::vertical()
                            .max_height(110.0)
                            .show(ui, |ui| {
                                if let Some((_, w)) = &current {
                                    for s in crate::spell::suggest(w) {
                                        let on = t.replacement == s;
                                        let r = ui.selectable_label(on, &s);
                                        if r.clicked() {
                                            t.replacement = s.clone();
                                        }
                                        if r.double_clicked() {
                                            t.replacement = s;
                                            replace_one = true;
                                        }
                                    }
                                }
                            });
                    });
                ui.end_row();
                let (label, status) = if !t.started {
                    (tr("writing.ready"), tr("writing.begin_spell"))
                } else if let Some((_, w)) = &current {
                    (tr("writing.unknown_word"), w.clone())
                } else {
                    (tr("writing.ready"), tr("writing.spell_done"))
                };
                ui.label(label);
                status_box(ui, &status);
                ui.end_row();
            });
        ui.vertical(|ui| {
            egui::Grid::new("spell_buttons")
                .num_columns(2)
                .spacing([6.0, 6.0])
                .show(ui, |ui| {
                    if side_button(
                        ui,
                        if current.is_some() {
                            "writing.replace"
                        } else {
                            "writing.start"
                        },
                        true,
                    ) {
                        if current.is_some() {
                            replace_one = true;
                        } else {
                            start = true;
                        }
                    }
                    replace_all |= side_button(ui, "writing.auto_replace", current.is_some());
                    ui.end_row();
                    skip_once |= side_button(ui, "writing.skip_once", current.is_some());
                    undo |= side_button(ui, "writing.undo", t.done > 0);
                    ui.end_row();
                    skip_all |= side_button(ui, "writing.skip_all", current.is_some());
                    ui.menu_button(tr("writing.options"), |ui| {
                        let p = &mut app.settings.spelling;
                        let mut changed = ui
                            .checkbox(&mut p.ignore_digits, tr("writing.ignore_digits"))
                            .changed();
                        changed |= ui
                            .checkbox(&mut p.ignore_capitals, tr("writing.ignore_capitals"))
                            .changed();
                        changed |= ui
                            .checkbox(&mut p.add_to_autocorrect, tr("options.add_to_autocorrect"))
                            .changed();
                        if changed {
                            app.settings.save();
                        }
                    });
                    ui.end_row();
                    add |= side_button(ui, "writing.add", current.is_some());
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                ui.label(tr("writing.check"));
                egui::ComboBox::from_id_salt("spell_scope")
                    .selected_text(tr(match scope {
                        CheckScope::Document => "writing.scope_document",
                        CheckScope::Selection => "writing.scope_selection",
                    }))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut st.scope,
                            CheckScope::Document,
                            tr("writing.scope_document"),
                        );
                        ui.selectable_value(
                            &mut st.scope,
                            CheckScope::Selection,
                            tr("writing.scope_selection"),
                        );
                    });
            });
        });
    });
    let t = &mut st.spell;
    if t.started && t.dictionary_words == 0 {
        ui.colored_label(
            egui::Color32::from_rgb(180, 80, 0),
            tr("dialog.no_dictionary"),
        );
    }
    let refresh = |app: &App, t: &mut SpellTab, scope: CheckScope| {
        let ids = scope_ids(app, scope);
        t.unknown = unknown_words(app, &ids, t);
        t.cursor = t.cursor.min(t.unknown.len());
        t.replacement = t
            .unknown
            .get(t.cursor)
            .map(|(_, w)| {
                crate::spell::suggest(w)
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| w.clone())
            })
            .unwrap_or_default();
    };
    if start {
        t.started = true;
        t.cursor = 0;
        t.skipped.clear();
        t.dictionary_words = crate::spell::check_document(app).1;
        refresh(app, t, st.scope);
    }
    if let Some((id, word)) = current {
        if replace_one || replace_all {
            if replace_all {
                for (sid, w) in t.unknown.clone() {
                    if w == word {
                        crate::spell::replace_word(app, sid, &word, &t.replacement);
                        t.done += 1;
                    }
                }
            } else {
                crate::spell::replace_word(app, id, &word, &t.replacement);
                t.done += 1;
            }
            if app.settings.spelling.add_to_autocorrect {
                add_correction(&mut app.settings.autocorrect, &word, &t.replacement);
                app.settings.save();
            }
            refresh(app, t, st.scope);
        } else if skip_once {
            t.cursor += 1;
            t.replacement = t
                .unknown
                .get(t.cursor)
                .map(|(_, w)| w.clone())
                .unwrap_or_default();
        } else if skip_all {
            t.skipped.push(word);
            refresh(app, t, st.scope);
        } else if add {
            crate::spell::add_user_word(&word);
            refresh(app, t, st.scope);
        }
    }
    if undo && t.done > 0 {
        app.undo();
        t.done -= 1;
        refresh(app, t, st.scope);
    }
}

fn grammar_options() -> crate::grammar::Options {
    crate::grammar::Options::for_language(&crate::i18n::language())
}

/// The findings of the text objects `ids`, rules skipped left out.
pub fn grammar_findings(
    app: &App,
    ids: &[ShapeId],
    skipped: &[crate::grammar::Rule],
) -> Vec<(ShapeId, crate::grammar::Finding)> {
    let mut out = Vec::new();
    for id in ids {
        let Some(s) = app.doc().find_shape(*id) else {
            continue;
        };
        let ShapeKind::Text { spans, .. } = &s.kind else {
            continue;
        };
        let text: String = spans.iter().map(|x| x.text.as_str()).collect();
        for f in crate::grammar::check_text(&text, grammar_options()) {
            if !skipped.contains(&f.rule) {
                out.push((*id, f));
            }
        }
    }
    out
}

fn grammar_tab(app: &mut App, ui: &mut Ui, st: &mut WritingToolsState, close: &mut bool) {
    let scope = st.scope;
    let t = &mut st.grammar;
    let current = t.findings.get(t.cursor).cloned();
    let (mut start, mut replace, mut skip, mut skip_rule, mut undo) =
        (false, false, false, false, false);
    ui.horizontal_top(|ui| {
        egui::Grid::new("grammar_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("writing.replacements"));
                egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(260.0, 60.0));
                        if let Some((_, f)) = &current {
                            if let Some(fix) = &f.fix {
                                let shown = if fix.text.is_empty() {
                                    tr("writing.delete")
                                } else {
                                    format!("\"{}\"", fix.text)
                                };
                                ui.label(shown);
                            }
                        }
                    });
                ui.end_row();
                ui.label(tr("writing.new_sentence"));
                let mut preview = current
                    .as_ref()
                    .and_then(|(_, f)| f.fix.as_ref().map(|_| f.sentence.clone()))
                    .unwrap_or_default();
                if let Some((id, f)) = &current {
                    if let (Some(fix), Some(s)) = (&f.fix, app.doc().find_shape(*id)) {
                        if let ShapeKind::Text { spans, .. } = &s.kind {
                            let text: String = spans.iter().map(|x| x.text.as_str()).collect();
                            let fixed = crate::grammar::apply_fix(&text, fix);
                            preview = fixed
                                .lines()
                                .find(|l| {
                                    !l.trim().is_empty()
                                        && f.sentence
                                            .split_whitespace()
                                            .next()
                                            .is_some_and(|w| l.contains(w))
                                })
                                .unwrap_or(&fixed)
                                .trim()
                                .to_string();
                        }
                    }
                }
                ui.add(
                    egui::TextEdit::multiline(&mut preview)
                        .desired_width(260.0)
                        .desired_rows(3)
                        .interactive(false),
                );
                ui.end_row();
                ui.label(tr("writing.rule_name"));
                let rule = current.as_ref().map(|(_, f)| {
                    if f.rule == crate::grammar::Rule::LongSentence {
                        trf(
                            f.rule.key(),
                            &[("n", &crate::grammar::LONG_SENTENCE_WORDS.to_string())],
                        )
                    } else {
                        tr(f.rule.key())
                    }
                });
                let status = match (&rule, t.started) {
                    (Some(r), _) => r.clone(),
                    (None, false) => tr("writing.begin_grammar"),
                    (None, true) => tr("grammar.no_issues"),
                };
                status_box(ui, &status);
                ui.end_row();
            });
        ui.vertical(|ui| {
            egui::Grid::new("grammar_buttons")
                .num_columns(2)
                .spacing([6.0, 6.0])
                .show(ui, |ui| {
                    let has_fix = current.as_ref().is_some_and(|(_, f)| f.fix.is_some());
                    if side_button(
                        ui,
                        if current.is_some() {
                            "writing.replace"
                        } else {
                            "writing.start"
                        },
                        current.is_none() || has_fix,
                    ) {
                        if current.is_some() {
                            replace = true;
                        } else {
                            start = true;
                        }
                    }
                    undo |= side_button(ui, "writing.undo", t.done > 0);
                    ui.end_row();
                    skip |= side_button(ui, "writing.skip_once", current.is_some());
                    skip_rule |= side_button(ui, "writing.skip_rule", current.is_some());
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                ui.label(tr("writing.check"));
                egui::ComboBox::from_id_salt("grammar_scope")
                    .selected_text(tr(match scope {
                        CheckScope::Document => "writing.scope_document",
                        CheckScope::Selection => "writing.scope_selection",
                    }))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut st.scope,
                            CheckScope::Document,
                            tr("writing.scope_document"),
                        );
                        ui.selectable_value(
                            &mut st.scope,
                            CheckScope::Selection,
                            tr("writing.scope_selection"),
                        );
                    });
            });
        });
    });
    let t = &mut st.grammar;
    let refresh = |app: &App, t: &mut GrammarTab, scope: CheckScope| {
        let ids = scope_ids(app, scope);
        t.findings = grammar_findings(app, &ids, &t.skipped_rules);
        t.cursor = t.cursor.min(t.findings.len());
    };
    if start {
        t.started = true;
        t.cursor = 0;
        t.skipped_rules.clear();
        refresh(app, t, st.scope);
        t.ask_close = t.findings.is_empty();
    }
    if let Some((id, f)) = current {
        if replace {
            if let Some(fix) = &f.fix {
                crate::grammar::apply_fix_to_shape(app, id, fix);
                t.done += 1;
            }
            refresh(app, t, st.scope);
        } else if skip {
            t.cursor += 1;
        } else if skip_rule {
            t.skipped_rules.push(f.rule);
            refresh(app, t, st.scope);
        }
        if t.cursor >= t.findings.len() {
            t.ask_close = true;
        }
    }
    if undo && t.done > 0 {
        app.undo();
        t.done -= 1;
        refresh(app, t, st.scope);
    }
    if t.ask_close {
        let mut answer = None;
        egui::Modal::new(egui::Id::new("grammar_done")).show(ui.ctx(), |ui| {
            ui.set_width(320.0);
            ui.label(egui::RichText::new(tr("writing.grammar")).strong());
            ui.label(tr("writing.grammar_done"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_sized([100.0, 26.0], egui::Button::new(tr("dialog.yes")))
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui
                    .add_sized([100.0, 26.0], egui::Button::new(tr("dialog.no")))
                    .clicked()
                {
                    answer = Some(false);
                }
            });
        });
        match answer {
            Some(true) => {
                t.ask_close = false;
                *close = true;
            }
            Some(false) => {
                t.ask_close = false;
                t.started = false;
                t.findings.clear();
            }
            None => {}
        }
    }
}

fn thesaurus_lookup(app: &App, t: &mut ThesaurusTab) {
    let th = crate::thesaurus::get(app.settings.thesaurus_file.as_deref());
    let w = t.word.trim().to_string();
    t.meanings = th.lookup(&w).to_vec();
    t.looked_up = Some(w.clone());
    t.selected = None;
    if !w.is_empty() {
        t.history.retain(|h| *h != w);
        t.history.insert(0, w);
        t.history.truncate(20);
    }
}

fn thesaurus_tab(app: &mut App, ui: &mut Ui, st: &mut WritingToolsState, close: &mut bool) {
    let t = &mut st.thesaurus;
    if !t.started {
        t.started = true;
        if let Some(s) = app.text_shapes().first() {
            if let ShapeKind::Text { spans, .. } = &s.kind {
                let text: String = spans.iter().map(|x| x.text.as_str()).collect();
                if let Some(w) = crate::thesaurus::first_word(&text) {
                    t.word = w.clone();
                    t.original = w;
                    t.target = Some(s.id);
                }
            }
        }
        if !t.word.is_empty() {
            thesaurus_lookup(app, t);
        }
    }
    let mut look = false;
    let mut insert = false;
    let mut pick_file = false;
    let mut relookup = None;
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(&mut t.word).desired_width(330.0));
        let history = t.history.clone();
        egui::ComboBox::from_id_salt("thesaurus_history")
            .width(20.0)
            .selected_text("")
            .show_ui(ui, |ui| {
                for h in &history {
                    if ui.selectable_label(false, h).clicked() {
                        relookup = Some(h.clone());
                    }
                }
            });
        look |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        look |= ui
            .add_sized([100.0, 24.0], egui::Button::new(tr("thesaurus.look_up")))
            .clicked();
        insert |= ui
            .add_enabled(
                t.selected.is_some(),
                egui::Button::new(tr("writing.insert")).min_size(egui::vec2(100.0, 24.0)),
            )
            .clicked();
        ui.menu_button(tr("writing.options"), |ui| {
            ui.checkbox(&mut t.auto_close, tr("writing.auto_close"));
            if ui.button(tr("thesaurus.pick_file")).clicked() {
                pick_file = true;
                ui.close();
            }
        });
    });
    let th = crate::thesaurus::get(app.settings.thesaurus_file.as_deref());
    match &th.source {
        crate::thesaurus::Source::BuiltIn => {
            ui.label(
                egui::RichText::new(tr("thesaurus.built_in_note"))
                    .size(11.0)
                    .color(Tokens::text_dim()),
            );
        }
        crate::thesaurus::Source::File(p) => {
            ui.label(
                egui::RichText::new(trf(
                    "thesaurus.source_file",
                    &[
                        ("path", &p.display().to_string()),
                        ("n", &th.len().to_string()),
                    ],
                ))
                .size(11.0)
                .color(Tokens::text_dim()),
            );
        }
    }
    if let Some(w) = &t.looked_up {
        if t.meanings.is_empty() && !w.is_empty() {
            ui.label(trf("thesaurus.no_entry", &[("w", w)]));
        }
        egui::ScrollArea::vertical()
            .max_height(160.0)
            .show(ui, |ui| {
                for m in &t.meanings {
                    ui.label(
                        egui::RichText::new(format!("({})", m.pos))
                            .italics()
                            .color(Tokens::text_dim()),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for syn in &m.synonyms {
                            let on = t.selected.as_deref() == Some(syn.as_str());
                            let r = ui.selectable_label(on, syn);
                            if r.clicked() {
                                t.selected = Some(syn.clone());
                            }
                            if r.double_clicked() {
                                relookup = Some(syn.clone());
                            }
                        }
                    });
                }
            });
    }
    if let Some(w) = relookup {
        t.word = w;
        look = true;
    }
    if look {
        thesaurus_lookup(app, t);
    }
    if insert {
        if let Some(sel) = t.selected.clone() {
            match t.target {
                Some(id) if !t.original.is_empty() => {
                    let repl = crate::thesaurus::match_case(&t.original, &sel);
                    crate::spell::replace_word(app, id, &t.original, &repl);
                    app.status = trf("thesaurus.replaced", &[("a", &t.original), ("b", &repl)]);
                    t.original = repl.clone();
                    t.word = repl;
                }
                _ if app.text_edit.is_some() => app.insert_text(&sel),
                _ => {}
            }
            if t.auto_close {
                *close = true;
            } else {
                thesaurus_lookup(app, t);
            }
        }
    }
    if pick_file {
        crate::files::Dialog::new()
            .add_filter("MyThes", &["dat"])
            .pick_into("thesaurus_file");
    }
    if let Some(p) = crate::files::take_picked("thesaurus_file") {
        app.settings.thesaurus_file = Some(p);
        app.settings.save();
        thesaurus_lookup(app, t);
    }
}

// ---- Custom Optional Hyphens -------------------------------------------

/// Text > Custom Optional Hyphens.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CustomHyphensState {
    pub loaded: bool,
    pub when_typing: bool,
    pub when_pasting: bool,
    pub language: String,
    pub all_languages: bool,
    pub word: String,
    pub hyphenated: String,
    /// (language, word, hyphenated word).
    pub definitions: Vec<(String, String, String)>,
    pub selected: Option<usize>,
}

/// The languages offered: the interface's, else every one.
const LANGUAGES: [(&str, &str); 18] = [
    ("en", "English"),
    ("pt", "Português"),
    ("es", "Español"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("it", "Italiano"),
    ("nl", "Nederlands"),
    ("pl", "Polski"),
    ("cs", "Čeština"),
    ("ru", "Русский"),
    ("tr", "Türkçe"),
    ("ar", "العربية"),
    ("hi", "हिन्दी"),
    ("bn", "বাংলা"),
    ("id", "Bahasa Indonesia"),
    ("ja", "日本語"),
    ("zh", "中文"),
    ("sv", "Svenska"),
];

fn language_name(code: &str) -> String {
    LANGUAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or_else(|| code.to_string(), |(_, n)| n.to_string())
}

/// A hyphenated word ("hy-phen-a-tion") as the plain word and its break
/// points (character counts before each hyphen). `None` when the letters
/// do not match `word` (case aside) or no hyphen is given.
pub fn parse_hyphenated(word: &str, hyphenated: &str) -> Option<Vec<usize>> {
    let mut points = Vec::new();
    let mut letters = String::new();
    for c in hyphenated.trim().chars() {
        if c == '-' || c == '\u{00AD}' || c == '=' {
            let n = letters.chars().count();
            if n > 0 && !points.contains(&n) {
                points.push(n);
            }
        } else {
            letters.push(c);
        }
    }
    let ok = letters.to_lowercase() == word.trim().to_lowercase()
        && !points.is_empty()
        && points.iter().all(|p| *p < letters.chars().count());
    ok.then_some(points)
}

/// Give the text engine the definitions kept in the settings.
pub fn install_custom_hyphens(app: &App) {
    let defs: Vec<(String, Vec<usize>)> = app
        .settings
        .custom_hyphens
        .iter()
        .filter_map(|(_, w, h)| parse_hyphenated(w, h).map(|p| (w.to_lowercase(), p)))
        .collect();
    tracedraw_text::hyphen::set_custom(defs);
    app.raster.borrow_mut().invalidate();
}

/// Turn hyphenation on for the text objects `ids` holding a defined word,
/// so their custom optional hyphens break lines. Returns how many changed.
pub fn apply_custom_hyphens(app: &mut App, ids: &[ShapeId]) -> usize {
    let words: Vec<String> = app
        .settings
        .custom_hyphens
        .iter()
        .map(|(_, w, _)| w.to_lowercase())
        .collect();
    let mut n = 0;
    for id in ids {
        let Some(s) = app.doc().find_shape(*id).cloned() else {
            continue;
        };
        let ShapeKind::Text { spans, para, .. } = &s.kind else {
            continue;
        };
        if para.hyphenate {
            continue;
        }
        let text: String = spans
            .iter()
            .map(|x| x.text.as_str())
            .collect::<String>()
            .to_lowercase();
        let has = text
            .split(|c: char| !c.is_alphabetic())
            .any(|w| words.iter().any(|d| d == w));
        if has {
            app.edit_paragraph_style(*id, |p| p.hyphenate = true);
            n += 1;
        }
    }
    n
}

pub fn custom_hyphens_dialog(
    app: &mut App,
    ctx: &Context,
    st: &mut CustomHyphensState,
    close: &mut bool,
) {
    if !st.loaded {
        st.loaded = true;
        st.when_typing = app.settings.hyphens_when_typing;
        st.when_pasting = app.settings.hyphens_when_pasting;
        st.definitions = app.settings.custom_hyphens.clone();
        let lang = crate::i18n::language();
        st.language = lang.split('-').next().unwrap_or("en").to_string();
    }
    let mut ok = false;
    let mut apply = false;
    let mut help = false;
    window(ctx, tr("hyphens.title")).show(ctx, |ui| {
        ui.set_min_width(420.0);
        ui.label(tr("hyphens.auto_insert"));
        ui.checkbox(&mut st.when_typing, tr("hyphens.when_typing"));
        ui.checkbox(&mut st.when_pasting, tr("hyphens.when_pasting"));
        ui.add_space(4.0);
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(0xC8)))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(tr("hyphens.define"));
                egui::Grid::new("hyphens_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(tr("hyphens.language"));
                        let ui_lang = crate::i18n::language();
                        let ui_lang = ui_lang.split('-').next().unwrap_or("en").to_string();
                        egui::ComboBox::from_id_salt("hyphens_lang")
                            .width(230.0)
                            .selected_text(language_name(&st.language))
                            .show_ui(ui, |ui| {
                                for (code, name) in LANGUAGES {
                                    if st.all_languages || code == ui_lang || code == "en" {
                                        ui.selectable_value(
                                            &mut st.language,
                                            code.to_string(),
                                            name,
                                        );
                                    }
                                }
                            });
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut st.all_languages, tr("hyphens.all_languages"));
                        ui.end_row();
                        ui.label(tr("hyphens.word"));
                        ui.add(egui::TextEdit::singleline(&mut st.word).desired_width(230.0));
                        ui.end_row();
                        ui.label(tr("hyphens.hyphenated"));
                        ui.add(
                            egui::TextEdit::singleline(&mut st.hyphenated)
                                .desired_width(230.0)
                                .hint_text("hy-phen-a-tion"),
                        );
                        ui.end_row();
                    });
                let valid = parse_hyphenated(&st.word, &st.hyphenated).is_some();
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            valid,
                            egui::Button::new(tr("hyphens.add")).min_size(egui::vec2(130.0, 24.0)),
                        )
                        .clicked()
                    {
                        let w = st.word.trim().to_string();
                        st.definitions
                            .retain(|(l, x, _)| !(l == &st.language && x.eq_ignore_ascii_case(&w)));
                        st.definitions.push((
                            st.language.clone(),
                            w,
                            st.hyphenated.trim().to_string(),
                        ));
                        st.word.clear();
                        st.hyphenated.clear();
                    }
                    if ui
                        .add_enabled(
                            st.selected.is_some(),
                            egui::Button::new(tr("hyphens.remove"))
                                .min_size(egui::vec2(130.0, 24.0)),
                        )
                        .clicked()
                    {
                        if let Some(i) = st.selected.take() {
                            if i < st.definitions.len() {
                                st.definitions.remove(i);
                            }
                        }
                    }
                });
                if !st.hyphenated.trim().is_empty() && !valid {
                    ui.label(
                        egui::RichText::new(tr("hyphens.mismatch"))
                            .size(11.0)
                            .color(Tokens::text_dim()),
                    );
                }
                ui.add_space(4.0);
                ui.label(tr("hyphens.list"));
                egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(ui.available_width(), 140.0));
                        egui::Grid::new("hyphens_list")
                            .num_columns(2)
                            .striped(true)
                            .min_col_width(180.0)
                            .show(ui, |ui| {
                                ui.strong(tr("hyphens.col_word"));
                                ui.strong(tr("hyphens.col_hyphenated"));
                                ui.end_row();
                                for (i, (l, w, h)) in st.definitions.iter().enumerate() {
                                    if !st.all_languages && *l != st.language {
                                        continue;
                                    }
                                    let on = st.selected == Some(i);
                                    if ui.selectable_label(on, w).clicked() {
                                        st.selected = Some(i);
                                    }
                                    if ui.selectable_label(on, h).clicked() {
                                        st.selected = Some(i);
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            help = ui
                .add_sized([27.0, 27.0], egui::Button::new("?"))
                .rich_tip(tr("dialog.help"))
                .clicked();
            apply = ui
                .add_enabled(
                    !app.text_shapes().is_empty(),
                    egui::Button::new(tr("hyphens.apply_selection"))
                        .min_size(egui::vec2(140.0, 27.0)),
                )
                .clicked();
            ok = crate::ui::dialogs::ok_cancel(ui, close);
        });
    });
    if help {
        app.open_dialog_help();
    }
    if ok || apply {
        app.settings.hyphens_when_typing = st.when_typing;
        app.settings.hyphens_when_pasting = st.when_pasting;
        app.settings.custom_hyphens = st.definitions.clone();
        app.save_settings();
        install_custom_hyphens(app);
    }
    if apply {
        let ids: Vec<ShapeId> = app.text_shapes().iter().map(|s| s.id).collect();
        let n = apply_custom_hyphens(app, &ids);
        app.status = trf("hyphens.applied", &[("n", &n.to_string())]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;
    use tracedraw_core::{ParagraphStyle, Point, Size, TextAlign, TextSpan};

    fn text(app: &mut App, s: &str, frame: Option<Size>) -> ShapeId {
        app.new_shape(ShapeKind::Text {
            spans: vec![TextSpan::new(s, "Arial", 12.0)],
            origin: Point::new(10.0, 10.0),
            frame,
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        })
        .expect("text")
    }

    /// Text > Use Hyphenation turns hyphenation on for the selected text
    /// and off again, the menu's check following it; with nothing
    /// selected it is the setting new text takes (#255).
    #[test]
    fn use_hyphenation_acts_on_the_selected_text() {
        let mut app = App::headless();
        let hy = |app: &App, id| match &app.doc().find_shape(id).expect("text").kind {
            ShapeKind::Text { para, .. } => para.hyphenate,
            _ => false,
        };
        let a = text(&mut app, "una informazione", Some(Size::new(30.0, 40.0)));
        let b = text(&mut app, "sillabazione", Some(Size::new(30.0, 40.0)));
        assert!(!app.text_hyphenation && !hy(&app, a));
        app.select(vec![a, b]);
        assert!(!app.uses_hyphenation());
        app.toggle_use_hyphenation();
        assert!(hy(&app, a) && hy(&app, b));
        assert!(app.uses_hyphenation());
        assert!(!app.text_hyphenation, "new text keeps its setting");
        app.toggle_use_hyphenation();
        assert!(!hy(&app, a) && !hy(&app, b));
        app.select(Vec::new());
        app.toggle_use_hyphenation();
        assert!(app.text_hyphenation && app.uses_hyphenation());
        app.text_hyphenation = false;
    }

    /// Typing in a text keeps its own hyphenation (it took the setting
    /// for new text, so editing turned it on or off), and Use
    /// Hyphenation while editing is that text's, its check following
    /// it (#470).
    #[test]
    fn editing_a_text_keeps_its_hyphenation() {
        let mut app = App::headless();
        // The paragraph's own setting, as the layout reads it.
        let hy = |app: &App, id| {
            let s = app.doc().find_shape(id).expect("text").clone();
            app.paragraph_style_of(&s).is_some_and(|(_, p)| p.hyphenate)
        };
        let a = text(&mut app, "una informazione", Some(Size::new(30.0, 40.0)));
        app.select(vec![a]);
        app.toggle_use_hyphenation();
        assert!(hy(&app, a) && !app.text_hyphenation);
        app.begin_text_edit(a);
        assert!(app.text_edit.is_some());
        app.text_insert(" nuova");
        assert!(hy(&app, a), "typing kept it");
        assert!(app.uses_hyphenation(), "the check is the edited text's");
        app.toggle_use_hyphenation();
        assert!(!hy(&app, a) && !app.uses_hyphenation());
        assert!(!app.text_hyphenation, "new text keeps its setting");
        app.text_insert("!");
        assert!(!hy(&app, a));
    }

    fn frame(ctx: &Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, &ui.ctx().clone()));
        out.textures_delta.clear();
    }

    #[test]
    fn every_tab_draws() {
        let mut app = App::headless();
        let id = text(&mut app, "Ths is a apple", None);
        app.select(vec![id]);
        let ctx = crate::theme::ui_context();
        for tab in 0..3 {
            app.dialog = Dialog::WritingTools(WritingToolsState::on_tab(tab));
            frame(&ctx, &mut app);
            frame(&ctx, &mut app);
            assert!(matches!(app.dialog, Dialog::WritingTools(_)));
        }
        app.dialog = Dialog::CustomHyphens(Default::default());
        frame(&ctx, &mut app);
        assert!(matches!(app.dialog, Dialog::CustomHyphens(_)));
    }

    #[test]
    fn spell_options_and_skips_leave_words_out() {
        let mut app = App::headless();
        let id = text(&mut app, "zqxw ZQXW zq2w", None);
        let mut tab = SpellTab::default();
        let all = unknown_words(&app, &[id], &tab);
        app.settings.spelling.ignore_capitals = true;
        app.settings.spelling.ignore_digits = true;
        let fewer = unknown_words(&app, &[id], &tab);
        assert!(all.len() > fewer.len(), "{all:?} {fewer:?}");
        tab.skipped = fewer.iter().map(|(_, w)| w.clone()).collect();
        assert!(unknown_words(&app, &[id], &tab).is_empty());
    }

    /// Options > Text > Spelling > Add corrections to Autocorrect: a
    /// replacement becomes a row, an existing row is replaced, and
    /// nothing comes of an unchanged word or one with spaces.
    #[test]
    fn corrections_go_to_the_autocorrect_table() {
        let mut p = crate::autocorrect::AutocorrectPrefs::default();
        let n = p.replacements.len();
        add_correction(&mut p, "teh", "the");
        assert_eq!(p.replacements.len(), n + 1);
        assert_eq!(p.replacements.last(), Some(&("teh".into(), "the".into())));
        add_correction(&mut p, "teh", "tech");
        assert_eq!(p.replacements.len(), n + 1);
        assert_eq!(p.replacements.last(), Some(&("teh".into(), "tech".into())));
        add_correction(&mut p, "same", "same");
        add_correction(&mut p, "two words", "x");
        add_correction(&mut p, "", "x");
        assert_eq!(p.replacements.len(), n + 1);
        // Typing the word wrong again corrects it.
        let style = crate::autocorrect::QuoteStyle::Curly;
        assert_eq!(
            crate::autocorrect::on_typed("I saw teh ", &p, style).as_deref(),
            Some("I saw tech ")
        );
    }

    #[test]
    fn grammar_findings_skip_the_rules_skipped() {
        let mut app = App::headless();
        let id = text(&mut app, "The the cat sat.", None);
        let found = grammar_findings(&app, &[id], &[]);
        assert!(found
            .iter()
            .any(|(_, f)| f.rule == crate::grammar::Rule::DoubledWord));
        let skipped = grammar_findings(&app, &[id], &[crate::grammar::Rule::DoubledWord]);
        assert!(!skipped
            .iter()
            .any(|(_, f)| f.rule == crate::grammar::Rule::DoubledWord));
    }

    #[test]
    fn hyphenated_words_give_break_points() {
        assert_eq!(parse_hyphenated("TraceDraw", "Trace-Draw"), Some(vec![5]));
        assert_eq!(
            parse_hyphenated("hyphenation", "hy-phen-a-tion"),
            Some(vec![2, 6, 7])
        );
        assert_eq!(parse_hyphenated("word", "wo-rds"), None);
        assert_eq!(parse_hyphenated("word", "word"), None);
    }

    #[test]
    fn custom_hyphens_turn_hyphenation_on_and_break_words() {
        let mut app = App::headless();
        app.settings.custom_hyphens = vec![("en".into(), "TraceDraw".into(), "Trace-Draw".into())];
        install_custom_hyphens(&app);
        assert_eq!(tracedraw_text::hyphen::break_points("tracedraw"), vec![5]);
        assert_eq!(tracedraw_text::hyphen::break_points("TraceDraw"), vec![5]);
        let a = text(
            &mut app,
            "Made with TraceDraw today",
            Some(Size::new(30.0, 40.0)),
        );
        let b = text(&mut app, "Nothing here", Some(Size::new(30.0, 40.0)));
        assert_eq!(apply_custom_hyphens(&mut app, &[a, b]), 1);
        let hy = |app: &App, id| match &app.doc().find_shape(id).expect("text").kind {
            ShapeKind::Text { para, .. } => para.hyphenate,
            _ => false,
        };
        assert!(hy(&app, a) && !hy(&app, b));
        // Pasted text with the word gets it too.
        let c = text(&mut app, "TraceDraw", Some(Size::new(30.0, 40.0)));
        app.select(vec![c]);
        app.copy();
        app.paste();
        let pasted = app.selection[0];
        assert!(hy(&app, pasted));
        app.settings.custom_hyphens.clear();
        install_custom_hyphens(&app);
    }
}
