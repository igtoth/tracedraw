//! Files dropped on the window from the desktop or a file manager (or
//! from the computer onto the page, in a browser). With a drawing open,
//! what File > Import reads goes into it, in the middle of the view
//! (the windowing layer does not say where the pointer let go);
//! otherwise each file opens like File > Open, and a picture without a
//! drawing gets a new one. See `docs/behavior/drop-files.md`.

use crate::app::{App, IMPORT_EXTENSIONS, OPEN_EXTENSIONS};
use std::path::{Path, PathBuf};

/// What a dropped file does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dropped {
    /// Into the open drawing, like File > Import.
    Import,
    /// In a tab of its own, like File > Open.
    Open,
    /// A new drawing, with the file imported into it (pictures).
    NewWithImport,
    /// Neither File > Open nor File > Import reads it.
    Unsupported,
}

impl App {
    /// Files dropped on the window this frame; the open drawing (or none)
    /// at the moment of the drop decides for all of them.
    pub fn take_dropped_files(&mut self, ctx: &egui::Context) {
        let into_drawing = self.drawing_shown();
        crate::files::take_dropped(ctx, self, move |app, path| {
            app.drop_file(&path, into_drawing)
        });
    }

    /// A drawing is open and shown (not the Welcome Screen); without one
    /// the property bar and the toolbox are pale too.
    pub(crate) fn drawing_shown(&self) -> bool {
        self.has_document() && !self.show_welcome
    }

    /// What dropping `path` does, `into_drawing` when a drawing is shown.
    pub fn dropped_kind(&self, path: &Path, into_drawing: bool) -> Dropped {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let importable =
            IMPORT_EXTENSIONS.contains(&ext.as_str()) || self.import_filter_for(&ext).is_some();
        let openable = OPEN_EXTENSIONS.contains(&ext.as_str());
        match (into_drawing, importable, openable) {
            (true, true, _) => Dropped::Import,
            (_, _, true) => Dropped::Open,
            (false, true, false) => Dropped::NewWithImport,
            _ => Dropped::Unsupported,
        }
    }

    /// Open or import one dropped file.
    pub fn drop_file(&mut self, path: &Path, into_drawing: bool) {
        match self.dropped_kind(path, into_drawing) {
            Dropped::Import => {
                let at = self.view_center();
                self.import_path_placed(path, Some(at));
            }
            Dropped::Open => self.open_path(PathBuf::from(path)),
            Dropped::NewWithImport => {
                self.new_document();
                let at = self.page_rect().center();
                self.import_path_placed(path, Some(at));
                self.fit_pending = true;
            }
            Dropped::Unsupported => {
                self.status = crate::i18n::trf(
                    "status.drop_unsupported",
                    &[("p", &path.display().to_string())],
                );
            }
        }
    }

    /// While files are dragged over the window: a frame round it and what
    /// a drop would do.
    pub fn draw_drop_hint(&self, ctx: &egui::Context) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let hint = crate::i18n::tr(if self.drawing_shown() {
            "hint.drop_import"
        } else {
            "hint.drop_open"
        });
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("drop_files_hint"),
        ));
        let screen = ctx.content_rect();
        let accent = crate::theme::Tokens::ACCENT;
        painter.rect(
            screen.shrink(2.0),
            4.0,
            accent.gamma_multiply(0.12),
            egui::Stroke::new(3.0, accent),
            egui::StrokeKind::Inside,
        );
        let font = egui::FontId::proportional(18.0);
        let galley = painter.layout_no_wrap(hint, font, egui::Color32::WHITE);
        let label = egui::Rect::from_center_size(screen.center(), galley.size())
            .expand2(egui::vec2(16.0, 10.0));
        painter.rect_filled(label, 6.0, accent);
        painter.galley(
            label.shrink2(egui::vec2(16.0, 10.0)).min,
            galley,
            egui::Color32::WHITE,
        );
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A file dropped by the windowing layer, as egui hands it over.
    #[derive(Debug)]
    struct File(PathBuf);

    impl egui::DroppedFile for File {
        fn path(&self) -> &Path {
            &self.0
        }

        fn bytes(&self) -> Result<Vec<u8>, String> {
            std::fs::read(&self.0).map_err(|e| e.to_string())
        }
    }

    fn frame(ctx: &egui::Context, app: &mut App, dropped: Vec<PathBuf>, hovered: bool) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            dropped_files: dropped
                .into_iter()
                .map(|p| Arc::new(File(p)) as egui::DroppedFileHandle)
                .collect(),
            hovered_files: if hovered {
                vec![egui::HoveredFile::default()]
            } else {
                Vec::new()
            },
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    fn setup() -> (egui::Context, App) {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for _ in 0..3 {
            frame(&ctx, &mut app, Vec::new(), false);
        }
        app.zoom_to_page();
        frame(&ctx, &mut app, Vec::new(), false);
        (ctx, app)
    }

    fn dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tracedraw-drop-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn svg(dir: &Path) -> PathBuf {
        let path = dir.join("square.svg");
        std::fs::write(
            &path,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200mm" height="200mm" viewBox="0 0 200 200"><rect x="150" y="150" width="20" height="20" fill="red"/></svg>"#,
        )
        .expect("svg");
        path
    }

    fn png(dir: &Path) -> PathBuf {
        let path = dir.join("dot.png");
        image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 120, 215, 255]))
            .save(&path)
            .expect("png");
        path
    }

    fn shape_count(app: &App) -> usize {
        app.doc().all_layers().map(|l| l.shapes.len()).sum()
    }

    fn close(a: tracedraw_core::geometry::Point, b: tracedraw_core::geometry::Point) -> bool {
        (a - b).hypot() < 0.01
    }

    #[test]
    fn what_a_drop_does_by_kind_and_open_drawing() {
        let app = App::headless();
        let kind = |p: &str, drawing| app.dropped_kind(Path::new(p), drawing);
        assert_eq!(kind("a.svg", true), Dropped::Import);
        assert_eq!(kind("a.PSD", true), Dropped::Import);
        assert_eq!(kind("a.png", true), Dropped::Import);
        assert_eq!(kind("a.tdraw", true), Dropped::Open, "not importable");
        assert_eq!(kind("a.svg", false), Dropped::Open);
        assert_eq!(kind("a.cdr", false), Dropped::Open);
        assert_eq!(kind("a.psd", false), Dropped::Open);
        assert_eq!(kind("a.jpg", false), Dropped::NewWithImport);
        assert_eq!(kind("a.zip", true), Dropped::Unsupported);
        assert_eq!(kind("noextension", false), Dropped::Unsupported);
    }

    #[test]
    fn a_file_dropped_on_a_drawing_is_imported_in_the_middle_of_the_view() {
        let (ctx, mut app) = setup();
        let path = svg(&dir("import"));
        let docs = app.docs.len();
        frame(&ctx, &mut app, vec![path], false);
        assert_eq!(app.docs.len(), docs, "no new tab");
        assert_eq!(shape_count(&app), 1);
        assert_eq!(app.selection.len(), 1, "what came is selected");
        let b = app.selection_bounds().expect("bounds");
        assert!(close(b.center(), app.view_center()), "{b:?}");
        // One undo step takes it all back.
        app.undo();
        assert_eq!(shape_count(&app), 0);
    }

    #[test]
    fn without_a_drawing_a_drop_opens_the_file_and_a_picture_gets_a_new_drawing() {
        let (ctx, mut app) = setup();
        let dir = dir("open");
        app.show_welcome = true;
        let docs = app.docs.len();
        frame(&ctx, &mut app, vec![svg(&dir)], false);
        assert_eq!(app.docs.len(), docs + 1, "opened in a tab of its own");
        assert!(!app.show_welcome);
        assert_eq!(shape_count(&app), 1);

        app.show_welcome = true;
        frame(&ctx, &mut app, vec![png(&dir)], false);
        assert_eq!(app.docs.len(), docs + 2, "a new drawing");
        assert!(!app.show_welcome);
        assert_eq!(shape_count(&app), 1);
        let b = app.selection_bounds().expect("bounds");
        assert!(close(b.center(), app.page_rect().center()), "{b:?}");
    }

    #[test]
    fn several_files_dropped_on_the_welcome_screen_open_side_by_side() {
        let (ctx, mut app) = setup();
        let dir = dir("several");
        app.show_welcome = true;
        let docs = app.docs.len();
        frame(&ctx, &mut app, vec![svg(&dir), png(&dir)], false);
        assert_eq!(app.docs.len(), docs + 2, "each file in its own tab");
    }

    #[test]
    fn an_unknown_file_only_says_so_and_dragging_files_over_draws_the_hint() {
        let (ctx, mut app) = setup();
        let path = dir("unknown").join("notes.zip");
        std::fs::write(&path, b"PK").expect("zip");
        frame(&ctx, &mut app, vec![path], true);
        assert_eq!(shape_count(&app), 0);
        assert!(app.status.contains("notes.zip"), "{}", app.status);
        // Files held over the window keep it painting the hint.
        assert!(ctx.has_requested_repaint());
    }
}
