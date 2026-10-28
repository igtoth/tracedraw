//! Files dropped on the window from the desktop or a file manager (or
//! from the computer onto the page, in a browser). Let go over the
//! drawing window of an open drawing, what File > Import reads goes into
//! it, in the middle of the view; let go over the title, menus, toolbars
//! or document tabs, the file opens in a tab of its own like File >
//! Open (#1142). Without a drawing each file opens like File > Open,
//! and a picture gets a new drawing. See `docs/behavior/drop-files.md`.

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

/// Where files are let go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropOn {
    /// The drawing window of the drawing shown.
    Drawing,
    /// Elsewhere on the window while a drawing is shown: its title,
    /// menus, toolbars, document tabs, dockers.
    Frame,
    /// A drawing is shown but the windowing layer does not say where
    /// the pointer is (Linux, macOS).
    Unknown,
    /// No drawing is shown (Welcome Screen, or no drawing open).
    NoDrawing,
}

impl App {
    /// Files dropped on the window this frame; where they were let go
    /// decides for all of them.
    pub fn take_dropped_files(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.raw.dropped_files.is_empty()) {
            return;
        }
        let on = self.drop_on(drop_pointer(ctx));
        crate::files::take_dropped(ctx, self, move |app, path| app.drop_file(&path, on));
    }

    /// A drawing is open and shown (not the Welcome Screen); without one
    /// the property bar and the toolbox are pale too.
    pub(crate) fn drawing_shown(&self) -> bool {
        self.has_document() && !self.show_welcome
    }

    /// Where files let go with the pointer at `pointer` (window points,
    /// when known) land.
    pub fn drop_on(&self, pointer: Option<egui::Pos2>) -> DropOn {
        if !self.drawing_shown() {
            return DropOn::NoDrawing;
        }
        match pointer {
            Some(p) if self.canvas_rect.contains(p) => DropOn::Drawing,
            Some(_) => DropOn::Frame,
            None => DropOn::Unknown,
        }
    }

    /// What dropping `path` on `on` does.
    pub fn dropped_kind(&self, path: &Path, on: DropOn) -> Dropped {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let importable =
            IMPORT_EXTENSIONS.contains(&ext.as_str()) || self.import_filter_for(&ext).is_some();
        let openable = OPEN_EXTENSIONS.contains(&ext.as_str());
        match on {
            DropOn::Drawing if importable => Dropped::Import,
            // Where the pointer is not known, a drawing File > Open reads
            // (.cdr, .svg, .tdraw...) opens in its own tab like a .tdraw
            // always did, and only pictures go into the drawing (#1142).
            DropOn::Unknown if importable && !openable => Dropped::Import,
            _ if openable => Dropped::Open,
            DropOn::Frame | DropOn::NoDrawing if importable => Dropped::NewWithImport,
            _ => Dropped::Unsupported,
        }
    }

    /// Open or import one dropped file.
    pub fn drop_file(&mut self, path: &Path, on: DropOn) {
        match self.dropped_kind(path, on) {
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

    /// While files are dragged over the window: a frame round where they
    /// would go (the drawing window, or the whole window) and what a drop
    /// there would do.
    pub fn draw_drop_hint(&self, ctx: &egui::Context) {
        let hovered: Vec<Option<PathBuf>> =
            ctx.input(|i| i.raw.hovered_files.iter().map(|f| f.path.clone()).collect());
        if hovered.is_empty() {
            return;
        }
        let on = self.drop_on(drop_pointer(ctx));
        // Without paths (some browsers) the hint follows the place alone.
        let imports = match on {
            DropOn::Drawing => true,
            DropOn::Unknown => hovered.iter().all(|p| {
                p.as_deref()
                    .is_some_and(|p| self.dropped_kind(p, on) == Dropped::Import)
            }),
            DropOn::Frame | DropOn::NoDrawing => false,
        };
        let hint = crate::i18n::tr(if imports {
            "hint.drop_import"
        } else {
            "hint.drop_open"
        });
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("drop_files_hint"),
        ));
        let area = if on == DropOn::Drawing {
            self.canvas_rect
        } else {
            ctx.content_rect()
        };
        let accent = crate::theme::Tokens::ACCENT;
        painter.rect(
            area.shrink(2.0),
            4.0,
            accent.gamma_multiply(0.12),
            egui::Stroke::new(3.0, accent),
            egui::StrokeKind::Inside,
        );
        let font = egui::FontId::proportional(18.0);
        let galley = painter.layout_no_wrap(hint, font, egui::Color32::WHITE);
        let label = egui::Rect::from_center_size(area.center(), galley.size())
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

/// The pointer over the window, in points, while files are dragged over
/// it or let go. On Windows the system drag sends the window no pointer
/// moves, so the cursor is asked for; elsewhere it is what egui last
/// saw, which is nothing during a desktop drag on Linux and macOS.
fn drop_pointer(ctx: &egui::Context) -> Option<egui::Pos2> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
        let inner = ctx.input(|i| i.viewport().inner_rect);
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: GetCursorPos writes the cursor's screen position, in
        // physical pixels, into `p`.
        if let Some(inner) = inner {
            if unsafe { GetCursorPos(&mut p) } != 0 {
                let ppp = ctx.pixels_per_point();
                let at = egui::pos2(p.x as f32 / ppp, p.y as f32 / ppp);
                return Some(at - inner.min.to_vec2());
            }
        }
    }
    ctx.input(|i| i.pointer.hover_pos())
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
        frame_at(ctx, app, dropped, hovered, None);
    }

    /// A frame with the pointer at `pointer` (none: not known, as in a
    /// desktop drag on Linux).
    fn frame_at(
        ctx: &egui::Context,
        app: &mut App,
        dropped: Vec<PathBuf>,
        hovered: bool,
        pointer: Option<egui::Pos2>,
    ) {
        let input = egui::RawInput {
            events: pointer
                .map(|p| vec![egui::Event::PointerMoved(p)])
                .unwrap_or_default(),
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
    fn what_a_drop_does_by_kind_and_place() {
        use DropOn::*;
        let app = App::headless();
        let kind = |p: &str, on| app.dropped_kind(Path::new(p), on);
        assert_eq!(kind("a.svg", Drawing), Dropped::Import);
        assert_eq!(kind("a.cdr", Drawing), Dropped::Import);
        assert_eq!(kind("a.PSD", Drawing), Dropped::Import);
        assert_eq!(kind("a.png", Drawing), Dropped::Import);
        assert_eq!(kind("a.tdraw", Drawing), Dropped::Open, "not importable");
        // On the title, menus or tabs a drawing opens in its own tab and
        // a picture gets a drawing of its own (#1142).
        assert_eq!(kind("a.cdr", Frame), Dropped::Open);
        assert_eq!(kind("a.svg", Frame), Dropped::Open);
        assert_eq!(kind("a.tdraw", Frame), Dropped::Open);
        assert_eq!(kind("a.png", Frame), Dropped::NewWithImport);
        // Where the pointer is not known, drawings open like a .tdraw
        // and pictures go into the drawing.
        assert_eq!(kind("a.cdr", Unknown), Dropped::Open);
        assert_eq!(kind("a.tdraw", Unknown), Dropped::Open);
        assert_eq!(kind("a.png", Unknown), Dropped::Import);
        assert_eq!(kind("a.svg", NoDrawing), Dropped::Open);
        assert_eq!(kind("a.cdr", NoDrawing), Dropped::Open);
        assert_eq!(kind("a.psd", NoDrawing), Dropped::Open);
        assert_eq!(kind("a.jpg", NoDrawing), Dropped::NewWithImport);
        assert_eq!(kind("a.zip", Drawing), Dropped::Unsupported);
        assert_eq!(kind("a.zip", Frame), Dropped::Unsupported);
        assert_eq!(kind("noextension", NoDrawing), Dropped::Unsupported);
    }

    #[test]
    fn the_pointer_decides_between_the_drawing_window_and_the_rest() {
        let (_ctx, mut app) = setup();
        let inside = app.canvas_rect.center();
        assert_eq!(app.drop_on(Some(inside)), DropOn::Drawing);
        assert_eq!(app.drop_on(Some(egui::pos2(5.0, 5.0))), DropOn::Frame);
        assert_eq!(app.drop_on(None), DropOn::Unknown);
        app.show_welcome = true;
        assert_eq!(app.drop_on(Some(inside)), DropOn::NoDrawing);
    }

    #[test]
    fn a_drawing_dropped_on_the_menus_opens_in_a_tab_of_its_own() {
        let (ctx, mut app) = setup();
        let dir = dir("frame");
        let docs = app.docs.len();
        frame_at(
            &ctx,
            &mut app,
            vec![svg(&dir)],
            false,
            Some(egui::pos2(5.0, 5.0)),
        );
        assert_eq!(app.docs.len(), docs + 1, "opened in a tab of its own");
        assert_eq!(shape_count(&app), 1);
        // A picture there gets a drawing of its own too.
        frame_at(
            &ctx,
            &mut app,
            vec![png(&dir)],
            false,
            Some(egui::pos2(5.0, 5.0)),
        );
        assert_eq!(app.docs.len(), docs + 2, "a new drawing");
        assert_eq!(shape_count(&app), 1);
    }

    #[test]
    fn where_the_pointer_is_unknown_a_drawing_opens_and_a_picture_is_imported() {
        let (ctx, mut app) = setup();
        let dir = dir("unknown-place");
        let docs = app.docs.len();
        frame(&ctx, &mut app, vec![png(&dir)], false);
        assert_eq!(app.docs.len(), docs, "imported, no new tab");
        assert_eq!(shape_count(&app), 1);
        frame(&ctx, &mut app, vec![svg(&dir)], false);
        assert_eq!(app.docs.len(), docs + 1, "opened in a tab of its own");
    }

    #[test]
    fn a_file_dropped_on_a_drawing_is_imported_in_the_middle_of_the_view() {
        let (ctx, mut app) = setup();
        let path = svg(&dir("import"));
        let docs = app.docs.len();
        let inside = app.canvas_rect.center();
        frame_at(&ctx, &mut app, vec![path], false, Some(inside));
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
