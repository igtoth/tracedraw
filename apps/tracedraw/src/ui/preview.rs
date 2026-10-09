//! Full-screen preview (F9) and the Page Sorter view.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::Ui;
use tracedraw_core::Command;

pub fn fullscreen(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_gray(40)))
        .show(ui, |ui| {
            let rect = ui.available_rect_before_wrap();
            let page = app.page_rect();
            let bounds = if app.preview_selected_only {
                app.selection_bounds().unwrap_or(page)
            } else {
                page
            };
            let painter = ui.painter_at(rect);
            // Fit the page into the screen.
            let margin = 20.0;
            let sx = (rect.width() - 2.0 * margin) as f64 / bounds.width().max(1e-6);
            let sy = (rect.height() - 2.0 * margin) as f64 / bounds.height().max(1e-6);
            let zoom = sx.min(sy);
            let w = (bounds.width() * zoom).ceil() as u32;
            let h = (bounds.height() * zoom).ceil() as u32;
            let doc = app.engine.document().clone();
            let view = tracedraw_render::ViewTransform {
                zoom,
                origin_x: -bounds.x0 * zoom,
                origin_y: bounds.y1 * zoom,
            };
            let opts = tracedraw_render::RenderOptions {
                width: w.max(1),
                height: h.max(1),
                view,
                preview: None,
                wireframe: false,
                ..tracedraw_render::RenderOptions::default()
            };
            let page_id = app.page;
            if let Some(mut pm) = tracedraw_render::render_page(&doc, page_id, &opts) {
                if !app.preview_selected_only {
                    let mut white = tiny_skia::Pixmap::new(pm.width(), pm.height())
                        .unwrap_or_else(|| tiny_skia::Pixmap::new(1, 1).expect("pixmap"));
                    white.fill(tiny_skia::Color::WHITE);
                    white.draw_pixmap(
                        0,
                        0,
                        pm.as_ref(),
                        &Default::default(),
                        tiny_skia::Transform::identity(),
                        None,
                    );
                    pm = white;
                }
                let image = egui::ColorImage::from_rgba_premultiplied(
                    [pm.width() as usize, pm.height() as usize],
                    pm.data(),
                );
                let tex =
                    ctx.load_texture("fullscreen_preview", image, egui::TextureOptions::LINEAR);
                let size = egui::vec2(pm.width() as f32, pm.height() as f32);
                let pos = rect.center() - size / 2.0;
                painter.image(
                    tex.id(),
                    egui::Rect::from_min_size(pos, size),
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            painter.text(
                egui::pos2(rect.center().x, rect.max.y - 10.0),
                egui::Align2::CENTER_BOTTOM,
                tr("preview.exit_hint"),
                egui::FontId::proportional(12.0),
                egui::Color32::from_gray(200),
            );
        });
    let exit = ctx.input(|i| {
        i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::F9) || i.pointer.any_click()
    });
    if exit {
        app.fullscreen_preview = false;
        app.preview_selected_only = false;
    }
}

pub fn page_sorter(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.heading(tr("menu.view.page_sorter"));
        if ui.button(tr("preview.back_to_page")).clicked() {
            app.page_sorter = false;
        }
        if ui.button(tr("menu.layout.insert_page")).clicked() {
            app.add_page();
        }
    });
    ui.separator();
    let pages: Vec<(tracedraw_core::PageId, String, tracedraw_core::Size)> = app
        .doc()
        .pages
        .iter()
        .map(|p| (p.id, p.name.clone(), p.size))
        .collect();
    let n = pages.len();
    let doc = app.engine.document().clone();
    let mut goto: Option<usize> = None;
    let mut mv: Option<(tracedraw_core::PageId, usize)> = None;
    let mut del: Option<tracedraw_core::PageId> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, (id, name, size)) in pages.iter().enumerate() {
                let thumb_h = 150.0f32;
                let thumb_w = (thumb_h as f64 * size.width / size.height.max(1e-6)) as f32;
                ui.vertical(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(thumb_w.max(40.0), thumb_h),
                        egui::Sense::click(),
                    );
                    let active = *id == app.page;
                    ui.painter().rect_filled(
                        rect.expand(3.0),
                        3.0,
                        if active {
                            Tokens::TOOL_ACTIVE
                        } else {
                            Tokens::PANEL_DARK
                        },
                    );
                    let dpi = thumb_h as f64 / (size.height / 25.4);
                    if let Some(pm) = tracedraw_render::render_page_image(&doc, *id, dpi.max(4.0)) {
                        let image = egui::ColorImage::from_rgba_premultiplied(
                            [pm.width() as usize, pm.height() as usize],
                            pm.data(),
                        );
                        let tex = ui.ctx().load_texture(
                            format!("sorter_{}", id.raw()),
                            image,
                            egui::TextureOptions::LINEAR,
                        );
                        ui.painter().image(
                            tex.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                    if resp.clicked() {
                        goto = Some(i);
                    }
                    if resp.double_clicked() {
                        goto = Some(i);
                        app.page_sorter = false;
                    }
                    ui.horizontal(|ui| {
                        ui.label(format!("{}. {name}", i + 1));
                        if i > 0 && ui.small_button("\u{25C0}").clicked() {
                            mv = Some((*id, i - 1));
                        }
                        if i + 1 < n && ui.small_button("\u{25B6}").clicked() {
                            mv = Some((*id, i + 1));
                        }
                        if n > 1 && ui.small_button("x").clicked() {
                            del = Some(*id);
                        }
                    });
                });
                ui.add_space(12.0);
            }
        });
    });
    if let Some(i) = goto {
        app.goto_page(i);
    }
    if let Some((id, to)) = mv {
        app.run(Command::MovePage { page: id, to });
    }
    if let Some(id) = del {
        app.run(Command::DeletePage { page: id });
        if app.page == id {
            app.page = app.doc().pages[0].id;
        }
    }
}
