//! Modal-ish dialogs: Rename Page, Go To Page, Page Size, Options, and the
//! Welcome Screen tab content.

use crate::app::{App, Units};
use crate::theme::Tokens;
use egui::Context;
use tracedraw_core::{geometry::Size, Command};

#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    None,
    RenamePage {
        name: String,
    },
    GoToPage {
        page: usize,
    },
    PageSize {
        width: f64,
        height: f64,
        all_pages: bool,
    },
    Options,
    RenameLayer {
        layer: tracedraw_core::LayerId,
        name: String,
    },
    DocumentProperties,
}

pub fn show(app: &mut App, ctx: &Context) {
    let mut dialog = std::mem::replace(&mut app.dialog, Dialog::None);
    let mut close = false;
    match &mut dialog {
        Dialog::None => {}
        Dialog::RenamePage { name } => {
            egui::Window::new("Rename Page")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Page name:");
                        ui.text_edit_singleline(name);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            let page = app.page;
                            app.run(Command::RenamePage {
                                page,
                                name: name.clone(),
                            });
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
        }
        Dialog::GoToPage { page } => {
            let n = app.doc().pages.len();
            egui::Window::new("Go To Page")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Go to page (1 to {n}):"));
                        ui.add(egui::DragValue::new(page).range(1..=n.max(1)));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            app.goto_page(page.saturating_sub(1));
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
        }
        Dialog::PageSize {
            width,
            height,
            all_pages,
        } => {
            egui::Window::new("Page Size")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    let u = app.units;
                    let mut w = u.from_mm(*width);
                    let mut h = u.from_mm(*height);
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(
                            egui::DragValue::new(&mut w)
                                .speed(0.5)
                                .suffix(format!(" {}", u.short())),
                        );
                        ui.label("Height:");
                        ui.add(
                            egui::DragValue::new(&mut h)
                                .speed(0.5)
                                .suffix(format!(" {}", u.short())),
                        );
                    });
                    *width = u.to_mm(w);
                    *height = u.to_mm(h);
                    ui.horizontal(|ui| {
                        for (n, s) in [
                            ("A4", tracedraw_core::document::paper::A4),
                            ("A3", tracedraw_core::document::paper::A3),
                            ("Letter", tracedraw_core::document::paper::LETTER),
                        ] {
                            if ui.button(n).clicked() {
                                *width = s.width;
                                *height = s.height;
                            }
                        }
                        if ui
                            .button("Swap")
                            .on_hover_text("Switch orientation")
                            .clicked()
                        {
                            std::mem::swap(width, height);
                        }
                    });
                    ui.checkbox(all_pages, "Apply to all pages");
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            let size = Size::new(width.max(1.0), height.max(1.0));
                            let pages: Vec<_> = if *all_pages {
                                app.doc().pages.iter().map(|p| p.id).collect()
                            } else {
                                vec![app.page]
                            };
                            let cmds: Vec<Command> = pages
                                .into_iter()
                                .map(|page| Command::ResizePage { page, size })
                                .collect();
                            let _ = app.engine.run_batch("Page Size", &cmds);
                            app.fit_pending = true;
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
        }
        Dialog::RenameLayer { layer, name } => {
            egui::Window::new("Rename Layer")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Layer name:");
                        ui.text_edit_singleline(name);
                    });
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            app.run(Command::RenameLayer {
                                layer: *layer,
                                name: name.clone(),
                            });
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                });
        }
        Dialog::Options => {
            egui::Window::new("Options")
                .collapsible(false)
                .resizable(false)
                .default_width(360.0)
                .show(ctx, |ui| {
                    ui.strong("Workspace");
                    ui.horizontal(|ui| {
                        ui.label("Units:");
                        egui::ComboBox::from_id_salt("opt_units")
                            .selected_text(app.units.label())
                            .show_ui(ui, |ui| {
                                for u in Units::ALL {
                                    if ui.selectable_label(app.units == u, u.label()).clicked() {
                                        app.units = u;
                                    }
                                }
                            });
                    });
                    let u = app.units;
                    let mut nudge = u.from_mm(app.nudge_mm);
                    ui.horizontal(|ui| {
                        ui.label("Nudge:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut nudge)
                                    .speed(0.1)
                                    .suffix(format!(" {}", u.short())),
                            )
                            .changed()
                            && nudge > 0.0
                        {
                            app.nudge_mm = u.to_mm(nudge);
                        }
                    });
                    let mut dx = u.from_mm(app.duplicate_offset.x);
                    let mut dy = u.from_mm(app.duplicate_offset.y);
                    ui.horizontal(|ui| {
                        ui.label("Duplicate offset:");
                        ui.add(
                            egui::DragValue::new(&mut dx)
                                .speed(0.1)
                                .suffix(format!(" {}", u.short())),
                        );
                        ui.add(
                            egui::DragValue::new(&mut dy)
                                .speed(0.1)
                                .suffix(format!(" {}", u.short())),
                        );
                    });
                    app.duplicate_offset =
                        tracedraw_core::geometry::Vec2::new(u.to_mm(dx), u.to_mm(dy));
                    ui.separator();
                    ui.strong("Snapping");
                    ui.checkbox(&mut app.snap.grid, "Document grid");
                    ui.checkbox(&mut app.snap.guides, "Guidelines");
                    ui.checkbox(&mut app.snap.objects, "Objects");
                    ui.checkbox(&mut app.snap.page, "Page");
                    ui.separator();
                    ui.strong("Display");
                    ui.checkbox(&mut app.show_rulers, "Rulers");
                    ui.checkbox(&mut app.show_grid, "Grid");
                    ui.checkbox(&mut app.show_guides, "Guidelines");
                    ui.checkbox(&mut app.show_status_bar, "Status bar");
                    ui.add_space(6.0);
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
        }
        Dialog::DocumentProperties => {
            egui::Window::new("Document Properties")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    let doc = app.doc();
                    ui.label(format!("Title: {}", doc.title));
                    ui.label(format!("Pages: {}", doc.pages.len()));
                    let objects: usize = doc
                        .pages
                        .iter()
                        .flat_map(|p| &p.layers)
                        .map(|l| l.shapes.len())
                        .sum();
                    ui.label(format!("Objects: {objects}"));
                    if let Some(f) = &app.file {
                        ui.label(format!("File: {}", f.display()));
                    }
                    ui.label(
                        egui::RichText::new("Colour profile: sRGB (no ICC management yet)")
                            .color(Tokens::TEXT_DIM),
                    );
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
        }
    }
    app.dialog = if close { Dialog::None } else { dialog };
}
