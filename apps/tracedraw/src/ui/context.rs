//! Right-click context menus: object menu (Pick tool), node menu (Shape
//! tool), page menu (empty area), text menu.

use crate::app::{App, DockerTab};
use crate::i18n::tr;
use crate::tools::Tool;
use crate::ui::dialogs::Dialog;
use egui::{Response, Ui};
use tracedraw_core::{document::ShapeKind, Command};

fn ci(ui: &mut Ui, close: &mut bool, key: &str, shortcut: &str, enabled: bool) -> bool {
    let r = item(ui, key, shortcut, enabled);
    if r {
        *close = true;
    }
    r
}

fn item(ui: &mut Ui, key: &str, shortcut: &str, enabled: bool) -> bool {
    let text = if shortcut.is_empty() {
        tr(key)
    } else {
        format!("{}    {shortcut}", tr(key))
    };
    let r = ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(text).size(12.0)).frame(false),
    );
    if r.clicked() {
        ui.close();
        true
    } else {
        false
    }
}

pub fn context_menu(app: &mut App, ui: &mut Ui, response: &Response) {
    let Some((pos, p)) = app.context_menu else {
        return;
    };
    let id = egui::Id::new("canvas_context_menu");
    let mut close = false;
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(200.0);
                if app.tool == Tool::Shape {
                    node_menu(app, ui, &mut close);
                } else if app.selection.is_empty() {
                    page_menu(app, ui, p, &mut close);
                } else {
                    object_menu(app, ui, &mut close);
                }
            });
        });
    // Close on any click outside or Escape.
    let clicked_elsewhere = ui.input(|i| i.pointer.any_pressed())
        && !ui
            .ctx()
            .layer_id_at(ui.input(|i| i.pointer.interact_pos().unwrap_or_default()))
            .map(|l| l.id == id)
            .unwrap_or(false);
    if close || clicked_elsewhere || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.context_menu = None;
    }
    let _ = response;
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
        ui.separator();
    }
    if is_bitmap {
        if ci(ui, close, "menu.bitmaps.crop_bitmap", "", true) {
            app.set_tool(Tool::Crop);
        }
        if ci(ui, close, "menu.bitmaps.quick_trace", "", true) {
            app.quick_trace();
        }
        ui.separator();
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
    ui.separator();
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
    ui.separator();
    ui.menu_button(tr("menu.object.order"), |ui| {
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
    ui.separator();
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
    ui.separator();
    if ci(ui, close, "menu.object.clip_frame_place_inside", "", true) {
        app.pending_clip_frame = true;
        app.status = tr("status.click_frame");
    }
    ui.menu_button(tr("context.frame_type"), |ui| {
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
    ui.separator();
    if ci(ui, close, "context.create_symbol", "", true) {
        app.create_symbol_from_selection();
    }
    ui.menu_button(tr("context.internet_links"), |ui| {
        if ci(ui, close, "docker.hyperlink", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::Links;
        }
    });
    ui.menu_button(tr("docker.object_styles"), |ui| {
        if ci(ui, close, "docker.new_style_from_selection", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::ObjectStyles;
        }
        let styles = app.doc().object_styles.clone();
        for os in styles {
            if ui.button(&os.name).clicked() {
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
                ui.close();
                *close = true;
            }
        }
    });
    ui.menu_button(tr("docker.color_styles"), |ui| {
        if ci(ui, close, "docker.new_from_selection", "", true) {
            app.show_dockers = true;
            app.docker_tab = DockerTab::ColorStyles;
        }
    });
    ui.separator();
    let mut wrap = shapes.iter().any(|s| s.wrap_text);
    if ui
        .checkbox(&mut wrap, tr("context.wrap_paragraph_text"))
        .changed()
    {
        let ids = app.selection.clone();
        app.run(Command::SetWrapText { shapes: ids, wrap });
    }
    ui.separator();
    let of = shapes.iter().any(|s| s.overprint_fill);
    let oo = shapes.iter().any(|s| s.overprint_outline);
    let mut f = of;
    if ui
        .checkbox(&mut f, tr("menu.object.overprint_fill"))
        .changed()
    {
        app.toggle_overprint(true);
    }
    let mut o = oo;
    if ui
        .checkbox(&mut o, tr("menu.object.overprint_outline"))
        .changed()
    {
        app.toggle_overprint(false);
    }
    let mut hint = shapes.iter().all(|s| app.object_hinted(s.id));
    if ui
        .checkbox(&mut hint, tr("menu.object.object_hinting"))
        .changed()
    {
        app.toggle_object_hinting();
    }
    if ci(ui, close, "context.align_pixel_grid", "", true) {
        app.align_to_pixel_grid();
    }
    ui.separator();
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
    ui.separator();
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
    ui.separator();
    if ci(ui, close, "context.node_to_line", "", has_nodes) {
        app.selected_segments_to_line();
    }
    if ci(ui, close, "context.node_to_curve", "", has_nodes) {
        app.selected_segments_to_curve();
    }
    ui.separator();
    if ci(ui, close, "context.node_cusp", "C", has_nodes) {
        app.set_selected_node_type(NodeType::Cusp);
    }
    if ci(ui, close, "context.node_smooth", "S", has_nodes) {
        app.set_selected_node_type(NodeType::Smooth);
    }
    if ci(ui, close, "context.node_symmetrical", "Y", has_nodes) {
        app.set_selected_node_type(NodeType::Symmetrical);
    }
    ui.separator();
    if ci(ui, close, "context.node_reverse", "", true) {
        app.reverse_selected_curves();
    }
    if ci(ui, close, "context.node_extract", "", has_nodes) {
        app.extract_subpath();
    }
    ui.separator();
    if ci(ui, close, "context.node_close", "", true) {
        app.close_selected_curves();
    }
    let mut elastic = app.elastic_mode;
    if ui
        .checkbox(&mut elastic, tr("context.elastic_mode"))
        .changed()
    {
        app.elastic_mode = elastic;
    }
    ui.separator();
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
    ui.separator();
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
    ui.separator();
    if ci(ui, close, "menu.view.zoom_to_page", "Shift+F4", true) {
        app.zoom_to_page();
    }
    if ci(ui, close, "menu.view.zoom_to_fit", "F4", true) {
        app.zoom_to_fit();
    }
    ui.separator();
    if ci(ui, close, "menu.tools.options_app", "Ctrl+J", true) {
        app.dialog = Dialog::Options;
    }
}
