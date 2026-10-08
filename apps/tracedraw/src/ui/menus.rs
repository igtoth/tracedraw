//! The menu bar, with the menu structure. Items without an
//! implementation are shown disabled so the layout is complete from day one.

use crate::app::{App, DockerTab};
use crate::tools::Tool;
use egui::Ui;

fn item(ui: &mut Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    let text = if shortcut.is_empty() {
        label.to_string()
    } else {
        format!("{label:<28}{shortcut}")
    };
    let r = ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(text).monospace().size(12.0)).frame(false),
    );
    if r.clicked() {
        ui.close();
        true
    } else {
        false
    }
}

fn todo(ui: &mut Ui, label: &str, shortcut: &str) {
    let _ = item(ui, label, shortcut, false);
}

pub fn menu_bar(app: &mut App, ui: &mut Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if item(ui, "New", "Ctrl+N", true) {
                app.new_document();
            }
            todo(ui, "New From Template...", "");
            if item(ui, "Open...", "Ctrl+O", true) {
                app.open_dialog();
            }
            todo(ui, "Open Recent", "");
            ui.separator();
            if item(ui, "Save", "Ctrl+S", true) {
                app.save(false);
            }
            if item(ui, "Save As...", "Ctrl+Shift+S", true) {
                app.save(true);
            }
            todo(ui, "Save As Template...", "");
            todo(ui, "Revert", "");
            ui.separator();
            if item(ui, "Import...", "Ctrl+I", true) {
                app.import();
            }
            if item(ui, "Import Bitmap...", "", true) {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter(
                        "Images",
                        &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"],
                    )
                    .pick_file()
                {
                    app.import_bitmap(&p);
                }
            }
            if item(ui, "Export...", "Ctrl+E", true) {
                app.export();
            }
            if item(ui, "Export PDF...", "", true) {
                app.export_pdf();
            }
            todo(ui, "Export For", "");
            todo(ui, "Publish to PDF...", "");
            ui.separator();
            todo(ui, "Print...", "Ctrl+P");
            todo(ui, "Print Preview...", "");
            ui.separator();
            todo(ui, "Document Properties...", "");
            ui.separator();
            if item(ui, "Exit", "Alt+F4", true) {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button("Edit", |ui| {
            let undo = app
                .engine
                .undo_label()
                .map(|l| format!("Undo {l}"))
                .unwrap_or_else(|| "Undo".into());
            let redo = app
                .engine
                .redo_label()
                .map(|l| format!("Redo {l}"))
                .unwrap_or_else(|| "Redo".into());
            if item(ui, &undo, "Ctrl+Z", app.engine.undo_label().is_some()) {
                app.undo();
            }
            if item(ui, &redo, "Ctrl+Shift+Z", app.engine.redo_label().is_some()) {
                app.redo();
            }
            todo(ui, "Repeat", "Ctrl+R");
            ui.separator();
            let has = !app.selection.is_empty();
            if item(ui, "Cut", "Ctrl+X", has) {
                app.cut();
            }
            if item(ui, "Copy", "Ctrl+C", has) {
                app.copy();
            }
            if item(ui, "Paste", "Ctrl+V", app.clipboard.is_some()) {
                app.paste();
            }
            todo(ui, "Paste Special...", "");
            if item(ui, "Delete", "Delete", has) {
                app.delete_selection();
            }
            if item(ui, "Duplicate", "Ctrl+D", has) {
                app.duplicate();
            }
            todo(ui, "Clone", "");
            todo(ui, "Step and Repeat...", "Ctrl+Shift+D");
            ui.separator();
            if item(ui, "Select All", "Ctrl+A", true) {
                app.select_all();
            }
            todo(ui, "Find and Replace...", "");
        });
        ui.menu_button("View", |ui| {
            todo(ui, "Simple Wireframe", "");
            if ui.radio(app.wireframe, "Wireframe").clicked() {
                app.wireframe = true;
            }
            todo(ui, "Draft", "");
            if ui.radio(!app.wireframe, "Normal").clicked() {
                app.wireframe = false;
            }
            todo(ui, "Enhanced", "");
            todo(ui, "Pixels", "");
            ui.separator();
            todo(ui, "Full-Screen Preview", "F9");
            ui.separator();
            if item(ui, "Zoom To Page", "Shift+F4", true) {
                app.zoom_to_page();
            }
            if item(ui, "Zoom To Fit", "F4", true) {
                app.zoom_to_fit();
            }
            if item(
                ui,
                "Zoom To Selected",
                "Shift+F2",
                !app.selection.is_empty(),
            ) {
                app.zoom_to_selection();
            }
            ui.separator();
            let mut rulers = app.show_rulers;
            if ui.checkbox(&mut rulers, "Rulers").changed() {
                app.show_rulers = rulers;
            }
            let mut grid = app.show_grid;
            if ui.checkbox(&mut grid, "Grid").changed() {
                app.show_grid = grid;
            }
            let mut guides = app.show_guides;
            if ui.checkbox(&mut guides, "Guidelines").changed() {
                app.show_guides = guides;
            }
            todo(ui, "Page", "");
            ui.separator();
            ui.menu_button("Snap To", |ui| {
                ui.checkbox(&mut app.snap.grid, "Document Grid");
                ui.checkbox(&mut app.snap.guides, "Guidelines");
                ui.checkbox(&mut app.snap.objects, "Objects");
                ui.checkbox(&mut app.snap.page, "Page");
            });
        });
        ui.menu_button("Layout", |ui| {
            if item(ui, "Insert Page...", "", true) {
                app.add_page();
            }
            todo(ui, "Duplicate Page...", "");
            todo(ui, "Rename Page...", "");
            if item(ui, "Delete Page...", "", app.doc().pages.len() > 1) {
                app.delete_page();
            }
            todo(ui, "Go To Page...", "");
            ui.separator();
            ui.menu_button("Page Size", |ui| {
                use tracedraw_core::document::paper;
                for (name, size) in [
                    ("A4", paper::A4),
                    ("A3", paper::A3),
                    ("Letter", paper::LETTER),
                ] {
                    if item(ui, name, "", true) {
                        let page = app.page;
                        app.run(tracedraw_core::Command::ResizePage { page, size });
                        app.fit_pending = true;
                    }
                }
            });
            if item(ui, "Switch Page Orientation", "", true) {
                let s = app.page_size();
                let page = app.page;
                app.run(tracedraw_core::Command::ResizePage {
                    page,
                    size: tracedraw_core::geometry::Size::new(s.height, s.width),
                });
                app.fit_pending = true;
            }
            todo(ui, "Page Background...", "");
            todo(ui, "Page Layout...", "");
        });
        ui.menu_button("Object", |ui| {
            let has = !app.selection.is_empty();
            todo(ui, "Insert", "");
            todo(ui, "ClipFrame", "");
            todo(ui, "Symbol", "");
            ui.separator();
            ui.menu_button("Transformations", |ui| {
                for (name, key, tab) in [
                    ("Position", "Alt+F7", crate::app::TransformTab::Position),
                    ("Rotate", "Alt+F8", crate::app::TransformTab::Rotate),
                    (
                        "Scale and Mirror",
                        "Alt+F9",
                        crate::app::TransformTab::Scale,
                    ),
                    ("Size", "Alt+F10", crate::app::TransformTab::Size),
                    ("Skew", "", crate::app::TransformTab::Skew),
                ] {
                    if item(ui, name, key, true) {
                        app.show_dockers = true;
                        app.docker_tab = DockerTab::Transformations;
                        app.transform_tab = tab;
                    }
                }
                ui.separator();
                if item(ui, "Clear Transformations", "", has) {
                    app.clear_transformations();
                }
            });
            ui.menu_button("Align and Distribute", |ui| {
                use crate::ops::{Align, Distribute};
                for (name, key, a) in [
                    ("Align Left", "L", Align::Left),
                    ("Align Right", "R", Align::Right),
                    ("Align Top", "T", Align::Top),
                    ("Align Bottom", "B", Align::Bottom),
                    ("Align Centers Horizontally", "E", Align::CenterH),
                    ("Align Centers Vertically", "C", Align::CenterV),
                    ("Center to Page", "P", Align::CenterPage),
                ] {
                    if item(ui, name, key, has) {
                        app.align(a);
                    }
                }
                ui.separator();
                let three = app.selection.len() >= 3;
                for (name, d) in [
                    ("Distribute Centers Horizontally", Distribute::CentersH),
                    ("Distribute Centers Vertically", Distribute::CentersV),
                    ("Distribute Spacing Horizontally", Distribute::SpacingH),
                    ("Distribute Spacing Vertically", Distribute::SpacingV),
                ] {
                    if item(ui, name, "", three) {
                        app.distribute(d);
                    }
                }
            });
            ui.menu_button("Order", |ui| {
                if item(ui, "To Front of Page", "Ctrl+Home", has) {
                    app.order(0);
                }
                if item(ui, "To Back of Page", "Ctrl+End", has) {
                    app.order(3);
                }
                if item(ui, "To Front of Layer", "Shift+PgUp", has) {
                    app.order(0);
                }
                if item(ui, "To Back of Layer", "Shift+PgDn", has) {
                    app.order(3);
                }
                if item(ui, "Forward One", "Ctrl+PgUp", has) {
                    app.order(1);
                }
                if item(ui, "Back One", "Ctrl+PgDn", has) {
                    app.order(2);
                }
                todo(ui, "In Front Of...", "");
                todo(ui, "Behind...", "");
                todo(ui, "Reverse Order", "");
            });
            ui.separator();
            if item(ui, "Group", "Ctrl+G", app.selection.len() > 1) {
                app.group_selection();
            }
            if item(ui, "Ungroup", "Ctrl+U", has) {
                app.ungroup_selection();
            }
            todo(ui, "Ungroup All", "");
            ui.separator();
            if item(ui, "Combine", "Ctrl+L", app.selection.len() > 1) {
                app.combine();
            }
            if item(ui, "Break Apart", "Ctrl+K", has) {
                app.break_apart();
            }
            ui.separator();
            if item(ui, "Lock Object", "", has) {
                app.set_locked(true);
            }
            if item(ui, "Unlock Object", "", has) {
                app.set_locked(false);
            }
            if item(ui, "Unlock All Objects", "", true) {
                app.unlock_all();
            }
            ui.separator();
            ui.menu_button("Shaping", |ui| {
                use crate::ops::Shaping;
                let two = app.selection.len() >= 2;
                for (name, op, need_two) in [
                    ("Weld", Shaping::Weld, true),
                    ("Trim", Shaping::Trim, true),
                    ("Intersect", Shaping::Intersect, true),
                    ("Simplify", Shaping::Simplify, false),
                    ("Front Minus Back", Shaping::FrontMinusBack, true),
                    ("Back Minus Front", Shaping::BackMinusFront, true),
                    ("Boundary", Shaping::Boundary, false),
                ] {
                    if item(ui, name, "", if need_two { two } else { has }) {
                        app.shaping(op);
                    }
                }
            });
            if item(ui, "Convert To Curves", "Ctrl+Q", has) {
                app.convert_to_curves();
            }
            todo(ui, "Convert Outline To Object", "Ctrl+Shift+Q");
            todo(ui, "Join Curves", "");
            ui.separator();
            if item(ui, "Properties", "Alt+Enter", true) {
                app.show_dockers = true;
                app.docker_tab = DockerTab::Properties;
            }
            if item(ui, "Objects", "", true) {
                app.show_dockers = true;
                app.docker_tab = DockerTab::Objects;
            }
        });
        ui.menu_button("Effects", |ui| {
            todo(ui, "Adjust", "");
            todo(ui, "Transform", "");
            todo(ui, "Correct", "");
            ui.separator();
            todo(ui, "Brush Strokes", "");
            todo(ui, "Blend", "");
            todo(ui, "Contour", "Ctrl+F9");
            todo(ui, "Envelope", "Ctrl+F7");
            todo(ui, "Extrude", "");
            todo(ui, "Bevel", "");
            todo(ui, "Lens", "Alt+F3");
            todo(ui, "Drop Shadow", "");
            ui.separator();
            todo(ui, "Add Perspective", "");
            todo(ui, "ClipFrame", "");
            todo(ui, "Rollover", "");
            ui.separator();
            todo(ui, "Clear Effect", "");
            todo(ui, "Copy Effect", "");
            todo(ui, "Clone Effect", "");
        });
        ui.menu_button("Bitmaps", |ui| {
            todo(ui, "Convert To Bitmap...", "");
            todo(ui, "Edit Bitmap...", "");
            todo(ui, "Crop Bitmap", "");
            todo(ui, "Trace Bitmap", "");
            ui.separator();
            todo(ui, "Resample...", "");
            todo(ui, "Mode", "");
            todo(ui, "Bitmap Color Mask", "");
            ui.separator();
            todo(ui, "3D Effects", "");
            todo(ui, "Art Strokes", "");
            todo(ui, "Blur", "");
            todo(ui, "Camera", "");
            todo(ui, "Color Transform", "");
            todo(ui, "Contour", "");
            todo(ui, "Creative", "");
            todo(ui, "Distort", "");
            todo(ui, "Noise", "");
            todo(ui, "Sharpen", "");
        });
        ui.menu_button("Text", |ui| {
            let is_text = app
                .selected_shapes()
                .iter()
                .any(|s| matches!(s.kind, tracedraw_core::document::ShapeKind::Text { .. }));
            todo(ui, "Text Properties", "Ctrl+T");
            todo(ui, "Tabs...", "");
            todo(ui, "Columns...", "");
            todo(ui, "Bullets...", "");
            todo(ui, "Drop Cap...", "");
            ui.separator();
            todo(ui, "Edit Text...", "Ctrl+Shift+T");
            todo(ui, "Insert Character", "Ctrl+F11");
            todo(ui, "Insert Formatting Code", "");
            ui.separator();
            todo(ui, "Fit Text To Path", "");
            todo(ui, "Align To Baseline", "Alt+F12");
            todo(ui, "Straighten Text", "");
            todo(ui, "Paragraph Text Frame", "");
            ui.separator();
            if item(ui, "Convert To Artistic Text", "Ctrl+F8", is_text) {}
            if item(ui, "Convert To Curves", "Ctrl+Q", is_text) {
                app.convert_to_curves();
            }
            ui.separator();
            todo(ui, "Spell Check...", "Ctrl+F12");
            todo(ui, "Font List Options...", "");
        });
        ui.menu_button("Table", |ui| {
            todo(ui, "Create New Table...", "");
            todo(ui, "Convert Text to Table...", "");
            todo(ui, "Convert Table to Text...", "");
            ui.separator();
            todo(ui, "Insert", "");
            todo(ui, "Select", "");
            todo(ui, "Delete", "");
            todo(ui, "Distribute", "");
            todo(ui, "Merge Cells", "Ctrl+M");
            todo(ui, "Split", "");
        });
        ui.menu_button("Tools", |ui| {
            todo(ui, "Options", "Ctrl+J");
            todo(ui, "Customization...", "");
            todo(ui, "Save Settings As Default", "");
            ui.separator();
            todo(ui, "Color Management", "");
            todo(ui, "Scripts", "");
            todo(ui, "Macros", "");
            ui.separator();
            todo(ui, "the vendor CONNECT", "");
            todo(ui, "Create", "");
        });
        ui.menu_button("Window", |ui| {
            todo(ui, "New Window", "");
            todo(ui, "Refresh Window", "Ctrl+W");
            ui.separator();
            let mut d = app.show_dockers;
            if ui.checkbox(&mut d, "Dockers").changed() {
                app.show_dockers = d;
            }
            if item(ui, "Objects", "", true) {
                app.show_dockers = true;
                app.docker_tab = DockerTab::Objects;
            }
            if item(ui, "Properties", "Alt+Enter", true) {
                app.show_dockers = true;
                app.docker_tab = DockerTab::Properties;
            }
            if item(ui, "Hints", "", true) {
                app.show_dockers = true;
                app.docker_tab = DockerTab::Hints;
            }
            if item(ui, "Transformations", "", true) {
                app.show_dockers = true;
                app.docker_tab = DockerTab::Transformations;
            }
            ui.separator();
            let mut sb = app.show_status_bar;
            if ui.checkbox(&mut sb, "Status Bar").changed() {
                app.show_status_bar = sb;
            }
            todo(ui, "Color Palettes", "");
            todo(ui, "Toolbars", "");
            todo(ui, "Workspace", "");
        });
        ui.menu_button("Help", |ui| {
            todo(ui, "Product Help", "F1");
            todo(ui, "Quick Start Guide", "");
            todo(ui, "Video Tutorials", "");
            ui.separator();
            if item(ui, "About TraceDraw", "", true) {
                app.about_open = true;
            }
        });
    });
    let _ = Tool::Pick;
}
