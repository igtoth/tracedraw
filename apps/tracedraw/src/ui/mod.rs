//! UI composition, laid out: menu bar, standard toolbar,
//! property bar, toolbox on the left, docker tab strip on the right edge,
//! document tabs, rulers and canvas in the middle, navigator and colour
//! palette under the canvas, status bar at the bottom.

pub mod context;
pub mod dialogs;
pub mod dockers;
pub mod dockers2;
pub mod icons;
pub mod menus;
pub mod palette;
pub mod preview;
pub mod status;
pub mod toolbar;
pub mod toolbox;
pub mod welcome;

use crate::app::App;
use crate::canvas;
use crate::i18n::tr;
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

    let doc_name = app.document_title();
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
        "TraceDraw - {doc_name}"
    )));

    if app.fullscreen_preview {
        preview::fullscreen(app, ui);
        return;
    }

    Panel::top("menu_bar")
        .frame(bar())
        .show(ui, |ui| menus::menu_bar(app, ui));
    if app.show_standard_toolbar {
        Panel::top("standard_toolbar")
            .frame(bar())
            .show(ui, |ui| toolbar::standard_toolbar(app, ui));
    }
    if app.show_property_bar {
        Panel::top("property_bar")
            .frame(bar())
            .show(ui, |ui| toolbar::property_bar(app, ui));
    }
    if app.show_text_toolbar {
        Panel::top("text_toolbar")
            .frame(bar())
            .show(ui, |ui| toolbar::text_toolbar(app, ui));
    }
    if app.show_zoom_toolbar {
        Panel::top("zoom_toolbar")
            .frame(bar())
            .show(ui, |ui| toolbar::zoom_toolbar(app, ui));
    }
    if app.show_transform_toolbar {
        Panel::top("transform_toolbar")
            .frame(bar())
            .show(ui, |ui| toolbar::transform_toolbar(app, ui));
    }

    if app.show_status_bar {
        Panel::bottom("status_bar")
            .frame(bar())
            .show(ui, |ui| status::status_bar(app, ui));
    }

    if app.show_toolbox {
        Panel::left("toolbox")
            .exact_size(Tokens::TOOLBOX_WIDTH)
            .frame(
                Frame::new()
                    .fill(Tokens::PANEL)
                    .inner_margin(egui::Margin::symmetric(3, 0)),
            )
            .show(ui, |ui| toolbox::toolbox(app, ui));
    }

    // Right-to-left languages mirror the workspace: dockers sit on the left.
    let rtl = crate::i18n::is_rtl();
    let docker_tabs = if rtl {
        Panel::left("docker_tabs")
    } else {
        Panel::right("docker_tabs")
    };
    docker_tabs
        .exact_size(30.0)
        .frame(
            Frame::new()
                .fill(Tokens::PANEL)
                .inner_margin(egui::Margin::symmetric(2, 0)),
        )
        .show(ui, |ui| dockers::tab_strip(app, ui));
    if app.show_dockers {
        let dockers_panel = if rtl {
            Panel::left("dockers")
        } else {
            Panel::right("dockers")
        };
        dockers_panel
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
                if tab(ui, &tr("welcome.title"), app.show_welcome) {
                    app.show_welcome = true;
                }
                if tab(ui, &doc_name, !app.show_welcome) {
                    app.show_welcome = false;
                }
                if ui
                    .add(egui::Button::new(egui::RichText::new("+").size(12.0)).frame(false))
                    .on_hover_text(tr("menu.file.new"))
                    .clicked()
                {
                    let s = app.page_size();
                    app.dialog = dialogs::Dialog::NewDocument {
                        width: s.width,
                        height: s.height,
                        preset: 0,
                        name: App::untitled_name(),
                    };
                }
            });
        });

    if app.show_welcome {
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Tokens::PANEL))
            .show(ui, |ui| welcome::welcome_screen(app, ui));
        dialogs::show(app, &ctx);
        return;
    }
    if app.page_sorter {
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Tokens::PANEL))
            .show(ui, |ui| preview::page_sorter(app, ui));
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
            canvas::draw_effect_nodes(app, &painter);
            context::context_menu(app, ui, &response);

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
        app.about_open = false;
        app.dialog = dialogs::Dialog::About;
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
    let corner = egui::Rect::from_min_max(egui::pos2(full.max.x - sb, full.max.y - sb), full.max);
    p.rect_filled(corner, 0.0, Tokens::PANEL_DARK);
    view_navigator(app, ui, corner, canvas);

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

/// The Navigator: the small button in the corner between the scrollbars
/// opens a thumbnail of the page while the button is held; moving the
/// pointer over the thumbnail pans the view to that spot, like the
/// target design's navigator pop-up.
fn view_navigator(app: &mut App, ui: &mut Ui, corner: egui::Rect, canvas: egui::Rect) {
    let resp = ui.interact(
        corner,
        egui::Id::new("view_navigator"),
        Sense::click_and_drag(),
    );
    let painter = ui.painter_at(corner);
    let icon = corner.shrink(3.0);
    painter.rect_stroke(
        icon,
        1.0,
        egui::Stroke::new(1.0, Tokens::TEXT_DIM),
        egui::StrokeKind::Inside,
    );
    painter.rect_filled(
        egui::Rect::from_center_size(icon.center(), icon.size() * 0.45),
        0.0,
        Tokens::TEXT_DIM,
    );
    let down = ui.input(|i| i.pointer.primary_down());
    if resp.drag_started() || resp.is_pointer_button_down_on() {
        app.navigator_open = true;
    }
    if !down {
        app.navigator_open = false;
        app.navigator_tex = None;
        return;
    }
    if !app.navigator_open {
        return;
    }
    // Thumbnail of the page, rendered once per opening.
    let page = app.page_rect();
    // Fit the page in a 220 px square without changing its aspect.
    let max_side = 220.0_f32;
    let ratio = (page.height() / page.width().max(1e-6)) as f32;
    let (thumb_w, thumb_h) = if ratio > 1.0 {
        ((max_side / ratio).max(40.0), max_side)
    } else {
        (max_side, (max_side * ratio).max(40.0))
    };
    if app.navigator_tex.is_none() {
        let dpi = thumb_w as f64 / page.width() * 25.4;
        if let Some(pm) = tracedraw_render::render_page_image(app.doc(), app.page, dpi) {
            let (w, h) = (pm.width() as usize, pm.height() as usize);
            let mut rgba = Vec::with_capacity(w * h * 4);
            for px in pm.pixels() {
                let c = px.demultiply();
                rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
            }
            let img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
            app.navigator_tex = Some(ui.ctx().load_texture(
                "navigator",
                img,
                egui::TextureOptions::LINEAR,
            ));
        }
    }
    let area_rect = egui::Rect::from_min_max(
        egui::pos2(corner.max.x - thumb_w - 8.0, corner.min.y - thumb_h - 8.0),
        egui::pos2(corner.max.x, corner.min.y),
    );
    egui::Area::new(egui::Id::new("view_navigator_popup"))
        .fixed_pos(area_rect.min)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(thumb_w, thumb_h), Sense::hover());
                let pt = ui.painter();
                pt.rect_filled(rect, 0.0, egui::Color32::WHITE);
                if let Some(tex) = &app.navigator_tex {
                    pt.image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                // Visible area as a rectangle on the thumbnail.
                let vis = app.view.visible_page_rect(canvas);
                let sx = rect.width() / page.width() as f32;
                let sy = rect.height() / page.height() as f32;
                let to_thumb = |x: f64, y: f64| {
                    egui::pos2(
                        rect.min.x + (x - page.x0) as f32 * sx,
                        rect.max.y - (y - page.y0) as f32 * sy,
                    )
                };
                let vr =
                    egui::Rect::from_two_pos(to_thumb(vis.x0, vis.y1), to_thumb(vis.x1, vis.y0))
                        .intersect(rect);
                pt.rect_stroke(
                    vr,
                    0.0,
                    egui::Stroke::new(1.5, Tokens::SELECTION),
                    egui::StrokeKind::Inside,
                );
                // Pointer over the thumbnail: centre the view there.
                if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
                    if rect.contains(pos) {
                        let px = page.x0 + ((pos.x - rect.min.x) / sx) as f64;
                        let py = page.y0 + ((rect.max.y - pos.y) / sy) as f64;
                        let target = app.view.to_screen(Point::new(px, py));
                        let delta = canvas.center() - target;
                        if delta.length() > 0.01 {
                            app.view.pan(delta);
                            // The canvas was painted before this pan; redraw.
                            ui.ctx().request_repaint();
                        }
                    }
                }
            });
        });
}
