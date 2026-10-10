//! Right-click context menus: object menu (Pick tool), node menu (Shape
//! tool), page menu (empty area), text menu.

use crate::app::{App, DockerTab};
use crate::i18n::tr;
use crate::tools::Tool;
use crate::ui::dialogs::Dialog;
use crate::ui::menus::{body, check, item, item_label, menu_popup_style, sep, sub};
use egui::{Response, Ui};
use tracedraw_core::{document::ShapeKind, Command};

fn ci(ui: &mut Ui, close: &mut bool, key: &str, shortcut: &str, enabled: bool) -> bool {
    let r = item(ui, key, shortcut, enabled);
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
            if app.tool == Tool::Shape {
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
        if let Some(g) = selected.first().and_then(|i| guides.get(*i)) {
            let mut form = app.guide_form;
            form.load(app, &g.line);
            app.guide_form = form;
        }
    }
}

fn object_menu(app: &mut App, ui: &mut Ui, close: &mut bool) {
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
            app.edit_selected_text();
        }
        if ci(ui, close, "menu.text.convert", "Ctrl+F8", true) {
            app.toggle_text_kind();
        }
        sep(ui);
    }
    if is_bitmap {
        if ci(ui, close, "menu.bitmaps.crop_bitmap", "", true) {
            app.set_tool(Tool::Crop);
        }
        if ci(ui, close, "menu.bitmaps.quick_trace", "", true) {
            app.quick_trace();
        }
        sep(ui);
    }
    if ci(
        ui,
        close,
        "menu.object.convert_to_curves",
        "Ctrl+Q",
        !is_curve || many,
    ) {
        app.convert_to_curves();
    }
    if ci(
        ui,
        close,
        "menu.object.break_apart",
        "Ctrl+K",
        is_curve || is_group,
    ) {
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
    sep(ui);
    sub(ui, "menu.object.order", |ui| {
        if ci(ui, close, "menu.object.to_front_of_page", "Ctrl+Home", true) {
            app.order(0);
        }
        if ci(ui, close, "menu.object.to_back_of_page", "Ctrl+End", true) {
            app.order(3);
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
        if ci(ui, close, "context.frame_none", "", true) {
            app.apply_outline_color(None);
        }
        if ci(ui, close, "context.frame_empty_clip_frame", "", true) {
            let shapes = app.selection.clone();
            for id in shapes {
                let Some(s) = app.doc().find_shape(id).cloned() else {
                    continue;
                };
                let frame = Box::new(s.clone());
                app.run(Command::SetShapeKind {
                    shape: id,
                    kind: ShapeKind::ClipFrame {
                        frame,
                        contents: Vec::new(),
                    },
                });
            }
        }
        if ci(ui, close, "context.frame_text", "", true) {
            app.pending_text_frame = true;
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
            app.show_dockers = true;
            app.docker_tab = DockerTab::ObjectStyles;
        }
        let styles = app.doc().object_styles.clone();
        for os in styles {
            if item_label(ui, &os.name, "", true) {
                let shapes = app.selection.clone();
                let cmds = vec![
                    Command::SetFill {
                        shapes: shapes.clone(),
                        fill: os.fill.clone(),
                    },
                    Command::SetStroke {
                        shapes,
                        stroke: os.stroke.clone(),
                    },
                ];
                let _ = app.engine.run_batch("Apply Style", &cmds);
                *close = true;
            }
        }
    });
    sub(ui, "docker.color_styles", |ui| {
        if ci(ui, close, "docker.new_from_selection", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::ColorStyles;
        }
    });
    sep(ui);
    let wrap = shapes.iter().any(|s| s.wrap_text);
    if check(ui, "context.wrap_paragraph_text", "", wrap) {
        let ids = app.selection.clone();
        app.run(Command::SetWrapText {
            shapes: ids,
            wrap: !wrap,
        });
        *close = true;
    }
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
