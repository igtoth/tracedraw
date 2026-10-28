//! UI composition, laid out: menu bar, standard toolbar,
//! property bar, toolbox on the left, docker tab strip on the right edge,
//! document tabs, rulers and canvas in the middle, navigator and colour
//! palette under the canvas, status bar at the bottom.

pub mod about;
pub mod ai_bar;
pub mod ai_docker;
pub mod ai_generate_docker;
pub mod ai_ui;
pub mod align_docker;
pub mod bar_items;
pub mod barcode_wizard;
pub mod bars;
pub mod bitmap_dialogs;
pub mod bitmap_preview;
pub mod brand;
pub mod brush_bars;
pub mod callout_bar;
pub mod chrome;
pub mod clip_bar;
pub mod color_docker;
pub mod color_management;
pub mod color_picker;
pub mod comments_docker;
pub mod connector_bar;
pub mod context;
pub mod contour_bar;
pub mod coords_docker;
pub mod crop_bar;
pub mod cursors;
pub mod curve_edit;
pub mod defaults_dialog;
pub mod dialogs;
pub mod dimension_bar;
pub mod distort_bar;
pub mod doc_properties;
pub mod docker_form;
pub mod dockers;
pub mod dockers2;
pub mod drawing_bars;
pub mod edit_fill;
pub mod effect_bars;
pub mod effect_dialog;
pub mod effect_picker;
pub mod effects_docker;
pub mod envelope_ui;
pub mod export_docker;
pub mod extrude_bar;
pub mod field;
pub mod fill_tool_bars;
pub mod focus_bar;
pub mod font_list;
pub mod font_manager;
pub mod font_sampler;
pub mod fx_icons;
pub mod github_dialogs;
pub mod glyphs_docker;
pub mod hints;
pub mod icons;
pub mod lab_dialog;
pub mod layout_options;
pub mod line_style;
pub mod macros_ui;
pub mod menu_glyphs;
pub mod menu_pics;
pub mod menus;
pub mod objects_docker;
pub mod options;
pub mod options_global;
pub mod outline_pen;
pub mod overflow;
pub mod palette;
pub mod pdf_import_dialog;
pub mod pdf_password;
pub mod picture_mosaic_docker;
pub mod preflight_dialog;
pub mod preview;
pub mod print_merge_docker;
pub mod print_preview;
pub mod propbar;
pub mod publish_pdf;
pub mod rulers;
pub mod save_as;
pub mod shaping_docker;
pub mod shelf_docker;
pub mod status;
pub mod straighten_dialog;
pub mod symmetry_bar;
pub mod table_bar;
pub mod tabs;
pub mod template_dialog;
pub mod text_docker;
pub mod text_path_bar;
pub mod time_tracker;
#[cfg(test)]
mod tip_audit;
pub mod tips;
pub mod toolbar;
pub mod toolbox;
pub mod transform_docker;
pub mod vector_mosaic_docker;
pub mod welcome;
pub mod window_bars;
pub mod windows;
pub mod writing_tools;

use crate::app::App;
use crate::canvas;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Frame, Panel, Sense, Ui};
use tracedraw_core::geometry::Point;

/// Customization > Appearance > Interface size: the size of the menus,
/// bars and panels relative to the scaling the system gives the monitor
/// the window is on, as other programs size themselves. 1 (100 %) is the
/// system's size; 0 (no choice made) is 100 % too.
pub(crate) fn apply_ui_scale(ctx: &egui::Context, size: f32) {
    let zoom = ui_zoom(size);
    if (ctx.zoom_factor() - zoom).abs() > 0.001 {
        ctx.set_zoom_factor(zoom);
    }
}

/// Use the theme the preferences name, when it is not the one in use.
/// egui's own theme preference stays pinned, with the chosen theme's
/// visuals in both of its style slots, so the system's dark mode (the
/// browser's, in the browser build) never picks another style.
pub(crate) fn apply_theme(ctx: &egui::Context, id: &str) {
    let wanted = crate::theme::Theme::from_id(id);
    let pinned = ctx.options(|o| o.theme_preference) == egui::ThemePreference::Light;
    if wanted != crate::theme::theme() || !pinned {
        crate::theme::set_theme(wanted);
        ctx.set_theme(egui::ThemePreference::Light);
        ctx.set_visuals_of(egui::Theme::Dark, crate::theme::visuals());
        ctx.set_visuals(crate::theme::visuals());
    }
}

/// Whether a software adapter draws the window (#616): no shadows, the
/// system's pointers and paced frames then, as each frame is rasterized
/// on the CPU. Never in the browser build.
pub(crate) fn software_renderer() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::gpu::software()
    }
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
}

/// The status bar's note that a software adapter draws the window: with
/// hardware acceleration off in Options that is why, and a click on the
/// note opens Options > General to turn it on; with it on, the card's own
/// driver is missing.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn note_software_renderer(app: &mut App, name: &str) {
    if app.settings.hardware_acceleration {
        app.status = crate::i18n::trf("status.software_renderer", &[("name", name)]);
    } else {
        app.status = crate::i18n::trf("status.software_renderer_off", &[("name", name)]);
        app.status_opens_options = Some(app.status.clone());
    }
}

/// Shadows under menus and dialogs on or off, as the preferences say.
pub(crate) fn apply_shadows(ctx: &egui::Context, on: bool) {
    if on != crate::theme::shadows() {
        crate::theme::set_shadows(on);
        ctx.set_visuals(crate::theme::visuals());
    }
}

/// Interface sizes relative to the system's scaling, as the tests try.
#[cfg(test)]
pub(crate) const UI_SIZES: [f32; 8] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

/// The zoom of the interface at 100 %, on every system: one interface
/// point per pixel at the system's scaling, so the measures (13 px text,
/// 20 px icons, 24 px rows) come out as drawn. It was 0.8 on macOS and in
/// the browser and 12 / 13 on Windows, to make the whole interface as
/// compact as the reference drawing program; what made that program
/// compact was its small button size, which the bars have now (Default
/// Button Size, Small at first), so the text and the menus stay at the
/// system's size (#1, #229).
pub(crate) const BASE_ZOOM: f32 = 1.0;

/// The interface size `size` relative to the system's scaling (1 is
/// 100 %): `size` itself from 50 % to 400 %, else 1.
pub(crate) fn interface_size(size: f32) -> f32 {
    if (0.5..=4.0).contains(&size) {
        size
    } else {
        1.0
    }
}

/// The zoom factor of the interface size `size`: egui multiplies the
/// monitor's own scaling by it.
pub(crate) fn ui_zoom(size: f32) -> f32 {
    BASE_ZOOM * interface_size(size)
}

/// A slider whose rail shows on the white and light grey panels. egui
/// paints slider rails with the fill of framed controls, which the theme
/// makes white for check boxes and fields; here the rail gets a thin grey
/// groove instead.
pub struct Rail<'a>(pub egui::Slider<'a>);

impl egui::Widget for Rail<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        ui.scope(|ui| {
            ui.spacing_mut().slider_rail_height = 4.0;
            ui.visuals_mut().widgets.inactive.bg_fill = Tokens::slider_rail();
            ui.add(self.0)
        })
        .inner
    }
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
    root_frame(app, ui);
    // A fill or an outline dragged in a colour picker ends with the
    // button (after this frame's last change).
    if !ui.ctx().input(|i| i.pointer.any_down()) {
        app.end_live_fill();
        app.end_live_stroke();
    }
}

fn root_frame(app: &mut App, ui: &mut Ui) {
    // The rest of a notch a number field took scrolls nothing (#783).
    crate::ui::field::hold_taken_wheel(ui.ctx());
    // Dockers set their canvas previews again each frame they are shown;
    // the document palette says where it is when it is drawn.
    app.docker_preview.clear();
    app.doc_palette_rect = None;
    app.docker_preview_point = None;
    let ctx = ui.ctx().clone();
    // Pixels are measured at the active drawing's resolution, distances
    // at its drawing scale.
    crate::app::set_pixel_dpi(app.document_dpi());
    crate::app::set_drawing_scale(app.doc().metadata.rulers.world_per_page());
    apply_ui_scale(&ctx, crate::settings::effective_ui_size(&app.settings));
    crate::theme::set_button_sizes(app.settings.button_size, app.settings.toolbox_button_size);
    apply_theme(&ctx, &crate::settings::effective_theme(&app.settings));
    apply_shadows(&ctx, app.settings.ui_shadows && !software_renderer());
    // Said once in the status bar: a software adapter draws the window.
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(name) = crate::gpu::take_software_note() {
        note_software_renderer(app, &name);
    }
    crate::ui::icons::set_colored(app.settings.colored_icons);
    menus::invoke::set_bindings(&app.settings.shortcuts);
    // F1: the Help page of the open dialog, the docker or the tool.
    if app.shortcut_editor.capture.is_none() {
        crate::help::handle_f1(app, &ctx);
    }
    // Clones take up what the last frame changed on their masters.
    app.sync_object_clones();
    app.keyboard(&ctx);
    // A menu command run by a shortcut of the user's.
    if let Some(cmd) = app.pending_command.take() {
        if !menus::run_command(app, &ctx, &cmd) {
            app.status = crate::i18n::trf("status.command_unavailable", &[("name", &tr(&cmd))]);
        }
    }

    let doc_name = if app.has_document() && !app.show_welcome {
        app.document_tab_title(app.active_doc)
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
    // With no drawing open the Welcome Screen shows, unless its tab was
    // closed: then the workspace stays empty (#1081).
    if !app.has_document() && !app.settings.welcome_tab_closed {
        app.show_welcome = true;
    }
    // Files dropped on the window, and what a drop would do while they
    // are dragged over it.
    app.take_dropped_files(&ctx);
    app.draw_drop_hint(&ctx);
    // Options > Display > Show tooltips.
    let delay = if app.settings.show_tooltips {
        0.5
    } else {
        f32::INFINITY
    };
    if ctx.global_style().interaction.tooltip_delay != delay {
        ctx.all_styles_mut(|s| s.interaction.tooltip_delay = delay);
    }
    // Session recovery: this session's marker, and what one that ended
    // without closing left.
    app.session_tick(crate::recovery::now_secs());
    if app.debug_tick() {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    // VBA macros: their requests, dialogs, forms and editor.
    macros_ui::frame(app, &ctx);
    // A print on its way to a printer (#810).
    if app.poll_print_job() {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
    // Help > Updates, once a day.
    app.update_tick(crate::recovery::now_secs(), Some(ctx.clone()));
    // Options > Save > Auto-backup.
    if app.auto_backup_tick(web_time::Instant::now()) {
        app.status = tr("status.auto_backup_saved");
    }
    // The Time Tracker's clock: while it counts, a frame a second keeps it
    // going (and its display current).
    let (now, input) = ctx.input(|i| (i.time, crate::time_tracker::is_user_input(&i.events)));
    if app.time_tracker_frame(now, input) {
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }

    if app.fullscreen_preview {
        preview::fullscreen(app, ui);
        return;
    }
    if app.print_preview.is_some() {
        print_preview::show(app, ui);
        return;
    }

    Panel::top("menu_bar")
        .frame(
            Frame::new()
                .fill(Tokens::menu_bar())
                .inner_margin(egui::Margin::symmetric(0, 1)),
        )
        .exact_size(24.0)
        .resizable(false)
        .show_separator_line(false)
        .show(ui, |ui| menus::menu_bar(app, ui));
    // The bars, where the layout puts them (see bars.rs). Outer panels
    // come first: the status bar (at the very bottom, or right under the
    // menu bar), then the toolbars, then the palettes and the toolbox.
    let bars_layout = app.settings.bars.clone();
    use bars::{Bar, Dock};
    let side = |panel: Panel, size: f32| {
        panel
            .exact_size(size)
            .resizable(false)
            .show_separator_line(false)
    };
    let status_dock = bars_layout.dock(Bar::Status);
    let status_panel = |app: &mut App, ui: &mut Ui, panel: Panel| {
        panel
            .frame(Frame::new().fill(Tokens::panel()))
            .exact_size(status::BAR_H)
            .show_separator_line(false)
            .show(ui, |ui| {
                bars::background(app, ui, Bar::Status);
                status::status_bar(app, ui)
            });
    };
    if app.show_status_bar && status_dock == Dock::Top {
        status_panel(app, ui, Panel::top("status_bar"));
    }
    // The toolbars along the top, in rows (a 36 px band for the standard
    // toolbar, 53 px for the property bar, each closed by a light line),
    // several side by side in a row where they were put (#837, #855).
    app.bar_rows.clear();
    bars::docked_rows(app, ui, Dock::Top);
    if app.show_status_bar && status_dock == Dock::Bottom {
        status_panel(app, ui, Panel::bottom("status_bar"));
    }
    bars::docked_rows(app, ui, Dock::Bottom);

    // The default palette as a column on a side lies outermost (outside
    // the dockers); as a row (at the bottom by default) it comes after the
    // dockers (below). The document palette is a row under the drawing
    // window (over it when the palette is docked at the top).
    let palette_frame = Frame::new().fill(Tokens::panel());
    let palette_dock = bars_layout.dock(Bar::Palette);
    let show_palette = app.settings.palette.show_palette;
    let show_doc_palette = show_palette && app.settings.palette.show_document;
    if matches!(palette_dock, Dock::Left | Dock::Right) {
        let panel = |id: &'static str| {
            if palette_dock == Dock::Left {
                Panel::left(id)
            } else {
                Panel::right(id)
            }
        };
        if show_palette {
            side(
                panel("palette"),
                palette::thickness(app, palette::Row::Main),
            )
            .frame(palette_frame)
            .show(ui, |ui| palette::palette_column(app, ui));
        }
    }

    // The toolbox: a column on a side, a row along the top or the bottom.
    if app.show_toolbox {
        let dock = bars_layout.dock(Bar::Toolbox);
        let frame = Frame::new()
            .fill(Tokens::panel())
            .inner_margin(if dock.horizontal() {
                egui::Margin::symmetric(0, 3)
            } else {
                egui::Margin::symmetric(3, 0)
            });
        // A row along the top or the bottom comes after the dockers.
        let panel = match dock {
            Dock::Left => Some(Panel::left("toolbox")),
            Dock::Right => Some(Panel::right("toolbox")),
            _ => None,
        };
        if let Some(panel) = panel {
            panel
                .exact_size(toolbox::column_width(app))
                .resizable(false)
                .frame(frame)
                .show(ui, |ui| toolbox::toolbox(app, ui, dock));
        }
    }

    // The docker columns: the default one at the right (right-to-left
    // languages mirror the workspace: it sits on the left), the other at
    // the other side, each a tab strip and, when open, the docker in
    // front. The other column exists only while it holds a docker.
    app.reconcile_dockers();
    let rtl = crate::i18n::is_rtl();
    use crate::app::DockerPlace;
    for place in [DockerPlace::Right, DockerPlace::Left] {
        if place == DockerPlace::Left && app.left_dockers.is_empty() {
            continue;
        }
        let at_left = (place == DockerPlace::Left) != rtl;
        let id = if place == DockerPlace::Left {
            ("docker_tabs_left", "dockers_left")
        } else {
            ("docker_tabs", "dockers")
        };
        let docker_tabs = if at_left {
            Panel::left(id.0)
        } else {
            Panel::right(id.0)
        };
        docker_tabs
            .exact_size(dockers::STRIP_W)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(crate::theme::Tokens::tab_strip()))
            .show(ui, |ui| dockers::tab_strip(app, ui, place));
        let (_, _, shown) = app.docker_column(place);
        if shown {
            let dockers_panel = if at_left {
                Panel::left(id.1)
            } else {
                Panel::right(id.1)
            };
            dockers_panel
                .default_size(340.0)
                .resizable(true)
                .frame(Frame::new().fill(crate::theme::Tokens::field()))
                .show(ui, |ui| dockers::dockers(app, ui, place));
        }
    }

    if app.show_toolbox && bars_layout.dock(Bar::Toolbox).horizontal() {
        let dock = bars_layout.dock(Bar::Toolbox);
        let panel = if dock == Dock::Top {
            Panel::top("toolbox")
        } else {
            Panel::bottom("toolbox")
        };
        panel
            .exact_size(Tokens::tool_button() + 6.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(Tokens::panel())
                    .inner_margin(egui::Margin::symmetric(0, 3)),
            )
            .show(ui, |ui| toolbox::toolbox(app, ui, dock));
    }
    // The palettes as rows under (or over) the drawing window; the
    // document palette is always a row.
    {
        let panel = |id: &'static str| {
            if palette_dock == Dock::Top {
                Panel::top(id)
            } else {
                Panel::bottom(id)
            }
        };
        if show_doc_palette {
            side(panel("document_palette"), palette::ROW_H)
                .frame(palette_frame)
                .show(ui, |ui| palette::document_palette_row(app, ui));
        }
        if show_palette && matches!(palette_dock, Dock::Bottom | Dock::Top) {
            side(
                panel("palette"),
                palette::thickness(app, palette::Row::Main),
            )
            .frame(palette_frame)
            .show(ui, |ui| palette::palette_row(app, ui));
        }
    }
    // The AI prompt bar, over the palette rows, while a service is set up.
    if ai_bar::visible(app) {
        side(Panel::bottom("ai_prompt_bar"), ai_bar::BAR_H)
            .frame(
                Frame::new()
                    .fill(Tokens::panel())
                    .inner_margin(egui::Margin::symmetric(0, 2)),
            )
            .show_separator_line(true)
            .show(ui, |ui| ai_bar::ai_bar(app, ui));
    }
    palette::popups(app, &ctx);

    // Floating bars over the window, and a bar being moved; the floating
    // dockers and a docker being moved likewise.
    bars::floating_bars(app, &ctx);
    bars::drag_overlay(app, &ctx, 24.0);
    bar_items::drop_overlay(app, &ctx);
    dockers::floating_dockers(app, &ctx);
    dockers::drag_overlay(app, &ctx, 24.0);

    // Document tabs above the rulers.
    // The tabs' blue line is the strip's lower edge: no separator and no
    // gap between it and the rulers.
    Panel::top("document_tabs")
        .frame(Frame::new().fill(crate::theme::Tokens::tab_strip()))
        .show_separator_line(false)
        .show(ui, |ui| tabs::document_tabs(app, ui));

    if app.show_welcome {
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Tokens::panel()))
            .show(ui, |ui| welcome::welcome_screen(app, ui));
        dialogs::show(app, &ctx);
        return;
    }
    // The Welcome Screen closed and no drawing open: an empty workspace.
    if !app.has_document() {
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Tokens::panel()))
            .show(ui, |_| {});
        dialogs::show(app, &ctx);
        return;
    }
    if app.page_sorter && !app.multipage {
        egui::CentralPanel::default()
            .frame(Frame::new().fill(Tokens::panel()))
            .show(ui, |ui| preview::page_sorter(app, ui));
        dialogs::show(app, &ctx);
        return;
    }

    egui::CentralPanel::default()
        .frame(Frame::NONE)
        .show(ui, |ui| {
            let full = ui.available_rect_before_wrap();
            if app.window_layout == windows::WindowLayout::Tabs || app.docs.is_empty() {
                drawing_window(app, ui, full);
            } else {
                windows::arranged(app, ui, full);
            }
        });

    if app.has_document() && !app.show_welcome {
        symmetry_bar::show(app, &ctx);
        clip_bar::show(app, &ctx);
        focus_bar::show(app, &ctx);
        effect_picker::show(app, &ctx);
        crate::perspective_draw::show_bar(app, &ctx);
    }
    dialogs::show(app, &ctx);
    // A change this frame made to a master shows on its clones at once.
    let before = app.engine.revision();
    app.sync_object_clones();
    if app.engine.revision() != before {
        ctx.request_repaint();
    }
}

/// Raw wheel units one notch of a mouse wheel sends. The desktop
/// backends report a notch as one line (or 50 points from a
/// trackpad); browsers report it as three lines or as 100 pixels, so
/// read as desktop notches a browser notch zoomed three or two notches
/// at once (#1016).
fn wheel_notch(unit: egui::MouseWheelUnit, web: bool) -> f32 {
    match (unit, web) {
        (egui::MouseWheelUnit::Line, false) => 1.0,
        (egui::MouseWheelUnit::Line, true) => 3.0,
        (egui::MouseWheelUnit::Point, false) => 50.0,
        (egui::MouseWheelUnit::Point, true) => 100.0,
        (egui::MouseWheelUnit::Page, _) => 0.1,
    }
}

/// Points of egui's smoothed scroll delta that one wheel notch in
/// `unit` adds up to: egui turns a line into `line_speed` points and a
/// page into the window's height. On the desktop a notch stays at
/// `line_speed` points whatever the unit, as before; in the browser
/// it is the browser's notch in points (#1016).
fn smooth_wheel_notch(unit: egui::MouseWheelUnit, line_speed: f32, height: f32, web: bool) -> f32 {
    let points = match (unit, web) {
        (_, false) => line_speed,
        (egui::MouseWheelUnit::Line, true) => wheel_notch(unit, web) * line_speed,
        (egui::MouseWheelUnit::Point, true) => wheel_notch(unit, web),
        (egui::MouseWheelUnit::Page, true) => wheel_notch(unit, web) * height,
    };
    points.max(1.0)
}

/// The active drawing's window in `full`: rulers, the canvas with its
/// tools, scrollbars, the page tabs and the navigator.
pub(crate) fn drawing_window(app: &mut App, ui: &mut Ui, full: egui::Rect) {
    let ruler = if app.show_rulers { Tokens::RULER } else { 0.0 };
    let sb = window_bars::BAR;
    let canvas_rect = egui::Rect::from_min_max(
        full.min + egui::vec2(ruler, ruler),
        full.max - egui::vec2(sb, sb),
    );
    app.canvas_rect = canvas_rect;
    app.anchor_view(canvas_rect.min);

    if app.fit_pending {
        app.view.fit(app.fit_rect(), canvas_rect);
        app.fit_pending = false;
    }

    let response = ui.allocate_rect(canvas_rect, Sense::click_and_drag());
    let painter = ui.painter_at(canvas_rect);

    // Wheel: with the default setting the wheel zooms about the
    // pointer, Ctrl+wheel scrolls vertically and Alt+wheel
    // horizontally; with the scroll setting the wheel scrolls,
    // Shift+wheel horizontally and Ctrl+wheel zooms. Pinch zooms.
    if response.hovered() {
        let (scroll, notches, zoom_delta, mods, unit) = ui.input(|i| {
            // Wheel notches this frame (the smoothed delta is spread
            // over several frames, so the zoom steps come from the
            // raw events).
            let mut unit = None;
            let notches: f32 = i
                .events
                .iter()
                .map(|e| match e {
                    egui::Event::MouseWheel { unit: u, delta, .. } => {
                        unit = Some(*u);
                        delta.y / wheel_notch(*u, cfg!(target_arch = "wasm32"))
                    }
                    _ => 0.0,
                })
                .sum();
            (
                i.smooth_scroll_delta,
                notches,
                i.zoom_delta(),
                i.modifiers,
                unit,
            )
        });
        // The unit of the last wheel event, kept while its smoothed
        // delta is still being spread over the next frames.
        let unit_id = egui::Id::new("canvas_wheel_unit");
        let unit = match unit {
            Some(u) => {
                ui.data_mut(|d| d.insert_temp(unit_id, u));
                u
            }
            None => ui
                .data(|d| d.get_temp(unit_id))
                .unwrap_or(egui::MouseWheelUnit::Line),
        };
        if let Some(h) = response.hover_pos() {
            let zoom_factor = 1.15f32.powf(notches);
            if zoom_delta != 1.0 {
                app.view.zoom_at(h, zoom_delta);
            } else if app.settings.wheel_zooms {
                if mods.alt && scroll != egui::Vec2::ZERO {
                    app.view.pan(egui::vec2(scroll.y, scroll.x));
                } else if mods.ctrl && scroll != egui::Vec2::ZERO {
                    app.view.pan(scroll);
                } else if scroll.y != 0.0 {
                    // The smoothed wheel delta, spread over a few frames
                    // as a scroll is, so a notch zooms in one glide
                    // instead of one jump; its frames multiply to the same
                    // 1.15 a notch (#766), in the browser too (#1016).
                    let line = ui.ctx().options(|o| o.input_options.line_scroll_speed);
                    let per_notch = smooth_wheel_notch(
                        unit,
                        line,
                        ui.ctx().content_rect().height(),
                        cfg!(target_arch = "wasm32"),
                    );
                    app.view.zoom_at(h, 1.15f32.powf(scroll.y / per_notch));
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

    // Two fingers on a touch screen pan (and pinch zooms, above):
    // what the first finger began is dropped, so the gesture does
    // not also drag a marquee or an object.
    let two_fingers = ui.input(|i| i.multi_touch());
    if let Some(t) = two_fingers {
        if response.rect.contains(t.center_pos) {
            app.view.pan(t.translation_delta);
        }
        if !matches!(app.drag, crate::app::Drag::None) {
            app.drag = crate::app::Drag::None;
            app.drag_copy = false;
        }
    }

    // A guideline or the ruler origin dragged out of the rulers
    // follows the pointer until release.
    if app.multipage && two_fingers.is_none() {
        let press = ui.input(|i| {
            i.pointer
                .primary_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        if let Some(s) = press.filter(|s| canvas_rect.contains(*s)) {
            app.multipage_press(s);
        }
    }
    if two_fingers.is_none()
        && !rulers::ruler_drag(app, ui)
        && !app.comments_pointer(&response)
        && !app.perspective_pointer(&response)
    {
        app.canvas_input(&response, mods);
    }
    app.perspective_tick();

    canvas::draw_canvas(app, &painter, canvas_rect);
    canvas::draw_guides(app, &painter, canvas_rect);
    rulers::draw_origin_drag(app, &painter, canvas_rect);
    canvas::draw_effect_nodes(app, &painter);
    envelope_ui::draw_envelopes(app, &painter);
    dimension_bar::draw_preview(app, &painter);
    callout_bar::draw_preview(app, &painter);
    distort_bar::draw_centers(app, &painter);
    extrude_bar::draw_handles(app, &painter);
    crop_bar::draw_area(app, &painter, canvas_rect);
    canvas::draw_docker_preview(app, &painter);
    canvas::draw_live_guides(app, &painter, canvas_rect);
    canvas::draw_snap_mark(app, &painter);
    app.draw_right_drag(&painter);
    context::context_menu(app, ui, &response);
    crate::drop_menu::drop_menu_ui(app, ui);
    crate::fill_tool::node_bar(app, ui);

    // Scrollbars, document navigator, page tabs, Navigator button.
    window_bars::window_bars(app, ui, full, canvas_rect, ruler);

    if app.show_rulers {
        rulers::rulers(app, ui, full);
    }

    // A right click during a move ends the windowing layer's drag (the
    // move goes on while the left button is held): the pointer keeps
    // being drawn until the move ends.
    if response.hovered() || response.dragged() || app.transforming() {
        let (shift, primary, middle, pos) = ui.input(|i| {
            (
                i.modifiers.shift,
                i.pointer.primary_down(),
                i.pointer.middle_down(),
                i.pointer.hover_pos().or(i.pointer.latest_pos()),
            )
        });
        let cursor = cursors::for_tool(app, shift, primary, middle, pos);
        cursors::show(ui.ctx(), cursor, pos);
        // A scale, turn or slant that leaves the original (after a right
        // click): the plus a move's pointer carries, beside the pointer.
        if app.drag_copy && app.transforming() && !matches!(app.drag, crate::app::Drag::Move { .. })
        {
            if let Some(p) = pos {
                cursors::paint_copy_plus(ui.ctx(), p);
            }
        }
    }
}

/// The Navigator: the small button in the corner between the scrollbars
/// opens a thumbnail of the page while the button is held; moving the
/// pointer over the thumbnail pans the view to that spot.
pub(crate) fn view_navigator(app: &mut App, ui: &mut Ui, corner: egui::Rect, canvas: egui::Rect) {
    let resp = ui.interact(
        corner,
        egui::Id::new("view_navigator"),
        Sense::click_and_drag(),
    );
    let painter = ui.painter_at(corner);
    if resp.hovered() {
        painter.rect_filled(corner, 0.0, Tokens::tool_hover());
    }
    // A magnifier over a cross.
    let c = corner.center() + egui::vec2(-1.0, -1.0);
    let s = egui::Stroke::new(1.2, Tokens::text_dim());
    painter.circle_stroke(c, 4.5, s);
    painter.line_segment([c + egui::vec2(3.3, 3.3), c + egui::vec2(6.5, 6.5)], s);
    painter.line_segment([c + egui::vec2(-2.5, 0.0), c + egui::vec2(2.5, 0.0)], s);
    painter.line_segment([c + egui::vec2(0.0, -2.5), c + egui::vec2(0.0, 2.5)], s);
    let resp = resp.rich_tip(crate::i18n::tr("status.navigator"));
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
    /// One browser wheel notch (100 pixels, or three lines at egui's
    /// web line speed of 8 points) zooms 15 %, as a desktop notch
    /// does; it zoomed about 5.7 times or 52 % at once (#1016).
    #[test]
    fn a_browser_wheel_notch_zooms_as_a_desktop_notch() {
        use egui::MouseWheelUnit::{Line, Page, Point};
        let zoom = |smoothed: f32, unit, line, web| {
            1.15f32.powf(smoothed / super::smooth_wheel_notch(unit, line, 800.0, web))
        };
        // The browser: Chrome, Edge and Safari send 100 pixels a notch,
        // Firefox three lines.
        assert!((zoom(100.0, Point, 8.0, true) - 1.15).abs() < 1e-4);
        assert!((zoom(3.0 * 8.0, Line, 8.0, true) - 1.15).abs() < 1e-4);
        assert!((zoom(-100.0, Point, 8.0, true) - 1.0 / 1.15).abs() < 1e-4);
        assert!((zoom(0.1 * 800.0, Page, 8.0, true) - 1.15).abs() < 1e-4);
        // The desktop keeps one line of 40 points a notch.
        assert!((zoom(40.0, Line, 40.0, false) - 1.15).abs() < 1e-4);
        assert!((zoom(40.0, Point, 40.0, false) - 1.15).abs() < 1e-4);
        // The raw notches of the scroll setting's Ctrl+wheel: one each.
        for (unit, raw) in [(Line, 3.0), (Point, 100.0)] {
            assert_eq!(raw / super::wheel_notch(unit, true), 1.0);
        }
        assert_eq!(1.0 / super::wheel_notch(Line, false), 1.0);
        assert_eq!(50.0 / super::wheel_notch(Point, false), 1.0);
        // A zero line speed never divides by zero.
        assert!(super::smooth_wheel_notch(Line, 0.0, 800.0, false) >= 1.0);
    }

    #[test]
    fn interface_size_is_relative_to_the_system_scaling() {
        // No choice made, or 100 %: the system's size, whatever the
        // monitor's scaling.
        assert_eq!(interface_size(0.0), 1.0);
        assert_eq!(interface_size(1.0), 1.0);
        assert!(UI_SIZES
            .iter()
            .all(|s| (interface_size(*s) - s).abs() < 1e-6));
        // 100 %: one interface point per pixel at the system's scaling,
        // on every system.
        let base = 1.0;
        assert_eq!(BASE_ZOOM, base);
        assert!((ui_zoom(0.0) - base).abs() < 1e-6);
        assert!((ui_zoom(1.0) - base).abs() < 1e-6);
        // 125 % is a quarter larger than 100 %.
        assert!((ui_zoom(1.25) - 1.25 * base).abs() < 1e-6);
        assert!((ui_zoom(2.0) - 2.0 * ui_zoom(1.0)).abs() < 1e-6);
        // Nonsense falls back to the system's size.
        for bad in [9.0, 0.1, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(interface_size(bad), 1.0, "{bad}");
            assert_eq!(ui_zoom(bad), BASE_ZOOM, "{bad}");
        }
    }

    /// The Touch workspace draws the interface larger and keeps drawings
    /// at their real size at zoom 100 %; the other workspaces leave the
    /// size as chosen.
    #[test]
    fn the_touch_workspace_is_larger_and_keeps_real_sizes() {
        let mut app = App::headless();
        app.settings.ui_size = 1.0;
        let ppi = crate::settings::screen_points_per_inch(&app.settings);
        app.set_workspace(crate::app::Workspace::Touch);
        assert_eq!(app.settings.workspace, "touch");
        let s = crate::settings::effective_ui_size(&app.settings);
        assert!((s - crate::settings::TOUCH_SCALE).abs() < 1e-6);
        // Bigger points, so fewer of them per inch: the same inch on screen.
        let touch_ppi = crate::settings::screen_points_per_inch(&app.settings);
        assert!((touch_ppi * crate::settings::TOUCH_SCALE - ppi).abs() < 1e-3);
        assert!(!app.show_rulers && !app.show_dockers && !app.show_status_bar);
        assert!(app.show_toolbox && app.show_property_bar);
        // At most 400 %.
        app.settings.ui_size = 3.5;
        assert!((crate::settings::effective_ui_size(&app.settings) - 4.0).abs() < 1e-6);
        app.set_workspace(crate::app::Workspace::Default);
        assert!((crate::settings::effective_ui_size(&app.settings) - 3.5).abs() < 1e-6);
        assert!(crate::app::Workspace::ALL
            .iter()
            .all(|w| crate::app::Workspace::from_id(w.id()) == *w));
    }

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

    /// Every tip the bars give, for every tool with nothing, each object
    /// and everything selected, written to `$TIPS_OUT` (one per line).
    #[test]
    #[ignore]
    fn collect_bar_tips() {
        let Some(out) = std::env::var_os("TIPS_OUT") else {
            return;
        };
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        app.show_welcome = false;
        frame(&ctx, &mut app);
        app.select_all();
        let ids = app.selection.clone();
        crate::ui::tips::start_recording();
        for tool in Tool::ALL {
            app.set_tool(tool);
            let mut sels = vec![Vec::new(), ids.clone()];
            sels.extend(ids.iter().map(|i| vec![*i]));
            for sel in sels {
                app.select(sel);
                frame(&ctx, &mut app);
                frame(&ctx, &mut app);
            }
        }
        let tips = crate::ui::tips::recorded();
        let text: Vec<String> = tips.iter().map(|t| t.replace('\n', "\\n")).collect();
        std::fs::write(out, text.join("\n")).unwrap();
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
        // Esc with a menu open closes the menu and keeps the selection;
        // with nothing open it deselects.
        egui::Popup::open_id(&ctx, egui::Id::new("some_menu"));
        key(&ctx, &mut app, Key::Escape, Modifiers::NONE);
        assert!(!app.selection.is_empty(), "Esc closed a menu only");
        egui::Popup::close_all(&ctx);
        key(&ctx, &mut app, Key::Escape, Modifiers::NONE);
        assert!(app.selection.is_empty());
    }

    /// The browser reports its dark mode as the system theme: the
    /// interface keeps the chosen theme (Light at first) all the same.
    #[test]
    fn the_system_dark_mode_does_not_change_the_theme() {
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        app.show_welcome = false;
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                system_theme: Some(egui::Theme::Dark),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(&mut app, ui));
            out.textures_delta.clear();
        }
        assert_eq!(ctx.theme(), egui::Theme::Light);
        assert!(!ctx.global_style().visuals.dark_mode);
        assert_eq!(crate::theme::theme(), crate::theme::Theme::Light);
        assert_eq!(
            ctx.global_style().visuals.panel_fill,
            crate::theme::visuals().panel_fill
        );
    }

    /// With Interactive Fill and a colour node picked, the node bar opens
    /// under the node.
    #[test]
    fn the_node_bar_opens_under_a_picked_fill_node() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(tracedraw_core::Fill::linear(
            tracedraw_core::Color::BLACK,
            tracedraw_core::Color::WHITE,
            0.0,
        ));
        app.tool = crate::tools::Tool::InteractiveFill;
        frame(&ctx, &mut app);
        let bar = egui::Id::new("fill_node_bar");
        assert!(ctx.memory(|m| m.area_rect(bar)).is_none());
        app.fill_node = Some(0);
        frame(&ctx, &mut app);
        frame(&ctx, &mut app);
        let r = ctx.memory(|m| m.area_rect(bar)).expect("node bar");
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let start = crate::fill_tool::fountain_handles(&f, b)[0].1;
        let at = app.view.to_screen(start);
        assert!(
            r.top() > at.y && (r.left() - at.x).abs() < 20.0,
            "{r:?} {at:?}"
        );
    }

    /// The window draws with the toolbox at every button size and with
    /// captions, docked on a side, along the top and floating; the
    /// measured captions widen the toolbox column.
    #[test]
    fn the_toolbox_draws_at_every_size_and_with_captions() {
        use crate::settings::{ButtonSize, Captions};
        use crate::ui::bars::{Bar, Dock};
        let ctx = crate::theme::ui_context();
        let mut app = mixed_app();
        app.show_welcome = false;
        let plain = {
            frame(&ctx, &mut app);
            crate::ui::toolbox::column_width(&app)
        };
        for size in ButtonSize::ALL {
            app.settings.button_size = size;
            for own in [None, Some(ButtonSize::Small)] {
                app.settings.toolbox_button_size = own;
                for captions in Captions::ALL {
                    app.settings.toolbox_captions = captions;
                    for dock in [Dock::Left, Dock::Top, Dock::Floating] {
                        crate::ui::bars::move_bar(&mut app, Bar::Toolbox, dock, None);
                        frame(&ctx, &mut app);
                        frame(&ctx, &mut app);
                    }
                }
            }
        }
        app.settings.button_size = ButtonSize::Medium;
        app.settings.toolbox_button_size = None;
        app.settings.toolbox_captions = Captions::CaptionRight;
        crate::ui::bars::move_bar(&mut app, Bar::Toolbox, Dock::Left, None);
        frame(&ctx, &mut app);
        frame(&ctx, &mut app);
        assert!(app.toolbox_caption_w > 30.0, "{}", app.toolbox_caption_w);
        assert!(crate::ui::toolbox::column_width(&app) > plain + 30.0);
        // Back to the default size the plain width was measured at.
        app.settings.button_size = ButtonSize::default();
        crate::ui::toolbox::reset_toolbox(&mut app);
        frame(&ctx, &mut app);
        assert_eq!(crate::ui::toolbox::column_width(&app), plain);
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

    /// The property bar has a button beside the font size that opens the
    /// axis sliders of the selected text's variable font (#870): a click
    /// on a slider sets the axis on the text, a click outside closes it,
    /// and for a static font the button is there but opens nothing.
    #[test]
    fn the_font_size_has_a_button_for_variable_font_axes() {
        use crate::ui::text_docker::{font_axes_button_rect, font_axes_popup_rect};
        use tracedraw_core::{ParagraphStyle, TextAlign, TextSpan};
        let quicksand = include_bytes!("../../../../crates/text/testdata/Quicksand-VF.ttf");
        let _ = tracedraw_text::fonts().add_fonts(vec![quicksand.to_vec()]);
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        let text = |app: &mut App, family: &str, y: f64| {
            app.new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Text", family, 24.0)],
                origin: Point::new(20.0, y),
                frame: None,
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text")
        };
        let variable = text(&mut app, "Quicksand", 40.0);
        let fixed = text(&mut app, "Selawik", 80.0);
        let wght = |app: &App| match app.doc().find_shape(variable).map(|s| s.kind.clone()) {
            Some(ShapeKind::Text { spans, .. }) => spans[0].variation("wght"),
            _ => None,
        };
        app.set_tool(Tool::Pick);
        app.select(vec![variable]);
        frame(&ctx, &mut app);
        frame(&ctx, &mut app);
        let button = font_axes_button_rect(&ctx, "propbar").expect("the button is on the bar");
        assert!(!egui::Popup::is_any_open(&ctx));
        click(
            &ctx,
            &mut app,
            button.center(),
            egui::PointerButton::Primary,
        );
        assert!(egui::Popup::is_any_open(&ctx), "the sliders open");
        let popup = font_axes_popup_rect(&ctx, "propbar").expect("the sliders show");
        assert!(popup.top() >= button.bottom() - 1.0, "under the button");
        // A click on the weight slider sets the weight on the text, one
        // undo step, with the sliders still open.
        let history = app.engine.history_labels().0.len();
        let x = popup.right() - 100.0;
        let mut y = popup.top() + 4.0;
        while wght(&app).is_none() && y < popup.bottom() {
            click(
                &ctx,
                &mut app,
                egui::pos2(x, y),
                egui::PointerButton::Primary,
            );
            y += 4.0;
        }
        assert!(wght(&app).is_some(), "a click on the slider sets the axis");
        assert!(egui::Popup::is_any_open(&ctx), "the sliders stay open");
        assert_eq!(app.engine.history_labels().0.len(), history + 1);
        // A click outside closes them.
        click(
            &ctx,
            &mut app,
            egui::pos2(1500.0, 900.0),
            egui::PointerButton::Primary,
        );
        assert!(!egui::Popup::is_any_open(&ctx));
        // A static font: the button is there and opens nothing.
        app.select(vec![fixed]);
        frame(&ctx, &mut app);
        let button = font_axes_button_rect(&ctx, "propbar").expect("still on the bar");
        click(
            &ctx,
            &mut app,
            button.center(),
            egui::PointerButton::Primary,
        );
        assert!(
            !egui::Popup::is_any_open(&ctx),
            "disabled for a static font"
        );
        // The Text docker has the same button beside its font size.
        app.select(vec![variable]);
        app.docker_tab = DockerTab::Text;
        frame(&ctx, &mut app);
        frame(&ctx, &mut app);
        assert!(
            font_axes_button_rect(&ctx, "docker").is_some(),
            "in the Text docker"
        );
    }
}
