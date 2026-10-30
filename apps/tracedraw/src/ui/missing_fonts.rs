//! The Missing Fonts dialog (#936): after a drawing opens with fonts that
//! are not installed, a row per missing family shows the installed font
//! drawing its text (the automatic choice) and lets the user pick another.
//! The choices either replace the family in the drawing's text for good
//! (one undo step, saved with the drawing) or hold for this session only
//! (the drawing keeps its font names; `FontSystem::set_substitute`).
//! Shown only by the windowed application, never for automated opens
//! (tests, scripts, the MCP server), which keep the plain note. See
//! docs/behavior/missing-fonts.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::ui::dialogs::window;
use egui::Context;
use tracedraw_core::Command;

/// One missing family.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub missing: String,
    /// The installed font drawing its text when the dialog opened.
    pub automatic: Option<String>,
    /// The font chosen to draw it (the automatic one to start with).
    pub chosen: String,
}

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct MissingFontsState {
    /// The drawing's file name.
    pub name: String,
    /// Objects of the file that could not be read, said after this dialog.
    pub unread: usize,
    pub rows: Vec<Row>,
    /// Replace the families in the drawing (saved with it) rather than
    /// for this session only.
    pub keep: bool,
    /// Do not ask again (Options > Global > Warnings).
    pub dont_show: bool,
    /// The installed families typed into the list's filter.
    pub filter: String,
}

impl MissingFontsState {
    /// A row per missing family, each with the font drawing it now.
    pub fn new(name: String, unread: usize, missing: &[String]) -> Self {
        let fonts = tracedraw_text::fonts();
        let rows = missing
            .iter()
            .map(|m| {
                let automatic = fonts.substitute(m);
                Row {
                    missing: m.clone(),
                    chosen: automatic.clone().unwrap_or_default(),
                    automatic,
                }
            })
            .collect();
        MissingFontsState {
            name,
            unread,
            rows,
            keep: false,
            dont_show: false,
            filter: String::new(),
        }
    }

    /// The commands that replace the families for good: one per row with
    /// a font chosen.
    pub fn commands(&self) -> Vec<Command> {
        self.rows
            .iter()
            .filter(|r| !r.chosen.is_empty() && r.chosen != r.missing)
            .map(|r| Command::ReplaceFontFamily {
                from: r.missing.clone(),
                to: r.chosen.clone(),
            })
            .collect()
    }
}

impl App {
    /// OK: the fonts chosen draw the missing families, replaced in the
    /// drawing (one undo step) or for this session.
    pub fn apply_font_choices(&mut self, st: &MissingFontsState) {
        let fonts = tracedraw_text::fonts();
        if st.keep {
            let cmds = st.commands();
            if !cmds.is_empty() {
                if let Err(e) = self.engine.run_batch("Replace Font", &cmds) {
                    self.status = e.to_string();
                    return;
                }
            }
            // The families are gone from the drawing: nothing to stand in.
            for r in &st.rows {
                fonts.set_substitute(&r.missing, None);
            }
        } else {
            for r in &st.rows {
                let pick = (Some(&r.chosen) != r.automatic.as_ref() && !r.chosen.is_empty())
                    .then_some(r.chosen.as_str());
                fonts.set_substitute(&r.missing, pick);
            }
        }
        if st.dont_show {
            // The dialog's own choice: the plain note stays as it is.
            self.settings.warnings.ask_missing_fonts = false;
            self.settings.save();
        }
        self.check_missing_fonts();
        self.status = if self.missing_fonts.is_empty() {
            tr("missing_fonts.replaced")
        } else {
            trf(
                "status.missing_fonts",
                &[("f", &crate::ops2::with_stand_ins(&self.missing_fonts))],
            )
        };
    }
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut MissingFontsState, close: &mut bool) {
    let mut help = false;
    let mut ok = false;
    window(ctx, tr("missing_fonts.title")).show(ctx, |ui| {
        ui.set_width(520.0);
        ui.label(trf("missing_fonts.intro", &[("name", &st.name)]));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(tr("missing_fonts.filter"));
            ui.add(egui::TextEdit::singleline(&mut st.filter).desired_width(200.0));
        });
        ui.add_space(4.0);
        let filter = st.filter.trim().to_lowercase();
        let families = &app.font_families;
        egui::ScrollArea::vertical()
            .max_height(260.0)
            .show(ui, |ui| {
                egui::Grid::new("missing_fonts_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(tr("missing_fonts.missing"));
                        ui.strong(tr("missing_fonts.substitute"));
                        ui.end_row();
                        for (i, row) in st.rows.iter_mut().enumerate() {
                            ui.label(&row.missing);
                            let shown = if row.chosen.is_empty() {
                                tr("missing_fonts.none")
                            } else {
                                row.chosen.clone()
                            };
                            egui::ComboBox::from_id_salt(("missing_font", i))
                                .selected_text(shown)
                                .width(260.0)
                                .height(300.0)
                                .show_ui(ui, |ui| {
                                    if let Some(auto) = &row.automatic {
                                        let label = trf("missing_fonts.automatic", &[("f", auto)]);
                                        ui.selectable_value(&mut row.chosen, auto.clone(), label);
                                        ui.separator();
                                    }
                                    for f in families.iter().filter(|f| {
                                        filter.is_empty() || f.to_lowercase().contains(&filter)
                                    }) {
                                        ui.selectable_value(&mut row.chosen, f.clone(), f);
                                    }
                                });
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(6.0);
        ui.radio_value(&mut st.keep, false, tr("missing_fonts.session"));
        ui.radio_value(&mut st.keep, true, tr("missing_fonts.keep"));
        ui.add_space(4.0);
        ui.checkbox(&mut st.dont_show, tr("notes.dont_show"));
        let (o, h) = crate::ui::dialogs::help_ok_cancel(ui, close);
        ok = o;
        help = h;
    });
    if ok {
        app.apply_font_choices(st);
    } else if *close && st.dont_show {
        // Closed without OK: the choices are dropped, the box is kept.
        app.settings.warnings.ask_missing_fonts = false;
        app.settings.save();
    }
    if help {
        app.open_dialog_help();
    }
    if *close && st.unread > 0 {
        // The note about the objects that could not be read follows.
        // A closing dialog's next one is kept (see `dialogs::show`).
        app.dialog = crate::ui::dialogs::Dialog::OpenNotes(crate::ui::dialogs::OpenNotes {
            name: st.name.clone(),
            unread: st.unread,
            fonts: Vec::new(),
            dont_show: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;
    use egui::{vec2, Pos2, Rect};
    use tracedraw_core::document::ShapeKind;

    fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, ui.ctx()));
        out.textures_delta.clear();
    }

    fn enter() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }
    }

    /// A drawing whose text is set in a family that is not installed.
    fn drawing() -> (App, String) {
        tracedraw_text::install();
        let mut app = App::headless();
        let family = app.font_families.first().cloned().unwrap_or_default();
        app.new_shape(ShapeKind::Text {
            spans: vec![tracedraw_core::TextSpan::new(
                "Hello",
                "Nowhere Grotesk 936",
                12.0,
            )],
            origin: tracedraw_core::geometry::Point::new(10.0, 10.0),
            frame: None,
            align: Default::default(),
            para: Default::default(),
            on_path: None,
        })
        .expect("text");
        app.check_missing_fonts();
        (app, family)
    }

    /// Automated opens never ask: the headless app keeps the plain note.
    /// The windowed app asks, a row per missing family with the automatic
    /// choice; replacing for good changes the drawing in one undo step.
    #[test]
    fn missing_fonts_are_replaced_in_the_drawing_when_asked() {
        let (mut app, family) = drawing();
        if family.is_empty() {
            return;
        }
        app.show_open_notes("a.tdraw".into(), 0);
        assert!(
            matches!(app.dialog, Dialog::OpenNotes(_)),
            "headless opens are not asked"
        );
        app.dialog = Dialog::None;
        app.ask_missing_fonts = true;
        app.show_open_notes("a.tdraw".into(), 2);
        let Dialog::MissingFonts(st) = &mut app.dialog else {
            panic!("asked");
        };
        assert_eq!(st.rows.len(), 1);
        assert_eq!(st.rows[0].missing, "Nowhere Grotesk 936");
        assert_eq!(
            st.rows[0].automatic,
            tracedraw_text::fonts().substitute("Nowhere Grotesk 936")
        );
        st.rows[0].chosen = family.clone();
        st.keep = true;
        let ctx = crate::theme::ui_context();
        frame(&mut app, &ctx, Vec::new());
        frame(&mut app, &ctx, Vec::new());
        frame(&mut app, &ctx, vec![enter()]);
        // The unread objects' note follows.
        frame(&mut app, &ctx, Vec::new());
        assert!(matches!(&app.dialog, Dialog::OpenNotes(n) if n.unread == 2 && n.fonts.is_empty()));
        assert_eq!(app.doc().font_families_used(), vec![family.clone()]);
        assert!(app.missing_fonts.is_empty());
        assert!(app.engine.is_dirty());
        app.undo();
        assert_eq!(
            app.doc().font_families_used(),
            vec!["Nowhere Grotesk 936".to_string()]
        );
    }

    /// For this session only: the drawing keeps its font name, the font
    /// chosen draws it; Cancel changes nothing.
    #[test]
    fn a_font_chosen_for_the_session_draws_the_text_and_the_drawing_keeps_its_font() {
        let (mut app, family) = drawing();
        let Some(other) = app
            .font_families
            .iter()
            .find(|f| {
                Some(f.as_str())
                    != tracedraw_text::fonts()
                        .substitute("Nowhere Grotesk 936")
                        .as_deref()
            })
            .cloned()
        else {
            return;
        };
        let _ = family;
        app.ask_missing_fonts = true;
        // Closed without OK: nothing changes.
        app.show_open_notes("b.tdraw".into(), 0);
        let ctx = crate::theme::ui_context();
        frame(&mut app, &ctx, Vec::new());
        app.dialog = Dialog::None;
        assert!(tracedraw_text::fonts()
            .chosen_substitute("Nowhere Grotesk 936")
            .is_none());
        // Chosen for the session.
        app.show_open_notes("b.tdraw".into(), 0);
        let Dialog::MissingFonts(st) = &mut app.dialog else {
            panic!("asked");
        };
        st.rows[0].chosen = other.clone();
        assert!(!st.keep, "this session is the default");
        let st = st.clone();
        app.apply_font_choices(&st);
        assert_eq!(
            tracedraw_text::fonts().chosen_substitute("Nowhere Grotesk 936"),
            Some(other.clone())
        );
        assert_eq!(
            app.doc().font_families_used(),
            vec!["Nowhere Grotesk 936".to_string()]
        );
        assert_eq!(app.missing_fonts, vec!["Nowhere Grotesk 936".to_string()]);
        assert!(app.status.contains(&other), "{}", app.status);
        tracedraw_text::fonts().set_substitute("Nowhere Grotesk 936", None);
    }

    /// The dialog has a choice of its own in Options > Global > Warnings:
    /// "Do not show this message again" ticked on the plain note before
    /// the dialog existed left the dialog unseen, the status line alone
    /// naming the fonts (#1243). Without the dialog, that line opens the
    /// Warnings page.
    #[test]
    fn the_dialog_asks_although_the_plain_note_was_turned_off() {
        let (mut app, family) = drawing();
        if family.is_empty() {
            return;
        }
        app.ask_missing_fonts = true;
        app.settings.warnings.missing_fonts = false;
        app.show_open_notes("old.cdr".into(), 0);
        assert!(matches!(app.dialog, Dialog::MissingFonts(_)), "asked");
        assert!(app.status_opens_options.is_none());
        // The dialog's box turns the dialog off, not the note.
        let Dialog::MissingFonts(st) = &mut app.dialog else {
            panic!("asked");
        };
        st.dont_show = true;
        let st = st.clone();
        app.apply_font_choices(&st);
        assert!(!app.settings.warnings.ask_missing_fonts);
        assert!(!app.settings.warnings.missing_fonts);
        // Both off: nothing but the status line, which leads to the page.
        app.dialog = Dialog::None;
        app.show_open_notes("old.cdr".into(), 0);
        assert!(matches!(app.dialog, Dialog::None));
        assert_eq!(
            app.status_opens_options,
            Some((
                app.status.clone(),
                crate::ui::dialogs::OptionsPage::Warnings
            ))
        );
        // The note on and the dialog off: the note, as before the dialog.
        app.settings.warnings.missing_fonts = true;
        app.show_open_notes("old.cdr".into(), 0);
        assert!(matches!(&app.dialog, Dialog::OpenNotes(n) if n.fonts.len() == 1));
        app.settings.warnings = Default::default();
    }
}
