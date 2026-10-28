//! Import PDF: how a PDF's (or AI file's) text is read in, asked when one
//! is opened or imported (#386): as editable text in the fonts installed
//! here, or as curves drawn from the fonts the file embeds, which keeps
//! its look exactly.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::ui::dialogs::{ok_cancel, window};
use crate::ui::tips::RichTip;
use egui::Context;
use std::path::PathBuf;

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfTextState {
    pub path: PathBuf,
    /// Import into the drawing (File > Import) rather than open.
    pub import: bool,
    pub curves: bool,
    /// Keep the choice and stop asking (Options > File Formats).
    pub dont_ask: bool,
}

impl App {
    /// Before a PDF is read: ask how to read its text, when the
    /// preferences say so and it has not been asked for this file yet.
    /// True when the dialog was opened (the caller stops there).
    pub fn ask_pdf_text(&mut self, path: &std::path::Path, import: bool) -> bool {
        if !self.settings.formats.pdf_ask_text || std::mem::take(&mut self.pdf_text_chosen) {
            return false;
        }
        self.dialog = crate::ui::dialogs::Dialog::PdfText(PdfTextState {
            path: path.to_path_buf(),
            import,
            curves: self.settings.formats.pdf_text_as_curves,
            dont_ask: false,
        });
        true
    }

    /// The import options the preferences give.
    pub fn pdf_options(&self) -> tracedraw_io::pdf_import::ImportOptions {
        tracedraw_io::pdf_import::ImportOptions {
            text_as_curves: self.settings.formats.pdf_text_as_curves,
        }
    }
}

/// Text whose font the file does not embed, left as text by the import:
/// as curves too, in the font that stands in for it here.
pub fn remaining_text_to_curves(pages: &mut [tracedraw_io::pdf_import::ImportedPage]) {
    let mut ids = tracedraw_core::id::IdSource::default();
    let mut next = || ids.shape();
    for p in pages {
        for s in &mut p.shapes {
            to_curves(s, &mut next);
        }
    }
}

fn to_curves(s: &mut tracedraw_core::Shape, next: &mut dyn FnMut() -> tracedraw_core::ShapeId) {
    use tracedraw_core::ShapeKind;
    match &mut s.kind {
        ShapeKind::Text { .. } => {
            if let Some(kind) = s.curves_kind(&[], next) {
                s.kind = kind;
            }
        }
        ShapeKind::Group { children } => {
            for c in children {
                to_curves(c, next);
            }
        }
        ShapeKind::ClipFrame { contents, .. } => {
            for c in contents {
                to_curves(c, next);
            }
        }
        _ => {}
    }
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut PdfTextState, close: &mut bool) {
    window(ctx, tr("pdf_import.title")).show(ctx, |ui| {
        ui.set_max_width(420.0);
        let name = st
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        ui.label(trf("pdf_import.prompt", &[("name", &name)]));
        ui.add_space(6.0);
        ui.label(tr("pdf_import.text_as"));
        ui.radio_value(&mut st.curves, false, tr("pdf_import.text"))
            .rich_tip(tr("pdf_import.text_tip"));
        ui.radio_value(&mut st.curves, true, tr("pdf_import.curves"))
            .rich_tip(tr("pdf_import.curves_tip"));
        ui.add_space(4.0);
        ui.checkbox(&mut st.dont_ask, tr("pdf_import.dont_ask"));
        if ok_cancel(ui, close) {
            app.settings.formats.pdf_text_as_curves = st.curves;
            if st.dont_ask {
                app.settings.formats.pdf_ask_text = false;
            }
            app.pdf_text_chosen = true;
            let path = st.path.clone();
            if st.import {
                app.import_pdf(&path);
            } else {
                app.open_path(path);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opening a PDF asks how to read its text once; the answer is kept,
    /// and Do not ask again stops the question.
    #[test]
    fn opening_a_pdf_asks_how_to_read_its_text() {
        let mut app = App::headless();
        app.settings.formats.pdf_ask_text = true;
        let p = std::path::Path::new("x.pdf");
        assert!(app.ask_pdf_text(p, false));
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::PdfText(PdfTextState { import: false, .. })
        ));
        // Answered: the next read goes through without asking.
        app.pdf_text_chosen = true;
        assert!(!app.ask_pdf_text(p, false));
        assert!(app.ask_pdf_text(p, true));
        app.settings.formats.pdf_ask_text = false;
        assert!(!app.ask_pdf_text(p, true));
        app.settings.formats.pdf_text_as_curves = true;
        assert!(app.pdf_options().text_as_curves);
    }
}
