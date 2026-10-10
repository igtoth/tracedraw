//! UI composition, laid out: menu bar, standard toolbar,
//! property bar, toolbox on the left, docker tab strip on the right edge,
//! document tabs, rulers and canvas in the middle, navigator and colour
//! palette under the canvas, status bar at the bottom.

pub mod bitmap_dialogs;
pub mod bitmap_preview;
pub mod chrome;
pub mod context;
pub mod coords_docker;
pub mod curve_edit;
pub mod dialogs;
pub mod dockers;
pub mod dockers2;
pub mod effect_dialog;
pub mod field;
pub mod hints;
pub mod icons;
pub mod layout_options;
pub mod menus;
pub mod options;
pub mod palette;
pub mod preview;
pub mod propbar;
pub mod rulers;
pub mod status;
pub mod tabs;
pub mod toolbar;
pub mod toolbox;
pub mod welcome;
pub mod window_bars;

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

/// Set the window title (the page title in a browser) when it changes.
fn set_window_title(ctx: &egui::Context, title: String) {
    let id = egui::Id::new("tracedraw_window_title");
    if ctx.data(|d| d.get_temp::<String>(id)).as_deref() == Some(title.as_str()) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(id, title.clone()));
    #[cfg(not(target_arch = "wasm32"))]
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    #[cfg(target_arch = "wasm32")]
    crate::web::set_title(&title);
}

pub fn root(app: &mut App, ui: &mut Ui) {
    // Dockers set their canvas previews again each frame they are shown.
    app.docker_preview.clear();
    app.docker_preview_point = None;
    let ctx = ui.ctx().clone();
    // Pixels are measured at the active drawing's resolution.
    crate::app::set_pixel_dpi(app.document_dpi());
    app.keyboard(&ctx);

    let doc_name = if app.has_document() && !app.show_welcome {
        app.document_title()
    } else {
        tr("welcome.title")
    };
    set_window_title(&ctx, format!("TraceDraw - {doc_name}"));

    // Closing the window with unsaved drawings asks about each first.
    #[cfg(not(target_arch = "wasm32"))]
    {
        if ctx.input(|i| i.viewport().close_requested())
            && !app.quit_now
            && !app.dirty_documents().is_empty()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            app.request_exit();
        }
        if app.quit_now {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    if !app.has_document() {
        app.show_welcome = true;
    }
    // Options > Display > Show tooltips.
    let delay = if app.settings.show_tooltips {
        0.5
    } else {
        f32::INFINITY
    };
    if ctx.global_style().interaction.tooltip_delay != delay {
        ctx.all_styles_mut(|s| s.interaction.tooltip_delay = delay);
    }
    // Options > Save > Auto-backup.
    if app.auto_backup_tick(web_time::Instant::now()) {
        app.status = tr("status.auto_backup_saved");
    }

    if app.fullscreen_preview {
        preview::fullscreen(app, ui);
        return;
    }

    Panel::top("menu_bar")
        .frame(
            Frame::new()
                .fill(Tokens::MENU_BAR)
                .inner_margin(egui::Margin::symmetric(0, 1)),
        )
        .exact_size(24.0)
        .resizable(false)
        .show_separator_line(false)
        .show(ui, |ui| menus::menu_bar(app, ui));
    // the target design's bands: a 35 px standard toolbar and a 52 px
    // property bar, each closed by a light line (included in the sizes).
    if app.show_standard_toolbar {
        Panel::top("standard_toolbar")
            .frame(Frame::new().fill(Tokens::PANEL).inner_margin(egui::Margin {
                left: 4,
                right: 4,
                top: 2,
                bottom: 2,
            }))
            .exact_size(36.0)
            .resizable(false)
            .show(ui, |ui| toolbar::standard_toolbar(app, ui));
    }
    if app.show_property_bar {
        Panel::top("property_bar")
            .frame(Frame::new().fill(Tokens::PANEL).inner_margin(egui::Margin {
                left: 4,
                right: 4,
                top: 8,
                bottom: 8,
            }))
            .exact_size(53.0)
            .resizable(false)
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
            .frame(Frame::new().fill(Tokens::PANEL))
            .exact_size(status::BAR_H)
            .show_separator_line(false)
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
        .exact_size(27.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(dockers::TITLE_FILL))
        .show(ui, |ui| dockers::tab_strip(app, ui));
    if app.show_dockers {
        let dockers_panel = if rtl {
            Panel::left("dockers")
        } else {
            Panel::right("dockers")
        };
        dockers_panel
            .default_size(340.0)
            .resizable(true)
            .frame(Frame::new().fill(egui::Color32::WHITE))
            .show(ui, |ui| dockers::dockers(app, ui));
    }

    // Under the canvas: the default palette, then the document palette
    // above the status bar.
    let palette_frame = Frame::new().fill(Tokens::PANEL);
    if app.settings.palette.show_document {
        Panel::bottom("document_palette")
            .frame(palette_frame)
            .exact_size(palette::ROW_H)
            .resizable(false)
            .show_separator_line(false)
            .show(ui, |ui| palette::document_palette_row(app, ui));
    }
    Panel::bottom("palette")
        .frame(palette_frame)
        .exact_size(palette::ROW_H)
        .resizable(false)
        .show_separator_line(false)
        .show(ui, |ui| palette::palette_row(app, ui));
    palette::popups(app, &ctx);

    // Document tabs above the rulers.
    Panel::top("document_tabs")
        .frame(Frame::new().fill(tabs::STRIP))
        .show(ui, |ui| tabs::document_tabs(app, ui));

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
            let sb = window_bars::BAR;
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

            // Wheel: with the default setting the wheel zooms about the
            // pointer, Ctrl+wheel scrolls vertically and Alt+wheel
            // horizontally; with the scroll setting the wheel scrolls,
            // Shift+wheel horizontally and Ctrl+wheel zooms. Pinch zooms.
            if response.hovered() {
                let (scroll, notches, zoom_delta, mods) = ui.input(|i| {
                    // Wheel notches this frame (the smoothed delta is spread
                    // over several frames, so the zoom steps come from the
                    // raw events: one notch is one line or 50 points).
                    let notches: f32 = i
                        .events
                        .iter()
                        .map(|e| match e {
                            egui::Event::MouseWheel { unit, delta, .. } => match unit {
                                egui::MouseWheelUnit::Line => delta.y,
                                egui::MouseWheelUnit::Point => delta.y / 50.0,
                                egui::MouseWheelUnit::Page => delta.y * 10.0,
                            },
                            _ => 0.0,
                        })
                        .sum();
                    (i.smooth_scroll_delta, notches, i.zoom_delta(), i.modifiers)
                });
                if let Some(h) = response.hover_pos() {
                    let zoom_factor = 1.15f32.powf(notches);
                    if zoom_delta != 1.0 {
                        app.view.zoom_at(h, zoom_delta);
                    } else if app.settings.wheel_zooms {
                        if mods.alt && scroll != egui::Vec2::ZERO {
                            app.view.pan(egui::vec2(scroll.y, scroll.x));
                        } else if mods.ctrl && scroll != egui::Vec2::ZERO {
                            app.view.pan(scroll);
                        } else if notches != 0.0 {
                            app.view.zoom_at(h, zoom_factor);
                        }
                    } else if mods.ctrl && notches != 0.0 {
                        app.view.zoom_at(h, zoom_factor);
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

            // A guideline or the ruler origin dragged out of the rulers
            // follows the pointer until release.
            if !rulers::ruler_drag(app, ui) {
                app.canvas_input(&response, mods);
            }

            canvas::draw_canvas(app, &painter, canvas_rect);
            canvas::draw_guides(app, &painter, canvas_rect);
            rulers::draw_origin_drag(app, &painter, canvas_rect);
            canvas::draw_effect_nodes(app, &painter);
            canvas::draw_docker_preview(app, &painter);
            canvas::draw_snap_mark(app, &painter);
            context::context_menu(app, ui, &response);

            // Scrollbars, document navigator, page tabs, Navigator button.
            window_bars::window_bars(app, ui, full, canvas_rect, ruler);

            if app.show_rulers {
                rulers::rulers(app, ui, full);
            }

            let cursor = match app.tool {
                crate::tools::Tool::Pan => egui::CursorIcon::Grab,
                crate::tools::Tool::Zoom => egui::CursorIcon::ZoomIn,
                crate::tools::Tool::Text => egui::CursorIcon::Text,
                crate::tools::Tool::Pick | crate::tools::Tool::FreeformPick
                    if app.settings.crosshair_cursor =>
                {
                    egui::CursorIcon::Crosshair
                }
                crate::tools::Tool::Pick | crate::tools::Tool::FreeformPick => {
                    egui::CursorIcon::Default
                }
                _ => egui::CursorIcon::Crosshair,
            };
            let cursor = if app.pending_palette_sample {
                egui::CursorIcon::Crosshair
            } else {
                cursor
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

/// The Navigator: the small button in the corner between the scrollbars
/// opens a thumbnail of the page while the button is held; moving the
/// pointer over the thumbnail pans the view to that spot, like the
/// target design's navigator pop-up.
pub(crate) fn view_navigator(app: &mut App, ui: &mut Ui, corner: egui::Rect, canvas: egui::Rect) {
    let resp = ui.interact(
        corner,
        egui::Id::new("view_navigator"),
        Sense::click_and_drag(),
    );
    let painter = ui.painter_at(corner);
    if resp.hovered() {
        painter.rect_filled(corner, 0.0, Tokens::TOOL_HOVER);
    }
    // A magnifier over a cross, as on the target design's button.
    let c = corner.center() + egui::vec2(-1.0, -1.0);
    let s = egui::Stroke::new(1.2, Tokens::TEXT_DIM);
    painter.circle_stroke(c, 4.5, s);
    painter.line_segment([c + egui::vec2(3.3, 3.3), c + egui::vec2(6.5, 6.5)], s);
    painter.line_segment([c + egui::vec2(-2.5, 0.0), c + egui::vec2(2.5, 0.0)], s);
    painter.line_segment([c + egui::vec2(0.0, -2.5), c + egui::vec2(0.0, 2.5)], s);
    let resp = resp.on_hover_text(crate::i18n::tr("status.navigator"));
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
        .fade_in(false)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::DockerTab;
    use crate::tools::Tool;
    use tracedraw_core::geometry::{Point, Rect};
    use tracedraw_core::ShapeKind;

    fn mixed_app() -> App {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        let r = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 2.0,
                corners: None,
            })
            .expect("rect");
        app.new_shape(ShapeKind::Ellipse {
            rect: Rect::new(70.0, 10.0, 120.0, 40.0),
            arc: None,
        });
        app.start_text(Point::new(20.0, 80.0), None);
        app.text_insert("Docker test");
        app.finish_text();
        app.select(vec![r]);
        app.duplicate();
        app.convert_to_bitmap(50.0, true);
        app.create_table(Rect::new(10.0, 120.0, 80.0, 160.0));
        app.select_all();
        app
    }

    fn frame(ctx: &egui::Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| root(app, ui));
        out.textures_delta.clear();
    }

    fn frame_with(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| root(app, ui));
        out.textures_delta.clear();
    }

    fn button(pos: egui::Pos2, pressed: bool, button: egui::PointerButton) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// A primary-button drag in screen space, one frame per step.
    fn drag(ctx: &egui::Context, app: &mut App, from: egui::Pos2, to: egui::Pos2) {
        frame_with(ctx, app, vec![egui::Event::PointerMoved(from)]);
        frame_with(
            ctx,
            app,
            vec![button(from, true, egui::PointerButton::Primary)],
        );
        for i in 1..=3 {
            let t = i as f32 / 3.0;
            let p = from + (to - from) * t;
            frame_with(ctx, app, vec![egui::Event::PointerMoved(p)]);
        }
        frame_with(
            ctx,
            app,
            vec![button(to, false, egui::PointerButton::Primary)],
        );
        frame_with(ctx, app, Vec::new());
    }

    fn click(ctx: &egui::Context, app: &mut App, at: egui::Pos2, b: egui::PointerButton) {
        frame_with(ctx, app, vec![egui::Event::PointerMoved(at)]);
        frame_with(ctx, app, vec![button(at, true, b)]);
        frame_with(ctx, app, vec![button(at, false, b)]);
        frame_with(ctx, app, Vec::new());
    }

    /// Every tool gets a drag across the page, a drag starting on an
    /// object, a click on an object, a click on empty space and a right
    /// click, in the real window with pointer events; nothing panics and
    /// the document keeps its page.
    #[test]
    fn every_tool_survives_pointer_input() {
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        app.show_welcome = false;
        frame(&ctx, &mut app);
        app.zoom_to_page();
        frame(&ctx, &mut app);
        // Where the first rectangle (10..60 x 10..40 mm) is on screen.
        let on_rect = app
            .view
            .to_screen(tracedraw_core::geometry::Point::new(35.0, 25.0));
        let empty = app
            .view
            .to_screen(tracedraw_core::geometry::Point::new(150.0, 250.0));
        let far = app
            .view
            .to_screen(tracedraw_core::geometry::Point::new(190.0, 200.0));
        for tool in Tool::ALL {
            app.set_tool(tool);
            app.select_all();
            drag(&ctx, &mut app, empty, far);
            drag(&ctx, &mut app, on_rect, far);
            click(&ctx, &mut app, on_rect, egui::PointerButton::Primary);
            click(&ctx, &mut app, empty, egui::PointerButton::Primary);
            click(&ctx, &mut app, on_rect, egui::PointerButton::Secondary);
            frame_with(
                &ctx,
                &mut app,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            app.dialog = crate::ui::dialogs::Dialog::None;
            app.finish_text();
            assert!(!app.doc().pages.is_empty(), "{tool:?}");
        }
        // Undo everything that was drawn.
        for _ in 0..200 {
            if app.engine.undo_label().is_none() {
                break;
            }
            app.undo();
        }
        frame(&ctx, &mut app);
    }

    fn key(ctx: &egui::Context, app: &mut App, key: egui::Key, modifiers: egui::Modifiers) {
        // Press and release inside one frame, with the modifiers released
        // too: what a fast typist or a test driver produces.
        frame_with(
            ctx,
            app,
            vec![
                egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                },
                egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }

    fn shape_count(app: &App) -> usize {
        app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum()
    }

    /// Shortcuts take the modifiers of each key press: Ctrl+Shift+Z redoes
    /// without also undoing, and a Ctrl+E released within the frame is the
    /// Export shortcut, not the plain E (align centres).
    #[test]
    fn shortcuts_use_the_modifiers_of_each_key_press() {
        use egui::{Key, Modifiers};
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        app.show_welcome = false;
        frame(&ctx, &mut app);
        let first = app.doc().pages[0].layers[0].shapes[0].id;
        app.select(vec![first]);
        let n0 = shape_count(&app);
        key(&ctx, &mut app, Key::D, Modifiers::COMMAND);
        assert_eq!(shape_count(&app), n0 + 1, "Ctrl+D duplicates");
        key(&ctx, &mut app, Key::Z, Modifiers::COMMAND);
        assert_eq!(shape_count(&app), n0, "Ctrl+Z undoes");
        key(
            &ctx,
            &mut app,
            Key::Z,
            Modifiers::COMMAND | Modifiers::SHIFT,
        );
        assert_eq!(shape_count(&app), n0 + 1, "Ctrl+Shift+Z redoes only");
        app.select(vec![first]);
        let before = app.selection_bounds();
        key(&ctx, &mut app, Key::E, Modifiers::COMMAND);
        assert!(
            matches!(app.dialog, crate::ui::dialogs::Dialog::Export(_)),
            "Ctrl+E opens Export"
        );
        assert_eq!(app.selection_bounds(), before, "and does not align");
        app.dialog = crate::ui::dialogs::Dialog::None;
        // Arrows nudge by the nudge distance, Shift by the super nudge and
        // Ctrl by the micro nudge distance.
        let b0 = app.selection_bounds().unwrap_or_default();
        key(&ctx, &mut app, Key::ArrowRight, Modifiers::NONE);
        key(&ctx, &mut app, Key::ArrowRight, Modifiers::SHIFT);
        key(&ctx, &mut app, Key::ArrowRight, Modifiers::COMMAND);
        let b1 = app.selection_bounds().unwrap_or_default();
        let moved = b1.x0 - b0.x0;
        let want = app.nudge_mm + app.settings.super_nudge_mm + app.settings.micro_nudge_mm;
        assert!((moved - want).abs() < 1e-6, "moved {moved}, want {want}");
    }

    /// The whole window draws with every docker tab, every tool and every
    /// kind of selection, including while a text is being edited.
    #[test]
    fn window_draws_with_every_docker_tool_and_selection() {
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        app.show_welcome = true;
        frame(&ctx, &mut app);
        app.show_welcome = false;
        app.show_dockers = true;
        for tab in DockerTab::ALL {
            app.docker_tab = tab;
            frame(&ctx, &mut app);
            frame(&ctx, &mut app);
        }
        app.docker_tab = DockerTab::Properties;
        for tool in Tool::ALL {
            app.set_tool(tool);
            frame(&ctx, &mut app);
        }
        app.set_tool(Tool::Pick);
        // One kind at a time: the property bar and dockers specialise.
        let ids: Vec<_> = app.doc().pages[0]
            .layers
            .iter()
            .flat_map(|l| l.shapes.iter().map(|s| s.id))
            .collect();
        for id in ids {
            app.select(vec![id]);
            frame(&ctx, &mut app);
            app.docker_tab = DockerTab::Objects;
            frame(&ctx, &mut app);
            app.docker_tab = DockerTab::Properties;
        }
        app.select(Vec::new());
        frame(&ctx, &mut app);
        // Editing text, with a selection inside it.
        app.edit_selected_text();
        if let Some(s) = app.text_shapes().first() {
            let id = s.id;
            app.begin_text_edit(id);
        }
        app.text_select_all();
        app.set_tool(Tool::Text);
        frame(&ctx, &mut app);
        app.finish_text();
        assert!(!app.doc().pages.is_empty());
    }
}
