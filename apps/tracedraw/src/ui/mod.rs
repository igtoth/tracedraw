//! UI composition, laid out: menu bar, standard toolbar,
//! property bar, toolbox on the left, docker tab strip on the right edge,
//! document tabs, rulers and canvas in the middle, navigator and colour
//! palette under the canvas, status bar at the bottom.

pub mod dialogs;
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
use tracedraw_core::geometry::Point;

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
                let tab = |ui: &mut Ui, label: &str, active: bool| -> bool {
                    ui.add(
                        egui::Button::new(egui::RichText::new(label).size(12.0))
                            .fill(if active {
                                Tokens::PANEL
                            } else {
                                Tokens::PANEL_DARK
                            })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if active {
                                    Tokens::BORDER
                                } else {
                                    Tokens::PANEL_DARK
                                },
                            )),
                    )
                    .clicked()
                };
                if tab(ui, "Welcome Screen", app.show_welcome) {
                    app.show_welcome = true;
                }
                if tab(ui, &doc_name, !app.show_welcome) {
                    app.show_welcome = false;
                }
                if ui
                    .add(egui::Button::new(egui::RichText::new("+").size(12.0)).frame(false))
                    .on_hover_text("New document")
                    .clicked()
                {
                    app.new_document();
                }
            });
        });

    if app.show_welcome {
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Tokens::PANEL))
            .show(ui, |ui| welcome_screen(app, ui));
        dialogs::show(app, &ctx);
        return;
    }

    egui::CentralPanel::default()
        .frame(Frame::NONE)
        .show(ui, |ui| {
            let full = ui.available_rect_before_wrap();
            let ruler = if app.show_rulers { Tokens::RULER } else { 0.0 };
            let sb = Tokens::SCROLLBAR;
            let canvas_rect = egui::Rect::from_min_max(
                full.min + egui::vec2(ruler, ruler),
                full.max - egui::vec2(sb, sb),
            );
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

            // A guideline being dragged out of a ruler follows the pointer until release.
            if let crate::app::Drag::NewGuide { horizontal, .. } = app.drag {
                if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
                    app.drag = crate::app::Drag::NewGuide {
                        horizontal,
                        pos: app.view.to_page(pos),
                    };
                }
                if ui.input(|i| i.pointer.primary_released()) {
                    app.finish_guide_drag();
                }
            } else {
                app.canvas_input(&response, mods);
            }

            canvas::draw_canvas(app, &painter, canvas_rect);
            canvas::draw_guides(app, &painter, canvas_rect);

            // Scrollbars: the desktop extends one page size around the page.
            scrollbars(app, ui, full, canvas_rect, ruler, sb);

            if app.show_rulers {
                let top = egui::Rect::from_min_max(
                    egui::pos2(full.min.x + ruler, full.min.y),
                    egui::pos2(full.max.x - sb, full.min.y + ruler),
                );
                let left = egui::Rect::from_min_max(
                    egui::pos2(full.min.x, full.min.y + ruler),
                    egui::pos2(full.min.x + ruler, full.max.y - sb),
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
                // Dragging out of a ruler creates a guideline.
                let top_resp = ui.interact(top, egui::Id::new("ruler_top"), Sense::drag());
                let left_resp = ui.interact(left, egui::Id::new("ruler_left"), Sense::drag());
                if top_resp.drag_started() {
                    app.drag = crate::app::Drag::NewGuide {
                        horizontal: true,
                        pos: Point::ZERO,
                    };
                }
                if left_resp.drag_started() {
                    app.drag = crate::app::Drag::NewGuide {
                        horizontal: false,
                        pos: Point::ZERO,
                    };
                }
                if top_resp.hovered() || left_resp.hovered() {
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
                }
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

    dialogs::show(app, &ctx);

    if app.about_open {
        egui::Window::new("About TraceDraw")
            .collapsible(false)
            .resizable(false)
            .open(&mut app.about_open)
            .show(&ctx, |ui| {
                ui.label("TraceDraw 0.1 (pre-alpha)");
                ui.label("An open-source vector illustration editor in Rust.");
                ui.label("MIT or Apache-2.0. Not affiliated with any other vendor.");
            });
    }
}

/// Horizontal and vertical scrollbars at the edges of the document window.
fn scrollbars(
    app: &mut App,
    ui: &mut Ui,
    full: egui::Rect,
    canvas: egui::Rect,
    ruler: f32,
    sb: f32,
) {
    let page = app.page_rect();
    let desktop = page.inflate(page.width(), page.height());
    let view = app.view;
    let visible_x0 = view.to_page(canvas.left_top()).x;
    let visible_x1 = view.to_page(canvas.right_top()).x;
    let visible_y1 = view.to_page(canvas.left_top()).y;
    let visible_y0 = view.to_page(canvas.left_bottom()).y;
    let ext_x0 = desktop.x0.min(visible_x0);
    let ext_x1 = desktop.x1.max(visible_x1);
    let ext_y0 = desktop.y0.min(visible_y0);
    let ext_y1 = desktop.y1.max(visible_y1);

    let hbar = egui::Rect::from_min_max(
        egui::pos2(full.min.x + ruler, full.max.y - sb),
        egui::pos2(full.max.x - sb, full.max.y),
    );
    let vbar = egui::Rect::from_min_max(
        egui::pos2(full.max.x - sb, full.min.y + ruler),
        egui::pos2(full.max.x, full.max.y - sb),
    );
    let p = ui.painter_at(full);
    p.rect_filled(hbar, 0.0, Tokens::PANEL_DARK);
    p.rect_filled(vbar, 0.0, Tokens::PANEL_DARK);
    p.rect_filled(
        egui::Rect::from_min_max(egui::pos2(full.max.x - sb, full.max.y - sb), full.max),
        0.0,
        Tokens::PANEL_DARK,
    );

    // Horizontal thumb.
    let ext_w = (ext_x1 - ext_x0).max(1e-6);
    let t0 = ((visible_x0 - ext_x0) / ext_w) as f32;
    let t1 = ((visible_x1 - ext_x0) / ext_w) as f32;
    let thumb_h = egui::Rect::from_min_max(
        egui::pos2(hbar.min.x + hbar.width() * t0, hbar.min.y + 3.0),
        egui::pos2(
            hbar.min.x + hbar.width() * t1.max(t0 + 0.02),
            hbar.max.y - 3.0,
        ),
    );
    let rh = ui.interact(hbar, egui::Id::new("hscroll"), Sense::click_and_drag());
    p.rect_filled(
        thumb_h,
        3.0,
        if rh.hovered() || rh.dragged() {
            Tokens::TEXT_DIM
        } else {
            Tokens::BORDER
        },
    );
    if rh.dragged() {
        let d = rh.drag_delta().x / hbar.width() * ext_w as f32;
        app.view.pan(egui::vec2(-d * view.zoom, 0.0));
    }
    // Vertical thumb (page y up: top of the bar is ext_y1).
    let ext_h = (ext_y1 - ext_y0).max(1e-6);
    let s0 = ((ext_y1 - visible_y1) / ext_h) as f32;
    let s1 = ((ext_y1 - visible_y0) / ext_h) as f32;
    let thumb_v = egui::Rect::from_min_max(
        egui::pos2(vbar.min.x + 3.0, vbar.min.y + vbar.height() * s0),
        egui::pos2(
            vbar.max.x - 3.0,
            vbar.min.y + vbar.height() * s1.max(s0 + 0.02),
        ),
    );
    let rv = ui.interact(vbar, egui::Id::new("vscroll"), Sense::click_and_drag());
    p.rect_filled(
        thumb_v,
        3.0,
        if rv.hovered() || rv.dragged() {
            Tokens::TEXT_DIM
        } else {
            Tokens::BORDER
        },
    );
    if rv.dragged() {
        let d = rv.drag_delta().y / vbar.height() * ext_h as f32;
        app.view.pan(egui::vec2(0.0, -d * view.zoom));
    }
}

fn welcome_screen(app: &mut App, ui: &mut Ui) {
    ui.add_space(30.0);
    ui.vertical_centered(|ui| {
        ui.heading(egui::RichText::new("TraceDraw").size(32.0));
        ui.label(egui::RichText::new("Open-source vector illustration").color(Tokens::TEXT_DIM));
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 220.0);
            if ui.add_sized([200.0, 60.0], egui::Button::new("New Document\nCtrl+N")).clicked() {
                app.new_document();
                app.show_welcome = false;
            }
            ui.add_space(20.0);
            if ui.add_sized([200.0, 60.0], egui::Button::new("Open Document...\nCtrl+O")).clicked() {
                app.open_dialog();
                app.show_welcome = false;
            }
        });
        ui.add_space(24.0);
        ui.label(egui::RichText::new("Page sizes").strong());
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() / 2.0 - 160.0);
            use tracedraw_core::document::paper;
            for (n, size) in [("A4 portrait", paper::A4), ("A4 landscape", tracedraw_core::geometry::Size::new(paper::A4.height, paper::A4.width)), ("A3", paper::A3), ("Letter", paper::LETTER)] {
                if ui.button(n).clicked() {
                    let doc = tracedraw_core::Document::new("Untitled-1", size);
                    app.page = doc.pages[0].id;
                    app.engine.replace(doc);
                    app.file = None;
                    app.fit_pending = true;
                    app.show_welcome = false;
                }
            }
        });
        ui.add_space(40.0);
        ui.label(egui::RichText::new("Shortcuts: F6 rectangle, F7 ellipse, F8 text, F5 freehand, F10 shape, Space pick, Shift+F4 zoom to page").color(Tokens::TEXT_DIM).size(11.0));
    });
}
