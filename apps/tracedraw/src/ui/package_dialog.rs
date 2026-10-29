//! File > Package: the Package Drawing dialog (#852). It chooses the
//! folder (a folder named after the drawing, beside its file, to start
//! with), the drawing's name in it and whether a PDF goes along; OK
//! writes the package (`App::package_copy`) and the status line says what
//! went in. See docs/behavior/package.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::ui::dialogs::window;
use egui::{Color32, Context, RichText};
use std::path::{Path, PathBuf};

const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageState {
    /// The package folder, as typed or chosen.
    pub folder: String,
    /// The drawing's name in the folder (without `.tdraw`).
    pub name: String,
    /// A PDF of the drawing goes along.
    pub pdf: bool,
    /// Why nothing was written yet.
    pub note: Option<String>,
}

impl PackageState {
    /// A folder named after the drawing, in the folder of its file (else
    /// the last one used, else the home folder).
    pub fn new(app: &App) -> Self {
        let name = app.document_name();
        let dir = app
            .file
            .as_ref()
            .and_then(|f| f.parent())
            .map(Path::to_path_buf)
            .or_else(|| app.settings.last_dir.clone())
            .or_else(crate::files::home_dir)
            .unwrap_or_default();
        PackageState {
            folder: dir.join(&name).display().to_string(),
            name,
            pdf: false,
            note: None,
        }
    }

    /// Why the package cannot be written (an i18n key).
    pub fn problem(&self) -> Option<&'static str> {
        if self.folder.trim().is_empty() {
            return Some("package.need_folder");
        }
        let name = self.stem();
        if name.is_empty() || name.contains(['/', '\\']) {
            return Some("dialog.need_file_name");
        }
        None
    }

    /// The name without a `.tdraw` the user may have typed.
    pub fn stem(&self) -> String {
        let n = self.name.trim();
        n.strip_suffix(".tdraw").unwrap_or(n).trim().to_string()
    }

    /// Write the package; true when it was written (the dialog closes).
    pub fn run(&mut self, app: &mut App) -> bool {
        if let Some(p) = self.problem() {
            self.note = Some(tr(p));
            return false;
        }
        let folder = PathBuf::from(self.folder.trim());
        match app.package_copy(&folder, &self.stem(), self.pdf) {
            Ok(r) => {
                app.status = crate::package::status(&r);
                if let Some(parent) = folder.parent() {
                    app.settings.last_dir = Some(parent.to_path_buf());
                }
                true
            }
            Err(e) => {
                self.note = Some(trf("status.package_failed", &[("e", &e)]));
                false
            }
        }
    }
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut PackageState, close: &mut bool) {
    let mut help = false;
    window(ctx, tr("package.title")).show(ctx, |ui| {
        ui.set_width(520.0);
        egui::Grid::new("package_grid")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("package.folder"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut st.folder).desired_width(330.0));
                    if ui.button(tr("publish.browse")).clicked() {
                        if let Some(dir) = crate::files::pick_folder() {
                            st.folder = dir.join(st.stem()).display().to_string();
                        }
                    }
                });
                ui.end_row();
                ui.label(tr("dialog.file_name"));
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(240.0));
                    ui.label(".tdraw");
                });
                ui.end_row();
            });
        ui.add_space(4.0);
        ui.checkbox(&mut st.pdf, tr("package.pdf"));
        ui.add_space(4.0);
        ui.label(RichText::new(tr("package.note")).weak());
        let note = st.note.clone().unwrap_or_else(|| " ".into());
        ui.label(RichText::new(note).color(ERROR));
        let (ok, h) = crate::ui::dialogs::help_ok_cancel(ui, close);
        help = h;
        if ok && !st.run(app) {
            *close = false;
        }
    });
    if help {
        app.open_dialog_help();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{vec2, Pos2, Rect};

    /// The dialog starts at a folder named after the drawing, checks the
    /// name, draws, and OK (Enter) writes the package and closes.
    #[test]
    fn the_dialog_writes_the_package_into_the_folder_chosen() {
        let dir = std::env::temp_dir().join(format!("tracedraw-pkgdlg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::headless();
        app.show_welcome = false;
        app.settings.last_dir = Some(dir.clone());
        app.package_dialog();
        let crate::ui::dialogs::Dialog::Package(st) = &mut app.dialog else {
            panic!("the dialog is open");
        };
        let name = st.name.clone();
        assert_eq!(PathBuf::from(&st.folder), dir.join(&name));
        st.name = " ".into();
        assert_eq!(st.problem(), Some("dialog.need_file_name"));
        st.name = "Flyer.tdraw".into();
        assert_eq!(st.stem(), "Flyer");
        assert!(st.problem().is_none());
        // Drawn, then OK by Enter.
        let ctx = crate::theme::ui_context();
        let frame = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, ui.ctx()));
            out.textures_delta.clear();
        };
        frame(&mut app, Vec::new());
        frame(&mut app, Vec::new());
        assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::Package(_)));
        frame(
            &mut app,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
        assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::None));
        let folder = dir.join(&name);
        assert!(folder.join("Flyer.tdraw").is_file(), "{}", app.status);
        assert!(std::fs::read_dir(&folder)
            .expect("folder")
            .flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "txt")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
