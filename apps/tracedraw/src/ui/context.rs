//! Right-click context menus: object menu (Pick tool), node menu (Shape
//! tool), page menu (empty area), text menu.

use crate::app::{App, DockerTab};
use crate::i18n::tr;
use crate::tools::Tool;
use crate::ui::dialogs::Dialog;
use crate::ui::menus::{body, check, item, item_label, menu_popup_style, radio, sep, sub};
use egui::{Response, Ui};
use tracedraw_core::{document::ShapeKind, Command, ShapeId};

fn ci(ui: &mut Ui, close: &mut bool, key: &str, shortcut: &str, enabled: bool) -> bool {
    let r = item(ui, key, shortcut, enabled);
    if r {
        *close = true;
    }
    r
}

/// [`ci`] with a hover tip for the greyed row.
fn ci_tip(ui: &mut Ui, close: &mut bool, key: &str, enabled: bool, tip: Option<&str>) -> bool {
    let r = crate::ui::menus::item_tip(ui, key, "", enabled, tip);
    if r {
        *close = true;
    }
    r
}

/// The canvas context menu, drawn like the menus of the menu bar, at the
/// position of the right click. Clicking an item or anywhere else closes
/// it; submenus open on hover.
pub fn context_menu(app: &mut App, ui: &mut Ui, response: &Response) {
    let Some((pos, p)) = app.context_menu else {
        return;
    };
    let id = egui::Id::new("canvas_context_menu");
    let ctx = ui.ctx().clone();
    // A new right click (a new position) opens the menu afresh.
    let pos_id = id.with("pos");
    if ctx.data(|d| d.get_temp::<egui::Pos2>(pos_id)) != Some(pos) {
        ctx.data_mut(|d| d.insert_temp(pos_id, pos));
        egui::Popup::open_id(&ctx, id);
    }
    let mut close = false;
    let shown = egui::Popup::new(
        id,
        ctx.clone(),
        egui::PopupAnchor::Position(pos),
        ui.layer_id(),
    )
    .kind(egui::PopupKind::Menu)
    .layout(egui::Layout::top_down_justified(egui::Align::Min))
    .style(menu_popup_style)
    .open_memory(None)
    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
    .show(|ui| {
        ui.set_min_width(200.0);
        body(ui, |ui| {
            let on_guide = app.selection.is_empty()
                && matches!(app.tool, Tool::Pick | Tool::FreeformPick)
                && app.guide_at(p).is_some();
            if let Some(locked) = app.context_locked {
                locked_menu(app, ui, locked, &mut close);
            } else if app.tool == Tool::Shape {
                node_menu(app, ui, &mut close);
            } else if on_guide {
                guide_menu(app, ui, &mut close);
            } else if app.selection.is_empty() {
                page_menu(app, ui, p, &mut close);
            } else {
                object_menu(app, ui, &mut close);
            }
        })
    });
    if close || shown.is_none() {
        egui::Popup::close_id(&ctx, id);
        app.context_menu = None;
        app.context_locked = None;
        ctx.data_mut(|d| d.remove::<egui::Pos2>(pos_id));
    }
    let _ = response;
}

/// Right-click on a guideline: delete, lock or unlock, and the Guidelines
/// docker.
fn guide_menu(app: &mut App, ui: &mut Ui, close: &mut bool) {
    let guides = app.page_guides();
    let selected = app.selected_guides.clone();
    let locked = !selected.is_empty()
        && selected
            .iter()
            .all(|i| guides.get(*i).is_some_and(|g| g.locked));
    if ci(
        ui,
        close,
        "menu.edit.undo",
        "Ctrl+Z",
        app.engine.undo_label().is_some(),
    ) {
        app.undo();
    }
    sep(ui);
    if ci(ui, close, "menu.edit.delete", "Delete", !locked) && !app.delete_selected_guides() {
        app.status = tr("status.guideline_locked");
    }
    sep(ui);
    if locked {
        if ci(ui, close, "guides.unlock", "", true) {
            app.set_guides_locked(&selected, false);
        }
    } else if ci(ui, close, "guides.lock", "", !selected.is_empty()) {
        app.set_guides_locked(&selected, true);
    }
    sep(ui);
    if ci(ui, close, "menu.object.properties", "", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Guidelines;
        if let Some(i) = selected.first().copied() {
            app.open_guide_properties(i);
        }
    }
}

/// Right-click on a locked object: only unlocking it (it is selected
/// once unlocked) or every locked object of the page.
fn locked_menu(app: &mut App, ui: &mut Ui, id: ShapeId, close: &mut bool) {
    if ci(ui, close, "menu.object.unlock_object", "", true) {
        app.unlock_object(id);
    }
    if ci(ui, close, "menu.object.unlock_all", "", true) {
        app.unlock_all();
    }
}

pub(crate) fn object_menu(app: &mut App, ui: &mut Ui, close: &mut bool) {
    let shapes = app.selected_shapes();
    let is_text = shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Text { .. }));
    let is_group = shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Group { .. }));
    let is_curve = shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Path { .. }));
    let is_bitmap = shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Bitmap { .. }));
    let many = shapes.len() > 1;
    if is_text {
        if ci(ui, close, "menu.text.edit_text", "Ctrl+Shift+T", true) {
            app.open_edit_text_dialog();
        }
        if ci(ui, close, "menu.text.convert", "Ctrl+F8", true) {
            app.toggle_text_kind();
        }
        if many
            && !app.joinable_text_lines().is_empty()
            && ci(ui, close, "menu.text.join_lines", "", true)
        {
            app.join_text_lines();
        }
        sep(ui);
    }
    if is_bitmap {
        // Edit Bitmap, then Remove Background (greyed until the AI
        // service makes pictures; the tip says why), then tracing,
        // cropping and remixing.
        let ai = app.ai_image_ready() && app.ai_source_bitmap().is_some() && !app.ai_image.busy();
        let tip = app.ai_image_tip();
        if ci(
            ui,
            close,
            "menu.bitmaps.edit_bitmap",
            "",
            app.selected_bitmap().is_some(),
        ) {
            app.edit_bitmap_externally();
        }
        // On this computer, no AI service needed (#231).
        if ci(
            ui,
            close,
            "menu.bitmaps.remove_background_local",
            "",
            app.selected_bitmap().is_some(),
        ) {
            crate::ui::bitmap_dialogs::open_remove_background(app);
        }
        if ci_tip(
            ui,
            close,
            "menu.bitmaps.remove_background",
            ai,
            tip.as_deref(),
        ) {
            app.ai_remove_background();
        }
        if ci(
            ui,
            close,
            "menu.bitmaps.save_bitmap_as",
            "",
            app.selected_bitmap().is_some(),
        ) {
            app.save_bitmap_as();
        }
        // The trace commands of the reference program's bitmap menu
        // (#452): Quick Trace, the last trace again or its settings, and
        // the Centerline and Outline presets.
        if ci(ui, close, "menu.bitmaps.quick_trace", "", true) {
            app.quick_trace();
        }
        let last = app.last_trace.is_some();
        if ci(ui, close, "menu.bitmaps.repeat_last_trace", "", last) {
            app.repeat_last_trace();
        }
        if ci(ui, close, "menu.bitmaps.last_trace_settings", "", last) {
            app.open_last_trace_settings();
        }
        sub(ui, "menu.bitmaps.centerline_trace", |ui| {
            for (key, preset) in crate::ui::menus::CENTERLINE_TRACES {
                if ci(ui, close, key, "", true) {
                    app.open_trace(preset);
                }
            }
        });
        sub(ui, "menu.bitmaps.outline_trace", |ui| {
            for (key, preset) in crate::ui::menus::OUTLINE_TRACES {
                if ci(ui, close, key, "", true) {
                    app.open_trace(preset);
                }
            }
        });
        if ci(ui, close, "menu.bitmaps.crop_bitmap", "", true) {
            app.set_tool(Tool::Crop);
        }
        if ci_tip(ui, close, "context.ai_remix", ai, tip.as_deref()) {
            app.ai_image.mode = crate::ai_image::Mode::Remix;
            app.ai_image.info = false;
            app.ai_source_from_selection();
            app.open_ai_generate();
        }
        sep(ui);
    }
    // Clones and their masters.
    let has_clone = app.selection_has_clone();
    let has_clones = app.selection_has_clones();
    if has_clone || has_clones {
        if has_clone {
            if ci(ui, close, "context.select_master", "", true) {
                app.select_clone_masters();
            }
            if ci(ui, close, "context.revert_to_master", "", true) {
                app.revert_clones_to_master();
            }
            if ci(ui, close, "context.unlink_clone", "", true) {
                app.unlink_clones();
            }
        }
        if has_clones && ci(ui, close, "context.select_clones", "", true) {
            app.select_clones_of_selection();
        }
        sep(ui);
    }
    // One object (a group most of all): edit it on its own.
    if ci(
        ui,
        close,
        "context.bring_into_focus",
        "",
        shapes.len() == 1 && app.focus_target() != Some(shapes[0].id),
    ) {
        app.bring_into_focus();
    }
    if ci(
        ui,
        close,
        "menu.object.convert_to_curves",
        "Ctrl+Q",
        app.can_convert_to_curves(),
    ) {
        app.convert_to_curves();
    }
    // Break Apart, or the drop shadow's or effect group's version of it.
    let label = app.break_apart_label();
    // Also a warped object (its warp is baked into curves) and artistic
    // text, as the Object menu and Ctrl+K do (#647).
    let can = is_curve
        || is_group
        || label != "menu.object.break_apart"
        || shapes.iter().any(App::breaks_apart);
    if ci(ui, close, label, "Ctrl+K", can) {
        app.break_apart();
    }
    if ci(ui, close, "menu.object.combine", "Ctrl+L", many) {
        app.combine();
    }
    sep(ui);
    if ci(ui, close, "menu.edit.cut", "Ctrl+X", true) {
        app.cut();
    }
    if ci(ui, close, "menu.edit.copy", "Ctrl+C", true) {
        app.copy_with_system();
    }
    if ci(ui, close, "menu.edit.duplicate", "Ctrl+D", true) {
        app.duplicate();
    }
    if ci(ui, close, "menu.edit.delete", "Delete", true) {
        app.delete_selection();
    }
    if ci(ui, close, "menu.object.add_to_shelf", "", true) && app.shelf_add_selection() {
        app.open_docker(DockerTab::Shelf);
    }
    sep(ui);
    sub(ui, "menu.object.order", |ui| {
        if ci(ui, close, "menu.object.to_front_of_page", "Ctrl+Home", true) {
            app.order_page(true);
        }
        if ci(ui, close, "menu.object.to_back_of_page", "Ctrl+End", true) {
            app.order_page(false);
        }
        if ci(ui, close, "menu.object.forward_one", "Ctrl+PgUp", true) {
            app.order(1);
        }
        if ci(ui, close, "menu.object.back_one", "Ctrl+PgDn", true) {
            app.order(2);
        }
        if ci(ui, close, "menu.object.in_front_of", "", true) {
            app.pending_order = Some(true);
        }
        if ci(ui, close, "menu.object.behind", "", true) {
            app.pending_order = Some(false);
        }
        if ci(ui, close, "menu.object.reverse_order", "", many) {
            app.reverse_order();
        }
    });
    if many && ci(ui, close, "menu.object.group_group", "Ctrl+G", true) {
        app.group_selection();
    }
    if is_group {
        if ci(ui, close, "menu.object.group_ungroup", "Ctrl+U", true) {
            app.ungroup_selection();
        }
        if ci(ui, close, "menu.object.group_ungroup_all", "", true) {
            app.ungroup_all();
        }
    }
    sep(ui);
    if ci(ui, close, "menu.object.hide_object", "", true) {
        app.set_visible(false);
    }
    let locked = shapes.iter().any(|s| s.locked);
    if ci(
        ui,
        close,
        if locked {
            "menu.object.unlock_object"
        } else {
            "menu.object.lock_object"
        },
        "",
        true,
    ) {
        app.set_locked(!locked);
    }
    sep(ui);
    if ci(ui, close, "menu.object.clip_frame_place_inside", "", true) {
        app.pending_clip_frame = true;
        app.status = tr("status.click_frame");
    }
    sub(ui, "context.frame_type", |ui| {
        let empty_frames = !app.empty_clip_frames().is_empty();
        if ci(ui, close, "context.frame_none", "", empty_frames) {
            app.frames_to_objects();
        }
        let framable = !app.framable().is_empty();
        if ci(ui, close, "context.frame_empty_clip_frame", "", framable) {
            app.objects_to_clip_frames();
        }
        if ci(
            ui,
            close,
            "context.frame_text",
            "",
            app.selection.len() == 1,
        ) {
            app.object_to_text_frame();
        }
    });
    sep(ui);
    if ci(ui, close, "menu.object.symmetry_create", "Alt+S", true) {
        app.create_symmetry();
    }
    if ci(ui, close, "context.create_symbol", "", true) {
        app.create_symbol_from_selection();
    }
    sub(ui, "context.internet_links", |ui| {
        if ci(ui, close, "docker.hyperlink", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::Links;
        }
    });
    sub(ui, "docker.object_styles", |ui| {
        if ci(ui, close, "docker.new_style_from_selection", "", true) {
            app.new_style_from_selection();
        }
        let styles = app.doc().object_styles.clone();
        if !styles.is_empty() {
            sep(ui);
        }
        for (i, os) in styles.iter().enumerate() {
            if item_label(ui, &os.name, "", true) {
                app.apply_style(i);
                *close = true;
            }
        }
        let linked = !app.styled_selection().is_empty();
        if linked {
            sep(ui);
        }
        if ci(ui, close, "docker.break_style_link", "", linked) {
            app.break_style_links();
        }
        if ci(ui, close, "docker.open_object_styles", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::ObjectStyles;
        }
    });
    sub(ui, "docker.color_styles", |ui| {
        if ci(ui, close, "docker.new_from_selection", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::ColorStyles;
        }
    });
    sep(ui);
    // Wrap Paragraph Text: on or off, the seven styles and the offset.
    let (wrap, current) = app.wrap_state();
    sub(ui, "context.wrap_paragraph_text", |ui| {
        if check(ui, "context.wrap_text_on", "", wrap) {
            let ids = app.selection.clone();
            app.run(Command::SetWrapText {
                shapes: ids,
                wrap: !wrap,
            });
            *close = true;
        }
        sep(ui);
        for (mode, side) in tracedraw_core::TextWrap::STYLES {
            let here = wrap && current.mode == mode && current.side == side;
            if radio(ui, crate::ui::propbar::wrap_style_key(mode, side), "", here) {
                app.set_wrap_style(mode, side);
                *close = true;
            }
        }
        sep(ui);
        crate::ui::propbar::wrap_offset_field(app, ui, current.offset);
    });
    sep(ui);
    let of = shapes.iter().any(|s| s.overprint_fill);
    let oo = shapes.iter().any(|s| s.overprint_outline);
    if check(ui, "menu.object.overprint_fill", "", of) {
        app.toggle_overprint(true);
        *close = true;
    }
    if check(ui, "menu.object.overprint_outline", "", oo) {
        app.toggle_overprint(false);
        *close = true;
    }
    let hint = shapes.iter().all(|s| app.object_hinted(s.id));
    if check(ui, "menu.object.object_hinting", "", hint) {
        app.toggle_object_hinting();
        *close = true;
    }
    if ci(ui, close, "context.align_pixel_grid", "", true) {
        app.align_to_pixel_grid();
    }
    sep(ui);
    if ci(ui, close, "menu.object.properties", "Alt+Enter", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
    }
}

fn node_menu(app: &mut App, ui: &mut Ui, close: &mut bool) {
    use tracedraw_core::nodes::NodeType;
    let has_nodes = !app.node_selection.is_empty();
    if ci(ui, close, "context.node_add", "", has_nodes) {
        app.add_node_midpoints();
    }
    if ci(ui, close, "context.node_delete", "Delete", has_nodes) {
        app.delete_selected_nodes();
    }
    sep(ui);
    if ci(
        ui,
        close,
        "context.node_join",
        "",
        app.node_selection.len() == 2,
    ) {
        app.join_selected_nodes();
    }
    if ci(ui, close, "context.node_break", "", has_nodes) {
        app.break_selected_nodes();
    }
    sep(ui);
    if ci(ui, close, "context.node_to_line", "", has_nodes) {
        app.selected_segments_to_line();
    }
    if ci(ui, close, "context.node_to_curve", "", has_nodes) {
        app.selected_segments_to_curve();
    }
    sep(ui);
    if ci(ui, close, "context.node_cusp", "C", has_nodes) {
        app.set_selected_node_type(NodeType::Cusp);
    }
    if ci(ui, close, "context.node_smooth", "S", has_nodes) {
        app.set_selected_node_type(NodeType::Smooth);
    }
    if ci(ui, close, "context.node_symmetrical", "Y", has_nodes) {
        app.set_selected_node_type(NodeType::Symmetrical);
    }
    sep(ui);
    if ci(ui, close, "context.node_reverse", "", true) {
        app.reverse_selected_curves();
    }
    if ci(ui, close, "context.node_extract", "", has_nodes) {
        app.extract_subpath();
    }
    sep(ui);
    if ci(ui, close, "context.node_close", "", true) {
        app.close_selected_curves();
    }
    if check(ui, "context.elastic_mode", "", app.elastic_mode) {
        app.elastic_mode = !app.elastic_mode;
        *close = true;
    }
    sep(ui);
    if ci(ui, close, "menu.object.properties", "Alt+Enter", true) {
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
    }
}

fn page_menu(app: &mut App, ui: &mut Ui, p: tracedraw_core::Point, close: &mut bool) {
    if ci(
        ui,
        close,
        "menu.edit.paste",
        "Ctrl+V",
        app.clipboard.is_some(),
    ) {
        app.paste_any();
        if let Some(b) = app.selection_bounds() {
            let shapes = app.selection.clone();
            app.run(Command::TransformShapes {
                shapes,
                transform: tracedraw_core::Affine::translate(p - b.center()),
            });
        }
    }
    if ci(ui, close, "menu.edit.select_all_objects", "Ctrl+A", true) {
        app.select_all();
    }
    sep(ui);
    if ci(ui, close, "menu.layout.insert_page", "", true) {
        app.dialog = Dialog::InsertPage {
            count: 1,
            after: true,
        };
    }
    if ci(ui, close, "menu.layout.rename_page", "", true) {
        let name = app
            .doc()
            .page(app.page)
            .map(|pg| pg.name.clone())
            .unwrap_or_default();
        app.dialog = Dialog::RenamePage { name };
    }
    if ci(ui, close, "menu.layout.page_size", "", true) {
        let s = app.page_size();
        app.dialog = Dialog::PageSize {
            width: s.width,
            height: s.height,
            all_pages: true,
        };
    }
    if ci(ui, close, "menu.layout.page_background", "", true) {
        app.dialog = Dialog::Options;
        app.options_page = crate::ui::dialogs::OptionsPage::Background;
    }
    sep(ui);
    if ci(ui, close, "menu.view.zoom_to_page", "Shift+F4", true) {
        app.zoom_to_page();
    }
    if ci(ui, close, "menu.view.zoom_to_fit", "F4", true) {
        app.zoom_to_fit();
    }
    sep(ui);
    if ci(ui, close, "menu.tools.options_app", "Ctrl+J", true) {
        app.dialog = Dialog::Options;
    }
}
