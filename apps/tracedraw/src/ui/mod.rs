//! UI composition, laid out like the editor 2019: menu bar, standard toolbar,
//! property bar, toolbox on the left, docker tab strip on the right edge,
//! document tabs, rulers and canvas in the middle, navigator and colour
//! palette under the canvas, status bar at the bottom.

pub mod dockers;
pub mod icons;
pub mod menus;
pub mod palette;
pub mod status;
pub mod toolbar;
pub mod toolbox;

use crate::app::App;
use crate::canvas;
use crate::theme::Tokens;
use egui::{Frame, Panel, Sense, Ui};

fn bar() -> Frame {
    Frame::new()
        .fill(Tokens::PANEL)
        .inner_margin(egui::Margin::symmetric(4, 2))
}

pub fn root(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    app.keyboard(&ctx);

    let doc_name = app
        .file
        .as_ref()
        .and_then(|p| p.file_stem())
        .and_then(|n| n.to_str())
        .map(String::from)
        .unwrap_or_else(|| "Untitled-1".into());
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
        "TraceDraw - {doc_name}{}",
        if app.engine.is_dirty() { "*" } else { "" }
    )));

    Panel::top("menu_bar")
        .frame(bar())
        .show(ui, |ui| menus::menu_bar(app, ui));
    Panel::top("standard_toolbar")
        .frame(bar())
        .show(ui, |ui| toolbar::standard_toolbar(app, ui));
    Panel::top("property_bar")
        .frame(bar())
        .show(ui, |ui| toolbar::property_bar(app, ui));

    if app.show_status_bar {
        Panel::bottom("status_bar")
            .frame(bar())
            .show(ui, |ui| status::status_bar(app, ui));
    }

    Panel::left("toolbox")
        .exact_size(Tokens::TOOLBOX_WIDTH)
        .frame(
            Frame::new()
                .fill(Tokens::PANEL)
                .inner_margin(egui::Margin::symmetric(3, 0)),
        )
        .show(ui, |ui| toolbox::toolbox(app, ui));

    Panel::right("docker_tabs")
        .exact_size(30.0)
        .frame(
            Frame::new()
                .fill(Tokens::PANEL)
                .inner_margin(egui::Margin::symmetric(2, 0)),
        )
        .show(ui, |ui| dockers::tab_strip(app, ui));
    if app.show_dockers {
        Panel::right("dockers")
            .default_size(300.0)
            .resizable(true)
            .frame(Frame::new().fill(Tokens::PANEL).inner_margin(6))
            .show(ui, |ui| dockers::dockers(app, ui));
    }

    // Under the canvas: document palette row, then navigator + palette row.
    Panel::bottom("document_palette")
        .frame(bar())
        .show(ui, |ui| palette::document_palette_row(app, ui));
    Panel::bottom("palette").frame(bar()).show(ui, |ui| {
        ui.horizontal(|ui| {
            status::navigator(app, ui);
            ui.add(egui::Separator::default().vertical());
            palette::palette_row(app, ui);
        });
    });

    // Document tabs above the rulers.
    Panel::top("document_tabs")
        .frame(
            Frame::new()
                .fill(Tokens::PANEL_DARK)
                .inner_margin(egui::Margin::symmetric(4, 1)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let _ = ui.add(
                    egui::Button::new(egui::RichText::new("Welcome Screen").size(12.0))
                        .frame(false),
                );
                let _ = ui.add(
                    egui::Button::new(egui::RichText::new(doc_name.clone()).size(12.0))
                        .fill(Tokens::PANEL)
                        .stroke(egui::Stroke::new(1.0, Tokens::BORDER)),
                );
                if ui
                    .add(egui::Button::new(egui::RichText::new("+").size(12.0)).frame(false))
                    .on_hover_text("New document")
                    .clicked()
                {
                    app.new_document();
                }
            });
        });

    egui::CentralPanel::default()
        .frame(Frame::NONE)
        .show(ui, |ui| {
            let full = ui.available_rect_before_wrap();
            let ruler = if app.show_rulers { Tokens::RULER } else { 0.0 };
            let canvas_rect =
                egui::Rect::from_min_max(full.min + egui::vec2(ruler, ruler), full.max);
            app.canvas_rect = canvas_rect;

            if app.fit_pending {
                app.view.fit(app.page_rect(), canvas_rect);
                app.fit_pending = false;
            }

            let response = ui.allocate_rect(canvas_rect, Sense::click_and_drag());
            let painter = ui.painter_at(canvas_rect);

            // Wheel: zoom with Ctrl, otherwise scroll. Pinch zooms.
            if response.hovered() {
                let (scroll, zoom_delta, mods) =
                    ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers));
                if let Some(h) = response.hover_pos() {
                    if zoom_delta != 1.0 {
                        app.view.zoom_at(h, zoom_delta);
                    } else if mods.ctrl && scroll.y != 0.0 {
                        app.view
                            .zoom_at(h, if scroll.y > 0.0 { 1.15 } else { 1.0 / 1.15 });
                    } else if scroll != egui::Vec2::ZERO {
                        app.view.pan(if mods.shift {
                            egui::vec2(scroll.y, scroll.x)
                        } else {
                            scroll
                        });
                    }
                }
            }
            let mods = ui.input(|i| i.modifiers);
            app.canvas_input(&response, mods);

            canvas::draw_canvas(app, &painter, canvas_rect);

            if app.show_rulers {
                let top = egui::Rect::from_min_max(
                    egui::pos2(full.min.x + ruler, full.min.y),
                    egui::pos2(full.max.x, full.min.y + ruler),
                );
                let left = egui::Rect::from_min_max(
                    egui::pos2(full.min.x, full.min.y + ruler),
                    egui::pos2(full.min.x + ruler, full.max.y),
                );
                let corner = egui::Rect::from_min_size(full.min, egui::vec2(ruler, ruler));
                let rp = ui.painter_at(full);
                canvas::draw_rulers(app, &rp, top, left);
                rp.rect_filled(corner, 0.0, Tokens::RULER_BG);
                rp.text(
                    corner.center(),
                    egui::Align2::CENTER_CENTER,
                    app.units.short(),
                    egui::FontId::proportional(8.0),
                    Tokens::TEXT_DIM,
                );
            }

            let cursor = match app.tool {
                crate::tools::Tool::Pan => egui::CursorIcon::Grab,
                crate::tools::Tool::Zoom => egui::CursorIcon::ZoomIn,
                crate::tools::Tool::Text => egui::CursorIcon::Text,
                crate::tools::Tool::Pick | crate::tools::Tool::FreeformPick => {
                    egui::CursorIcon::Default
                }
                _ => egui::CursorIcon::Crosshair,
            };
            if response.hovered() {
                ui.output_mut(|o| o.cursor_icon = cursor);
            }
        });

    if app.about_open {
        egui::Window::new("About TraceDraw")
            .collapsible(false)
            .resizable(false)
            .open(&mut app.about_open)
            .show(&ctx, |ui| {
                ui.label("TraceDraw 0.1 (pre-alpha)");
                ui.label("An open-source vector illustration editor in Rust.");
                ui.label("MIT or Apache-2.0. Not affiliated with the vendor.");
            });
    }
}
