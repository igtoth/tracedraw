//! Standard toolbar and the context-sensitive property bar.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::icons;
use crate::ui::tips::RichTip;
use egui::{Ui, Vec2};
use tracedraw_core::{document::ShapeKind, geometry::Rect, Fill};

fn vsep(ui: &mut Ui) {
    ui.add(egui::Separator::default().vertical().spacing(6.0));
}

/// Standard toolbar buttons: 32 px square with a 20 px icon.
const STD: f32 = 32.0;
const STD_ICON: f32 = 20.0;

fn std_frame(ui: &Ui, rect: egui::Rect, hovered: bool, pressed: bool) {
    if pressed {
        ui.painter()
            .rect_filled(rect, 0.0, crate::theme::Tokens::field());
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            egui::Stroke::new(1.0, Tokens::control_border()),
            egui::StrokeKind::Middle,
        );
    } else if hovered {
        ui.painter()
            .rect_filled(rect, 0.0, crate::theme::Tokens::hovered_row());
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            egui::Stroke::new(1.0, crate::theme::Tokens::selected_row()),
            egui::StrokeKind::Middle,
        );
    }
}

/// A standard toolbar button; `pressed` draws it pushed in (toggles).
fn std_button(
    ui: &mut Ui,
    action: icons::Action,
    tip: &str,
    enabled: bool,
    pressed: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(STD),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    std_frame(ui, rect, enabled && resp.hovered(), pressed);
    let resp = resp.on_hover_ui(|ui| rich_tip(ui, tip));
    icons::draw_action(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), Vec2::splat(STD_ICON)),
        action,
        if enabled {
            Tokens::icon()
        } else {
            Tokens::icon_off()
        },
    );
    resp
}

/// A standard toolbar button that opens a list: the icon with a small
/// down arrow on its right, one button.
fn std_list_button(ui: &mut Ui, action: icons::Action, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(STD + 11.0, STD), egui::Sense::click());
    std_frame(ui, rect, resp.hovered(), resp.is_pointer_button_down_on());
    let resp = resp.on_hover_ui(|ui| rich_tip(ui, tip));
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.left() + STD / 2.0, rect.center().y),
        Vec2::splat(STD_ICON),
    );
    icons::draw_action(ui.painter(), icon, action, Tokens::icon());
    let c = egui::pos2(rect.right() - 7.0, rect.center().y + 1.0);
    ui.painter().add(egui::epaint::PathShape::convex_polygon(
        vec![
            c + Vec2::new(-3.0, -1.5),
            c + Vec2::new(3.0, -1.5),
            c + Vec2::new(0.0, 2.0),
        ],
        Tokens::text_dim(),
        egui::Stroke::NONE,
    ));
    resp
}

/// A button's tip: the title (with its shortcut) in bold on the first
/// line, what it does under it.
pub(crate) fn rich_tip(ui: &mut Ui, tip: &str) {
    crate::ui::tips::show(ui, tip);
}

/// The tip of the standard bar's button `key`: its title and the
/// description kept under `key_desc`.
fn tip(key: &str) -> String {
    format!("{}\n{}", tr(key), tr(&format!("{key}_desc")))
}

/// The small arrow after a button that opens its list.
fn std_arrow(ui: &mut Ui, id: &str, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(14.0, STD),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let resp = resp.rich_tip(tr("toolbar.more"));
    let _ = id;
    std_frame(ui, rect, enabled && resp.hovered(), false);
    let c = rect.center();
    let color = if enabled {
        Tokens::text_dim()
    } else {
        Tokens::icon_off()
    };
    ui.painter().add(egui::epaint::PathShape::convex_polygon(
        vec![
            c + Vec2::new(-4.0, -2.0),
            c + Vec2::new(4.0, -2.0),
            c + Vec2::new(0.0, 2.5),
        ],
        color,
        egui::Stroke::NONE,
    ));
    resp
}

/// A thin vertical separator between button groups.
fn std_sep(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, STD), egui::Sense::hover());
    let x = rect.center().x.round() + 0.5;
    ui.painter().vline(
        x,
        (rect.top() + 4.0)..=(rect.bottom() - 4.0),
        egui::Stroke::new(1.0, crate::theme::Tokens::border()),
    );
}

/// The zoom levels the zoom box lists.
fn zoom_level_menu(app: &mut App, ui: &mut Ui) {
    for (key, f) in [
        ("toolbar.zoom_to_selected", 0),
        ("toolbar.zoom_to_fit", 1),
        ("toolbar.zoom_to_page", 2),
        ("toolbar.zoom_to_width", 3),
        ("toolbar.zoom_to_height", 4),
    ] {
        if ui.button(tr(key)).clicked() {
            match f {
                0 => app.zoom_to_selection(),
                1 => app.zoom_to_fit(),
                2 => app.zoom_to_page(),
                3 => app.zoom_to_page_width(),
                _ => app.zoom_to_page_height(),
            }
            ui.close();
        }
    }
    ui.separator();
    for pct in [10.0, 25.0, 50.0, 75.0, 100.0, 200.0, 400.0, 800.0, 1600.0] {
        if ui.button(format!("{pct:.0}%")).clicked() {
            app.set_zoom_percent(pct);
            ui.close();
        }
    }
}

/// The zoom box: the zoom level as text (type a value and press Enter)
/// and the list of levels behind its arrow.
pub(crate) fn zoom_box_pub(app: &mut App, ui: &mut Ui, id: &str, width: f32) {
    zoom_box(app, ui, id, width)
}

fn zoom_box(app: &mut App, ui: &mut Ui, id: &str, width: f32) {
    let edit_id = egui::Id::new((id, "text"));
    let mut text = ui
        .data(|d| d.get_temp::<String>(edit_id))
        .unwrap_or_else(|| format!("{:.0}%", app.zoom_percent()));
    let field = egui::Frame::new()
        .fill(crate::theme::Tokens::field())
        .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
        .inner_margin(egui::Margin::ZERO);
    field.show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.horizontal(|ui| {
            let r = ui.add_sized(
                [width - 22.0, 24.0],
                egui::TextEdit::singleline(&mut text)
                    .frame(egui::Frame::NONE)
                    .vertical_align(egui::Align::Center)
                    .margin(egui::Margin::symmetric(4, 2)),
            );
            if r.has_focus() {
                ui.data_mut(|d| d.insert_temp(edit_id, text.clone()));
            } else {
                ui.data_mut(|d| d.remove::<String>(edit_id));
            }
            if r.lost_focus() {
                if let Ok(v) = text.trim().trim_end_matches('%').trim().parse::<f32>() {
                    if v > 0.0 {
                        app.set_zoom_percent(v);
                    }
                }
            }
            let (arrow, resp) = ui.allocate_exact_size(Vec2::new(22.0, 24.0), egui::Sense::click());
            ui.painter().rect_filled(
                arrow,
                0.0,
                if resp.hovered() {
                    crate::theme::Tokens::hovered_row()
                } else {
                    Tokens::panel_dark()
                },
            );
            let c = arrow.center();
            ui.painter().add(egui::epaint::PathShape::convex_polygon(
                vec![
                    c + Vec2::new(-4.0, -2.0),
                    c + Vec2::new(4.0, -2.0),
                    c + Vec2::new(0.0, 2.5),
                ],
                Tokens::text_dim(),
                egui::Stroke::NONE,
            ));
            egui::Popup::menu(&resp)
                .id(egui::Id::new((id, "levels")))
                .show(|ui| zoom_level_menu(app, ui));
        });
    });
}

pub fn standard_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        ui.set_min_height(STD);
        let doc = app.has_document();
        if std_button(ui, icons::Action::New, &tip("toolbar.new"), true, false).clicked() {
            app.request_new_document();
        }
        if std_button(ui, icons::Action::Open, &tip("toolbar.open"), true, false).clicked() {
            app.open_dialog();
        }
        // Recent drawings behind the Open arrow.
        let recent = app.settings.recent_files.clone();
        let arrow = std_arrow(ui, "open_recent", !recent.is_empty());
        egui::Popup::menu(&arrow)
            .id(egui::Id::new("open_recent_menu"))
            .show(|ui| {
                for path in &recent {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if ui
                        .button(name)
                        .rich_tip(path.display().to_string())
                        .clicked()
                    {
                        app.open_path(path.clone());
                        ui.close();
                    }
                }
            });
        if std_button(ui, icons::Action::Save, &tip("toolbar.save"), doc, false).clicked() {
            app.save(false);
        }
        // The drawing's Git repository: open a file from it, or save the
        // drawing to it as a commit.
        if std_button(
            ui,
            icons::Action::GitPull,
            &tip("toolbar.git_pull"),
            true,
            false,
        )
        .clicked()
        {
            app.github.status.clear();
            app.dialog = crate::ui::dialogs::Dialog::GithubOpen(app.github_form(false));
        }
        if std_button(
            ui,
            icons::Action::GitPush,
            &tip("toolbar.git_push"),
            doc,
            false,
        )
        .clicked()
        {
            app.github.status.clear();
            app.dialog = crate::ui::dialogs::Dialog::GithubSave(app.github_form(true));
        }
        if std_button(ui, icons::Action::Print, &tip("toolbar.print"), doc, false).clicked() {
            app.print();
        }
        std_sep(ui);
        let has = doc && !app.selection.is_empty();
        if std_button(ui, icons::Action::Cut, &tip("toolbar.cut"), has, false).clicked() {
            app.cut();
        }
        if std_button(ui, icons::Action::Copy, &tip("toolbar.copy"), has, false).clicked() {
            app.copy_with_system();
        }
        let can_paste = doc && app.clipboard.is_some();
        if std_button(
            ui,
            icons::Action::Paste,
            &tip("toolbar.paste"),
            can_paste,
            false,
        )
        .clicked()
        {
            app.paste_any();
        }
        std_sep(ui);
        // Undo and Redo, each with the list of steps behind its arrow.
        let (undo_steps, redo_steps): (Vec<String>, Vec<String>) = if doc {
            let (u, r) = app.engine.history_labels();
            (
                u.iter().rev().map(|s| s.to_string()).collect(),
                r.iter().rev().map(|s| s.to_string()).collect(),
            )
        } else {
            (Vec::new(), Vec::new())
        };
        if std_button(
            ui,
            icons::Action::Undo,
            &tip("toolbar.undo"),
            !undo_steps.is_empty(),
            false,
        )
        .clicked()
        {
            app.undo();
        }
        let arrow = std_arrow(ui, "undo_list", !undo_steps.is_empty());
        egui::Popup::menu(&arrow)
            .id(egui::Id::new("undo_list_menu"))
            .show(|ui| {
                for (i, label) in undo_steps.iter().enumerate().take(30) {
                    if ui.button(label).clicked() {
                        for _ in 0..=i {
                            app.undo();
                        }
                        ui.close();
                    }
                }
            });
        if std_button(
            ui,
            icons::Action::Redo,
            &tip("toolbar.redo"),
            !redo_steps.is_empty(),
            false,
        )
        .clicked()
        {
            app.redo();
        }
        let arrow = std_arrow(ui, "redo_list", !redo_steps.is_empty());
        egui::Popup::menu(&arrow)
            .id(egui::Id::new("redo_list_menu"))
            .show(|ui| {
                for (i, label) in redo_steps.iter().enumerate().take(30) {
                    if ui.button(label).clicked() {
                        for _ in 0..=i {
                            app.redo();
                        }
                        ui.close();
                    }
                }
            });
        std_sep(ui);
        if std_button(
            ui,
            icons::Action::Import,
            &tip("toolbar.import"),
            doc,
            false,
        )
        .clicked()
        {
            app.import();
        }
        if std_button(
            ui,
            icons::Action::Export,
            &tip("toolbar.export"),
            doc,
            false,
        )
        .clicked()
        {
            app.export();
        }
        if std_button(
            ui,
            icons::Action::Pdf,
            &tip("toolbar.publish_pdf"),
            doc,
            false,
        )
        .clicked()
        {
            app.publish_pdf();
        }
        std_sep(ui);
        if doc {
            zoom_box(app, ui, "std_zoom", 107.0);
        }
        if std_button(
            ui,
            icons::Action::Fullscreen,
            &tip("toolbar.fullscreen_preview"),
            doc,
            false,
        )
        .clicked()
        {
            app.fullscreen_preview = true;
        }
        std_sep(ui);
        if std_button(
            ui,
            icons::Action::ShowRulers,
            &tip("toolbar.show_rulers"),
            doc,
            app.show_rulers,
        )
        .clicked()
        {
            app.show_rulers = !app.show_rulers;
        }
        if std_button(
            ui,
            icons::Action::ShowGrid,
            &tip("toolbar.show_grid"),
            doc,
            app.show_grid,
        )
        .clicked()
        {
            app.show_grid = !app.show_grid;
        }
        if std_button(
            ui,
            icons::Action::ShowGuidelines,
            &tip("toolbar.show_guidelines"),
            doc,
            app.show_guides,
        )
        .clicked()
        {
            app.show_guides = !app.show_guides;
        }
        std_sep(ui);
        if std_button(
            ui,
            icons::Action::SnapOff,
            &tip("toolbar.snap_off"),
            doc,
            app.snap.off,
        )
        .clicked()
        {
            app.snap.off = !app.snap.off;
        }
        // "Snap To" and its arrow open the snapping list.
        // As wide as its label needs (at least the English size), with
        // room for the arrow.
        let label = tr("menu.view.snap_to");
        let font = egui::FontId::proportional(13.0);
        let text_w = ui
            .painter()
            .layout_no_wrap(label.clone(), font.clone(), Tokens::text())
            .size()
            .x;
        let width = (text_w + 6.0 + 22.0).max(86.0);
        let (rect, snap_resp) = ui.allocate_exact_size(Vec2::new(width, STD), egui::Sense::click());
        std_frame(ui, rect, snap_resp.hovered(), false);
        ui.painter().text(
            egui::pos2(rect.left() + 6.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            Tokens::text(),
        );
        let c = egui::pos2(rect.right() - 9.0, rect.center().y);
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + Vec2::new(-4.0, -2.0),
                c + Vec2::new(4.0, -2.0),
                c + Vec2::new(0.0, 2.5),
            ],
            Tokens::text_dim(),
            egui::Stroke::NONE,
        ));
        // The list stays open while its boxes are ticked; the X or a click
        // outside closes it.
        egui::Popup::menu(&snap_resp)
            .id(egui::Id::new("snap_to_menu"))
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut app.snap.pixels, tr("menu.view.snap_pixels"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new("\u{2715}").frame(false))
                            .rich_tip(tr("toolbar.snap_close"))
                            .clicked()
                        {
                            ui.close();
                        }
                    });
                });
                ui.checkbox(&mut app.snap.grid, tr("menu.view.snap_document_grid"));
                ui.checkbox(
                    &mut app.snap.baseline_grid,
                    tr("menu.view.snap_baseline_grid"),
                );
                ui.checkbox(&mut app.snap.guides, tr("menu.view.snap_guidelines"));
                ui.checkbox(&mut app.snap.objects, tr("menu.view.snap_objects"));
                ui.checkbox(&mut app.snap.page, tr("menu.view.snap_page"));
                ui.separator();
                ui.checkbox(
                    &mut app.snap.alignment_guides,
                    tr("menu.view.alignment_guides"),
                );
                ui.checkbox(&mut app.snap.dynamic_guides, tr("menu.view.dynamic_guides"));
            });
        std_sep(ui);
        // Options opens its list of option pages.
        let options = std_list_button(ui, icons::Action::Options, &tip("toolbar.options"));
        egui::Popup::menu(&options)
            .id(egui::Id::new("options_button_menu"))
            .show(|ui| crate::ui::menus::options_items(app, ui));
        std_sep(ui);
        // Launch: the other tools that come with the editor.
        let (rect, launch) = ui.allocate_exact_size(Vec2::new(108.0, STD), egui::Sense::click());
        std_frame(ui, rect, launch.hovered(), false);
        icons::draw_action(
            ui.painter(),
            egui::Rect::from_center_size(
                egui::pos2(rect.left() + 14.0, rect.center().y),
                Vec2::splat(STD_ICON),
            ),
            icons::Action::Launch,
            Tokens::icon(),
        );
        ui.painter().text(
            egui::pos2(rect.left() + 30.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            tr("toolbar.launch"),
            egui::FontId::proportional(13.0),
            Tokens::text(),
        );
        let c = egui::pos2(rect.right() - 9.0, rect.center().y);
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + Vec2::new(-4.0, -2.0),
                c + Vec2::new(4.0, -2.0),
                c + Vec2::new(0.0, 2.5),
            ],
            Tokens::text_dim(),
            egui::Stroke::NONE,
        ));
        egui::Popup::menu(&launch)
            .id(egui::Id::new("launch_menu"))
            .show(|ui| {
                if ui.button(tr("menu.tools.font_manager")).clicked() {
                    app.dialog = crate::ui::dialogs::Dialog::FontManager(Default::default());
                    ui.close();
                }
                if ui.button(tr("menu.window.welcome_screen")).clicked() {
                    app.show_welcome = true;
                    ui.close();
                }
                if ui.button(tr("menu.tools.color_management")).clicked() {
                    app.dialog = crate::ui::dialogs::Dialog::ColorManagement(Default::default());
                    ui.close();
                }
            });
        std_sep(ui);
        ai_button(app, ui);
    });
}

/// AI Generate on the standard toolbar: the AI docker, with the other AI
/// commands in its list.
fn ai_button(app: &mut App, ui: &mut Ui) {
    let label = tr("toolbar.ai_generate");
    let galley = ui.painter().layout_no_wrap(
        label.clone(),
        egui::FontId::proportional(13.0),
        Tokens::text(),
    );
    let w = galley.size().x + 52.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, STD), egui::Sense::click());
    std_frame(ui, rect, resp.hovered(), false);
    let p = ui.painter();
    p.text(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        "\u{2726}",
        egui::FontId::proportional(15.0),
        Tokens::ACCENT,
    );
    p.galley(
        egui::pos2(rect.left() + 26.0, rect.center().y - galley.size().y / 2.0),
        galley,
        Tokens::text(),
    );
    let c = egui::pos2(rect.right() - 9.0, rect.center().y);
    p.add(egui::epaint::PathShape::convex_polygon(
        vec![
            c + Vec2::new(-4.0, -2.0),
            c + Vec2::new(4.0, -2.0),
            c + Vec2::new(0.0, 2.5),
        ],
        Tokens::text_dim(),
        egui::Stroke::NONE,
    ));
    let resp = resp.on_hover_ui(|ui| {
        rich_tip(
            ui,
            &format!(
                "{}\n{}",
                tr("toolbar.ai_generate"),
                tr("toolbar.ai_generate_tip")
            ),
        )
    });
    // The arrow is a button of its own: it opens the list.
    let arrow_rect =
        egui::Rect::from_min_max(egui::pos2(rect.right() - 18.0, rect.top()), rect.max);
    let arrow = ui.interact(
        arrow_rect,
        egui::Id::new("ai_menu_arrow"),
        egui::Sense::click(),
    );
    if resp.clicked() && !arrow.clicked() && !arrow.hovered() {
        app.open_ai_generate();
    }
    egui::Popup::menu(&arrow)
        .id(egui::Id::new("ai_menu"))
        .show(|ui| {
            if ui.button(tr("menu.tools.ai_generate")).clicked() {
                app.open_ai_generate();
                ui.close();
            }
            if ui.button(tr("menu.object.ai_draw_words")).clicked() {
                app.open_ai_drawing();
                ui.close();
            }
            let cutout = app.ai_image_ready() && app.ai_source_bitmap().is_some();
            let tip = app.ai_image_tip().unwrap_or_default();
            if ui
                .add_enabled(
                    cutout,
                    egui::Button::new(tr("menu.bitmaps.remove_background")),
                )
                .rich_disabled_tip(tip)
                .clicked()
            {
                app.ai_remove_background();
                ui.close();
            }
            ui.separator();
            if ui.button(tr("menu.tools.options_ai")).clicked() {
                crate::ui::ai_ui::open_options(app, crate::ui::dialogs::OptionsPage::AiGeneral);
                ui.close();
            }
        });
}

/// Window > Toolbars > Text: font controls as a standalone bar.
pub fn text_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        text_properties(app, ui);
    });
}

/// Window > Toolbars > Zoom.
pub fn zoom_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        for (label, pct) in [
            ("25%", 25.0),
            ("50%", 50.0),
            ("100%", 100.0),
            ("200%", 200.0),
            ("400%", 400.0),
        ] {
            if ui.button(label).clicked() {
                app.set_zoom_percent(pct);
            }
        }
        vsep(ui);
        if ui.button(tr("menu.view.zoom_in")).clicked() {
            app.zoom_step(true);
        }
        if ui.button(tr("menu.view.zoom_out")).clicked() {
            app.zoom_step(false);
        }
        if ui.button(tr("toolbar.zoom_to_page")).clicked() {
            app.zoom_to_page();
        }
        if ui.button(tr("toolbar.zoom_to_fit")).clicked() {
            app.zoom_to_fit();
        }
        if ui.button(tr("toolbar.zoom_to_selected")).clicked() {
            app.zoom_to_selection();
        }
    });
}

/// Window > Toolbars > Transform.
pub fn transform_toolbar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        let has = !app.selection.is_empty();
        let mut b = app.selection_bounds().unwrap_or_default();
        let (x0, y0, w, h) = (b.x0, b.y0, b.width(), b.height());
        let mut changed = false;
        // Positions count from the ruler origin.
        let o = app.ruler_origin();
        let (mut rx, mut ry) = (b.x0 - o.x, b.y0 - o.y);
        changed |= unit_value(ui, app, "x", &mut rx, 0.5);
        changed |= unit_value(ui, app, "y", &mut ry, 0.5);
        b.x0 = rx + o.x;
        b.y0 = ry + o.y;
        let mut nw = w;
        let mut nh = h;
        changed |= unit_value(ui, app, "w", &mut nw, 0.5);
        changed |= unit_value(ui, app, "h", &mut nh, 0.5);
        if changed && has && w > 1e-9 && h > 1e-9 {
            let t = tracedraw_core::Affine::translate((b.x0, b.y0))
                * tracedraw_core::Affine::scale_non_uniform(nw / w, nh / h)
                * tracedraw_core::Affine::translate((-x0, -y0));
            app.transform_selection(t);
        }
        vsep(ui);
        if ui
            .add_enabled(has, egui::Button::new(tr("toolbar.mirror_h_short")))
            .clicked()
        {
            app.mirror(true);
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("toolbar.mirror_v_short")))
            .clicked()
        {
            app.mirror(false);
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("toolbar.rotate_90")))
            .clicked()
        {
            let c = app.selection_bounds().unwrap_or_default().center();
            app.transform_selection(tracedraw_core::Affine::rotate_about(
                std::f64::consts::FRAC_PI_2,
                c,
            ));
        }
    });
}

fn unit_value(ui: &mut Ui, app: &App, label: &str, mm: &mut f64, speed: f64) -> bool {
    ui.label(
        egui::RichText::new(label)
            .color(Tokens::text_dim())
            .size(11.0),
    );
    // Positions and sizes of the selection: through the drawing scale.
    let mut v = app.units.world_from_mm(*mm);
    let r = ui.add(
        crate::ui::field::NumField::new(&mut v)
            .speed(speed)
            .fixed_decimals(app.settings.precision.min(10) as usize)
            .suffix(format!(" {}", app.units.short())),
    );
    if r.changed() {
        *mm = app.units.world_to_mm(v);
    }
    r.drag_stopped() || r.lost_focus()
}

pub fn property_bar(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.set_min_height(crate::ui::propbar::BAR_H);
        // Fields and lists are white with a grey border.
        let v = ui.visuals_mut();
        v.widgets.inactive.weak_bg_fill = crate::theme::Tokens::field();
        v.widgets.hovered.weak_bg_fill = crate::theme::Tokens::field();
        let shapes = app.selected_shapes();
        match app.tool {
            Tool::Shape => shape_tool_bar(app, ui),
            Tool::Pick | Tool::FreeformPick if shapes.is_empty() => page_properties(app, ui),
            Tool::Pick | Tool::FreeformPick
                if shapes
                    .iter()
                    .all(|s| matches!(s.kind, ShapeKind::Text { .. })) =>
            {
                // Text selected with the Pick tool: the object fields, then
                // the text fields.
                object_properties(app, ui);
                vsep(ui);
                if let Some(first) = shapes.first() {
                    let id = first.id;
                    app.sync_text_defaults_from(id);
                }
                text_properties(app, ui);
                crate::ui::propbar::arrange_part(app, ui);
            }
            Tool::Pick | Tool::FreeformPick
                if shapes
                    .iter()
                    .all(|s| crate::connectors::Connection::of(s).is_some()) =>
            {
                // Connectors selected with the Pick tool: the object
                // fields, then how they run and their outline.
                object_properties(app, ui);
                vsep(ui);
                crate::ui::connector_bar::connector_bar(app, ui);
            }
            Tool::Pick | Tool::FreeformPick if app.fitted_text().is_some() => {
                // A text fitted to a path: the object fields, then how it
                // follows the path.
                object_properties(app, ui);
                vsep(ui);
                crate::ui::text_path_bar::text_path_bar(app, ui);
            }
            Tool::Pick | Tool::FreeformPick
                if shapes
                    .iter()
                    .all(|s| crate::callouts::Callout::of(s).is_some()) =>
            {
                // Callouts selected with the Pick tool: the object fields,
                // then the callout style.
                object_properties(app, ui);
                vsep(ui);
                crate::ui::callout_bar::callout_bar(app, ui);
            }
            Tool::Pick | Tool::FreeformPick
                if shapes
                    .iter()
                    .all(|s| crate::dimensions::Dimension::of(s).is_some()) =>
            {
                // Dimensions selected with the Pick tool: the object
                // fields, then the dimension style.
                object_properties(app, ui);
                vsep(ui);
                crate::ui::dimension_bar::dimension_bar(app, ui);
            }
            Tool::Pick | Tool::FreeformPick
                if shapes
                    .iter()
                    .all(|s| matches!(s.kind, ShapeKind::Bitmap { .. })) =>
            {
                // Bitmaps selected with the Pick tool: the object fields,
                // then the bitmap commands of the Bitmaps menu.
                object_properties(app, ui);
                vsep(ui);
                bitmap_properties(app, ui);
                crate::ui::propbar::arrange_part(app, ui);
            }
            Tool::Pick | Tool::FreeformPick => {
                object_properties(app, ui);
                crate::ui::propbar::sep(ui);
                if crate::ui::propbar::kind_part(app, ui) {
                    crate::ui::propbar::sep(ui);
                }
                crate::ui::propbar::outline_part(app, ui);
                crate::ui::propbar::sep(ui);
                crate::ui::propbar::order_part(app, ui);
                // Several objects, a group or a curve of several parts:
                // group, combine and shape them.
                crate::ui::propbar::arrange_part(app, ui);
            }
            Tool::Zoom | Tool::Pan => crate::ui::propbar::zoom_bar(app, ui),
            Tool::Rectangle | Tool::ThreePointRectangle => {
                object_properties(app, ui);
                crate::ui::propbar::sep(ui);
                crate::ui::propbar::rectangle_part(app, ui);
                if !app.selection.is_empty() {
                    crate::ui::propbar::sep(ui);
                    crate::ui::propbar::outline_part(app, ui);
                    crate::ui::propbar::sep(ui);
                    crate::ui::propbar::order_part(app, ui);
                }
            }
            Tool::Ellipse | Tool::ThreePointEllipse => {
                object_properties(app, ui);
                crate::ui::propbar::sep(ui);
                crate::ui::propbar::ellipse_part(app, ui);
                if !app.selection.is_empty() {
                    crate::ui::propbar::sep(ui);
                    crate::ui::propbar::outline_part(app, ui);
                    crate::ui::propbar::sep(ui);
                    crate::ui::propbar::order_part(app, ui);
                }
            }
            Tool::Polygon | Tool::Star => {
                object_properties(app, ui);
                crate::ui::propbar::sep(ui);
                let star = app.tool == Tool::Star;
                crate::ui::propbar::polygon_part(app, ui, star);
                if !app.selection.is_empty() {
                    crate::ui::propbar::sep(ui);
                    crate::ui::propbar::outline_part(app, ui);
                    crate::ui::propbar::sep(ui);
                    crate::ui::propbar::order_part(app, ui);
                }
            }
            Tool::Text => text_properties(app, ui),
            Tool::Freehand
            | Tool::Bezier
            | Tool::Pen
            | Tool::Polyline
            | Tool::TwoPointLine
            | Tool::BSpline
            | Tool::ThreePointCurve => curve_tool_properties(app, ui),
            Tool::Blend => crate::ui::effect_bars::blend_bar(app, ui),
            Tool::Extrude => crate::ui::extrude_bar::extrude_bar(app, ui),
            Tool::Envelope => crate::ui::envelope_ui::envelope_bar(app, ui),
            Tool::Distort => crate::ui::distort_bar::distort_bar(app, ui),
            Tool::Eraser => {
                ui.label(
                    egui::RichText::new(tr("toolbar.eraser_thickness"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                let mut w = app.units.from_mm(app.eraser_width);
                if ui
                    .add(
                        crate::ui::field::NumField::new(&mut w)
                            .speed(0.1)
                            .range(0.01..=1000.0)
                            .suffix(format!(" {}", app.units.short())),
                    )
                    .changed()
                {
                    app.eraser_width = app.units.to_mm(w).max(0.01);
                }
                if ui
                    .add(egui::Button::new("\u{25cf}").selected(!app.eraser_square))
                    .rich_tip(tr("toolbar.eraser_round"))
                    .clicked()
                {
                    app.eraser_square = false;
                }
                if ui
                    .add(egui::Button::new("\u{25a0}").selected(app.eraser_square))
                    .rich_tip(tr("toolbar.eraser_square"))
                    .clicked()
                {
                    app.eraser_square = true;
                }
                ui.label(
                    egui::RichText::new(tr("hint.bar_eraser"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
            }
            Tool::Smooth
            | Tool::Smear
            | Tool::Twirl
            | Tool::AttractRepel
            | Tool::Smudge
            | Tool::Roughen => crate::ui::brush_bars::brush_bar(app, ui),
            Tool::VariableOutline => crate::variable_outline::variable_outline_bar(app, ui),
            Tool::Contour => crate::ui::contour_bar::contour_bar(app, ui),
            Tool::Crop => crate::ui::crop_bar::crop_bar(app, ui),
            Tool::Knife => crate::ui::drawing_bars::knife_bar(app, ui),
            Tool::SegmentDelete => {
                ui.label(
                    egui::RichText::new(tr("segment_delete.hint"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
            }
            Tool::Spiral => {
                ui.label(
                    egui::RichText::new(tr("toolbar.revolutions"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                ui.add(crate::ui::field::NumField::new(&mut app.spiral_revolutions).range(1..=100));
                if ui
                    .selectable_label(!app.spiral_logarithmic, tr("toolbar.symmetrical"))
                    .clicked()
                {
                    app.spiral_logarithmic = false;
                }
                if ui
                    .selectable_label(app.spiral_logarithmic, tr("toolbar.logarithmic"))
                    .clicked()
                {
                    app.spiral_logarithmic = true;
                }
            }
            Tool::GraphPaper => {
                ui.label(
                    egui::RichText::new(tr("toolbar.rows"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                ui.add(crate::ui::field::NumField::new(&mut app.graph_rows).range(1..=99));
                ui.label(
                    egui::RichText::new(tr("toolbar.columns"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                ui.add(crate::ui::field::NumField::new(&mut app.graph_cols).range(1..=99));
            }
            Tool::ActionLines => {
                ui.label(
                    egui::RichText::new(tr("toolbar.lines"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                ui.add(crate::ui::field::NumField::new(&mut app.action_lines_count).range(2..=500));
                if ui
                    .selectable_label(!app.action_lines_radial, tr("toolbar.parallel"))
                    .clicked()
                {
                    app.action_lines_radial = false;
                }
                if ui
                    .selectable_label(app.action_lines_radial, tr("toolbar.radial"))
                    .clicked()
                {
                    app.action_lines_radial = true;
                }
            }
            Tool::CommonShapes => common_shapes_bar(app, ui),
            Tool::Table => crate::ui::table_bar::table_bar(app, ui),
            Tool::PainterlyBrush => crate::painterly_tool::painterly_bar(app, ui),
            Tool::BrushStrokes => {
                ui.label(
                    egui::RichText::new(tr("media.calligraphic"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                ui.label(
                    egui::RichText::new(tr("toolbar.width"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                ui.add(
                    crate::ui::field::NumField::new(&mut app.media_width)
                        .speed(0.1)
                        .range(0.2..=50.0)
                        .suffix(" mm"),
                );
                ui.label(
                    egui::RichText::new(tr("docker.nib_angle"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                ui.add(
                    crate::ui::field::NumField::new(&mut app.media_angle)
                        .speed(1.0)
                        .range(0.0..=180.0)
                        .suffix("°"),
                );
            }
            Tool::ParallelDimension
            | Tool::HorizontalVerticalDimension
            | Tool::SegmentDimension
            | Tool::AngularDimension => crate::ui::dimension_bar::dimension_bar(app, ui),
            Tool::Callout => crate::ui::callout_bar::callout_bar(app, ui),
            Tool::Connector | Tool::RightAngleConnector | Tool::RoundedConnector => {
                crate::ui::connector_bar::connector_bar(app, ui)
            }
            Tool::AnchorEditing => {
                let id = app.selection.first().copied();
                let n = id
                    .and_then(|id| app.doc().find_shape(id))
                    .map(|s| crate::anchors::custom_anchors(&s.data).len())
                    .unwrap_or(0);
                ui.label(
                    egui::RichText::new(trf("toolbar.anchors_n", &[("n", &n.to_string())]))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                if ui
                    .add_enabled(
                        app.anchor_sel.is_some(),
                        egui::Button::new(tr("toolbar.anchor_delete")),
                    )
                    .clicked()
                {
                    app.delete_selected_anchor();
                }
                if ui
                    .add_enabled(n > 0, egui::Button::new(tr("toolbar.anchor_clear")))
                    .clicked()
                {
                    if let Some(id) = id {
                        app.clear_anchors(id);
                    }
                }
                ui.label(
                    egui::RichText::new(tr("status.anchor_hint"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
            }
            Tool::DropShadow => shadow_bar(app, ui, &shapes),
            Tool::Transparency => {
                transparency_part(app, ui);
                if shapes.is_empty() {
                    ui.label(
                        egui::RichText::new(tr("transparency.hint"))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                }
            }
            Tool::FreeTransform => {
                for m in crate::app::FreeTransformMode::ALL {
                    if ui
                        .selectable_label(app.free_transform_mode == m, tr(m.label_key()))
                        .clicked()
                    {
                        app.free_transform_mode = m;
                    }
                }
                vsep(ui);
                ui.checkbox(
                    &mut app.free_transform_duplicate,
                    tr("toolbar.apply_to_duplicate"),
                );
                vsep(ui);
                if !shapes.is_empty() {
                    object_properties(app, ui);
                } else {
                    ui.label(
                        egui::RichText::new(tr("hint.bar_free_transform"))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                }
            }
            Tool::ColorEyedropper => {
                let has = app.eyedropper_color.is_some();
                if ui
                    .selectable_label(!app.eyedropper_apply, tr("toolbar.select_color"))
                    .clicked()
                {
                    app.eyedropper_apply = false;
                }
                if ui
                    .add_enabled(
                        has,
                        egui::Button::selectable(app.eyedropper_apply, tr("toolbar.apply_color")),
                    )
                    .clicked()
                {
                    app.eyedropper_apply = true;
                }
                vsep(ui);
                ui.label(
                    egui::RichText::new(tr("toolbar.sample_size"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                for n in [1u32, 2, 5] {
                    if ui
                        .selectable_label(app.eyedropper_sample == n, format!("{n}x{n}"))
                        .clicked()
                    {
                        app.eyedropper_sample = n;
                    }
                }
                vsep(ui);
                if let Some(c) = app.eyedropper_color {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                    let [cr, cg, cb] = c.to_rgb8();
                    ui.painter()
                        .rect_filled(r, 2.0, egui::Color32::from_rgb(cr, cg, cb));
                    ui.painter().rect_stroke(
                        r,
                        2.0,
                        egui::Stroke::new(1.0, Tokens::border()),
                        egui::StrokeKind::Inside,
                    );
                    ui.label(
                        egui::RichText::new(crate::app::color_description(c))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(tr("hint.bar_eyedropper"))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                }
            }
            Tool::AttributesEyedropper => {
                let has = app.eyedropper_attrs.is_some();
                if ui
                    .selectable_label(!app.eyedropper_apply, tr("toolbar.select_attrs"))
                    .clicked()
                {
                    app.eyedropper_apply = false;
                }
                if ui
                    .add_enabled(
                        has,
                        egui::Button::selectable(app.eyedropper_apply, tr("toolbar.apply_attrs")),
                    )
                    .clicked()
                {
                    app.eyedropper_apply = true;
                }
                vsep(ui);
                let g = &mut app.eyedropper_groups;
                ui.menu_button(tr("toolbar.attr_properties"), |ui| {
                    ui.checkbox(&mut g.outline, tr("toolbar.attr_outline"));
                    ui.checkbox(&mut g.fill, tr("toolbar.attr_fill"));
                    ui.checkbox(&mut g.text, tr("toolbar.attr_text"));
                });
                ui.menu_button(tr("toolbar.attr_transformations"), |ui| {
                    ui.checkbox(&mut g.size, tr("toolbar.attr_size"));
                    ui.checkbox(&mut g.rotation, tr("toolbar.attr_rotation"));
                    ui.checkbox(&mut g.position, tr("toolbar.attr_position"));
                });
                ui.checkbox(&mut g.effects, tr("toolbar.attr_effects"));
                vsep(ui);
                ui.label(
                    egui::RichText::new(if has {
                        tr("hint.bar_attrs_apply")
                    } else {
                        tr("hint.bar_attrs")
                    })
                    .color(Tokens::text_dim())
                    .size(11.0),
                );
            }
            Tool::BlockShadow => {
                use tracedraw_core::{geometry::Vec2 as CVec2, live::Effect};
                // The selected object's block shadow, else the defaults.
                let current = shapes.first().and_then(|s| {
                    s.effects.iter().find_map(|e| match e {
                        Effect::BlockShadow { offset, color, gap } => Some((*offset, *color, *gap)),
                        _ => None,
                    })
                });
                let (mut offset, mut color, mut gap) = current.unwrap_or((
                    CVec2::new(5.0, -5.0),
                    app.block_shadow_color,
                    app.block_shadow_gap,
                ));
                let mut changed = false;
                let mut depth = offset.hypot();
                let mut dir = offset.y.atan2(offset.x).to_degrees();
                ui.label(
                    egui::RichText::new(tr("toolbar.depth"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                changed |= ui
                    .add(
                        crate::ui::field::NumField::new(&mut depth)
                            .speed(0.1)
                            .range(0.0..=500.0)
                            .fixed_decimals(2)
                            .suffix(" mm"),
                    )
                    .changed();
                ui.label(
                    egui::RichText::new(tr("toolbar.direction"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                changed |= ui
                    .add(
                        crate::ui::field::NumField::new(&mut dir)
                            .speed(1.0)
                            .range(-180.0..=180.0)
                            .fixed_decimals(1)
                            .suffix("°"),
                    )
                    .changed();
                if changed {
                    let a = dir.to_radians();
                    offset = CVec2::new(depth * a.cos(), depth * a.sin());
                }
                let mut rgb = color.to_rgb8();
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    color = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
                    changed = true;
                }
                ui.label(
                    egui::RichText::new(tr("toolbar.gap"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                changed |= ui
                    .add(
                        crate::ui::field::NumField::new(&mut gap)
                            .speed(0.05)
                            .range(0.0..=50.0)
                            .fixed_decimals(2)
                            .suffix(" mm"),
                    )
                    .changed();
                if changed {
                    app.block_shadow_color = color;
                    app.block_shadow_gap = gap;
                    if current.is_some() {
                        app.push_effect(Effect::BlockShadow { offset, color, gap }, true);
                    }
                }
                vsep(ui);
                if ui
                    .add_enabled(
                        current.is_some(),
                        egui::Button::new(tr("toolbar.clear_block_shadow")),
                    )
                    .clicked()
                {
                    app.remove_effects_of_kind(&Effect::BlockShadow {
                        offset: CVec2::ZERO,
                        color,
                        gap,
                    });
                }
                if current.is_none() {
                    ui.label(
                        egui::RichText::new(tr("hint.bar_block_shadow"))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                }
            }
            Tool::InteractiveFill => interactive_fill_bar(app, ui, &shapes),
            Tool::ShapeRecognition | Tool::Sketch => crate::ui::drawing_bars::drawing_bar(app, ui),
            Tool::AreaFill => crate::ui::fill_tool_bars::smart_fill_bar(app, ui),
            Tool::MeshFill => crate::ui::fill_tool_bars::mesh_bar(app, ui),
            t => {
                ui.label(
                    egui::RichText::new(trf("toolbar.tool_n", &[("t", &t.name())]))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
                if !t.implemented() {
                    ui.label(
                        egui::RichText::new(tr("toolbar.not_implemented"))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                }
            }
        }
    });
}

fn page_properties(app: &mut App, ui: &mut Ui) {
    crate::ui::propbar::page_bar(app, ui);
}

/// The object part of the property bar (position, size, scale, rotation,
/// mirroring); the page's when nothing is selected.
/// The curve tools' bar: the selection's position and size, then (with a
/// selection) its outline width, line style and arrowheads and Close
/// Curve; the Freehand tool's smoothing; the Pen's preview mode and
/// automatic adding and deleting of nodes; with nothing selected, how to
/// finish a curve.
fn curve_tool_properties(app: &mut App, ui: &mut Ui) {
    use crate::ui::propbar::{pic_button, sep, Pic};
    object_properties(app, ui);
    let selected = !app.selection.is_empty();
    if selected {
        sep(ui);
        crate::ui::propbar::outline_part(app, ui);
        let open = app.selected_shapes().iter().any(|s| match &s.kind {
            ShapeKind::Path { path, .. } => !path
                .elements()
                .iter()
                .any(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath)),
            _ => false,
        });
        if pic_button(ui, Pic::CloseCurve, &tr("toolbar.close_tip"), open, false).clicked() {
            app.close_selected_curves();
        }
    }
    if app.tool == Tool::Freehand {
        sep(ui);
        let mut v = app.settings.curves.smoothing.min(100);
        ui.add(crate::ui::Rail(
            egui::Slider::new(&mut v, 0..=100).show_value(true),
        ))
        .rich_tip(tr("toolbar.freehand_smoothing"));
        app.settings.curves.smoothing = v;
    }
    if app.tool == Tool::Pen {
        // Preview mode and automatic adding and deleting of nodes, also in
        // Options > Tools > Curve tools.
        sep(ui);
        let c = &mut app.settings.curves;
        if pic_button(
            ui,
            Pic::PenPreview,
            &tr("toolbar.pen_preview_tip"),
            true,
            c.pen_preview,
        )
        .clicked()
        {
            c.pen_preview = !c.pen_preview;
        }
        if pic_button(
            ui,
            Pic::AutoAddDelete,
            &tr("toolbar.pen_auto_add_delete_tip"),
            true,
            c.pen_auto_add_delete,
        )
        .clicked()
        {
            c.pen_auto_add_delete = !c.pen_auto_add_delete;
        }
    }
    if !selected {
        vsep(ui);
        let hint = if app.tool == Tool::ThreePointCurve {
            tr("toolbar.three_point_hint")
        } else {
            tr("toolbar.curve_finish_hint")
        };
        ui.label(
            egui::RichText::new(hint)
                .color(Tokens::text_dim())
                .size(11.0),
        );
    }
}

fn object_properties(app: &mut App, ui: &mut Ui) {
    if app.selection_bounds().is_none() {
        page_properties(app, ui);
        return;
    }
    crate::ui::propbar::object_bar(app, ui);
}

/// The bitmap part of the Pick tool's property bar: the commands of the
/// Bitmaps menu that act on the selected bitmap.
fn bitmap_properties(app: &mut App, ui: &mut Ui) {
    use crate::ui::dialogs::{Dialog, TraceState};
    if ui.button(tr("toolbar.edit_bitmap")).clicked() {
        app.edit_bitmap_externally();
    }
    if ui.button(tr("toolbar.crop_bitmap")).clicked() {
        app.set_tool(Tool::Crop);
    }
    ui.menu_button(tr("toolbar.trace_bitmap"), |ui| {
        if ui.button(tr("menu.bitmaps.quick_trace")).clicked() {
            app.quick_trace();
            ui.close();
        }
        ui.separator();
        for (key, preset) in [
            (
                "menu.bitmaps.trace_technical",
                crate::trace::Preset::Technical,
            ),
            (
                "menu.bitmaps.trace_line_drawing",
                crate::trace::Preset::LineDrawing,
            ),
            ("menu.bitmaps.trace_line_art", crate::trace::Preset::LineArt),
            ("menu.bitmaps.trace_logo", crate::trace::Preset::Logo),
            (
                "menu.bitmaps.trace_detailed_logo",
                crate::trace::Preset::DetailedLogo,
            ),
            ("menu.bitmaps.trace_clipart", crate::trace::Preset::Clipart),
            (
                "menu.bitmaps.trace_low_quality",
                crate::trace::Preset::LowQualityImage,
            ),
            (
                "menu.bitmaps.trace_high_quality",
                crate::trace::Preset::HighQualityImage,
            ),
        ] {
            if ui.button(tr(key)).clicked() {
                app.dialog = Dialog::Trace(TraceState::new(preset, &app.settings.tracing));
                ui.close();
            }
        }
    });
    if ui.button(tr("menu.bitmaps.straighten_image")).clicked() {
        app.dialog = Dialog::StraightenImage(Default::default());
    }
    if ui.button(tr("menu.bitmaps.resample")).clicked() {
        app.open_resample();
    }
    if ui
        .button(tr("menu.bitmaps.remove_background_local"))
        .on_hover_text(tr("dialog.bg_contiguous_tip"))
        .clicked()
    {
        app.dialog = crate::ui::dialogs::Dialog::RemoveBackground(Default::default());
    }
    // AI: greyed until the AI service makes pictures.
    let ready = app.ai_image_ready() && app.ai_source_bitmap().is_some();
    let busy = app.ai_image.busy();
    let tip = app
        .ai_image_tip()
        .unwrap_or_else(|| tr("toolbar.remove_background_tip"));
    if ui
        .add_enabled(
            ready && !busy,
            egui::Button::new(format!("\u{2726} {}", tr("toolbar.remove_background"))),
        )
        .rich_tip(tip.clone())
        .rich_disabled_tip(tip)
        .clicked()
    {
        app.ai_remove_background();
    }
    if busy {
        ui.spinner();
    }
}

/// The point sizes the font size list offers.
pub const TEXT_SIZES: [f64; 22] = [
    6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 40.0,
    48.0, 60.0, 72.0, 96.0, 144.0, 288.0,
];

/// The alignments of the alignment list, with their pictures and names.
const TEXT_ALIGNS: [(tracedraw_core::TextAlign, crate::ui::propbar::Pic, &str); 4] = [
    (
        tracedraw_core::TextAlign::Left,
        crate::ui::propbar::Pic::TextLeft,
        "toolbar.align_left",
    ),
    (
        tracedraw_core::TextAlign::Center,
        crate::ui::propbar::Pic::TextCenter,
        "toolbar.align_center",
    ),
    (
        tracedraw_core::TextAlign::Right,
        crate::ui::propbar::Pic::TextRight,
        "toolbar.align_right",
    ),
    (
        tracedraw_core::TextAlign::Justify,
        crate::ui::propbar::Pic::TextJustify,
        "toolbar.full_justify",
    ),
];

/// Left to right and Right to left: which way the selected text (the
/// paragraph at the caret while text in a frame is typed) runs. The
/// button of the way it runs now is down.
pub fn text_direction_buttons(app: &mut App, ui: &mut Ui) {
    use tracedraw_core::TextDirection;
    let current = app
        .selected_shapes()
        .iter()
        .find(|s| matches!(s.kind, ShapeKind::Text { .. }))
        .and_then(|s| app.text_runs_rtl(s));
    for (d, label, key) in [
        (TextDirection::Ltr, "\u{00b6}\u{2192}", "toolbar.text_ltr"),
        (TextDirection::Rtl, "\u{2190}\u{00b6}", "toolbar.text_rtl"),
    ] {
        let on = current == Some(d == TextDirection::Rtl);
        if ui
            .add_sized(
                [26.0, 22.0],
                egui::Button::selectable(on, egui::RichText::new(label).size(13.0)),
            )
            .on_hover_text(tr(key))
            .clicked()
        {
            app.apply_text_direction(d);
        }
    }
}

/// The text part of the property bar: the font list, the size list, bold,
/// italic and underline, the alignment list, bullets and drop cap (for
/// paragraph text), the Text docker and Edit Text.
fn text_properties(app: &mut App, ui: &mut Ui) {
    use crate::ui::propbar::{pic_button, Pic};
    let mut changed = crate::ui::font_list::text_font_list(app, ui, "propbar_font", 180.0);
    changed |= text_size_list(app, ui);
    for (on, label, key) in [
        (
            app.text_bold,
            egui::RichText::new("B").strong(),
            "toolbar.bold",
        ),
        (
            app.text_italic,
            egui::RichText::new("I").italics(),
            "toolbar.italic",
        ),
        (
            app.text_underline,
            egui::RichText::new("U").underline(),
            "toolbar.underline",
        ),
    ] {
        if ui
            .add_sized([22.0, 22.0], egui::Button::selectable(on, label.size(14.0)))
            .rich_tip(tr(key))
            .clicked()
        {
            match key {
                "toolbar.bold" => app.text_bold = !app.text_bold,
                "toolbar.italic" => app.text_italic = !app.text_italic,
                _ => app.text_underline = !app.text_underline,
            }
            changed = true;
        }
    }
    vsep(ui);
    if text_align_list(app, ui) {
        let a = app.text_align;
        app.apply_text_align(a);
    }
    text_direction_buttons(app, ui);
    vsep(ui);
    let shapes = app.selected_shapes();
    let is_text = shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Text { .. }));
    let is_para = shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Text { frame: Some(_), .. }));
    if pic_button(ui, Pic::Bullets, &tr("menu.text.bullets"), is_para, false).clicked() {
        app.dialog = crate::ui::dialogs::Dialog::TextBullets;
    }
    if pic_button(ui, Pic::DropCap, &tr("menu.text.drop_cap"), is_para, false).clicked() {
        app.dialog = crate::ui::dialogs::Dialog::TextDropCap;
    }
    vsep(ui);
    let docker_open = app.show_dockers && app.docker_tab == crate::app::DockerTab::Text;
    if pic_button(
        ui,
        Pic::TextProperties,
        &format!("{} (Ctrl+T)", tr("toolbar.text_properties")),
        true,
        docker_open,
    )
    .clicked()
    {
        app.toggle_docker(crate::app::DockerTab::Text);
    }
    if pic_button(
        ui,
        Pic::EditText,
        &format!("{} (Ctrl+Shift+T)", tr("menu.text.edit_text")),
        is_text,
        false,
    )
    .clicked()
    {
        app.edit_selected_text();
    }
    if changed {
        app.apply_text_style();
    }
}

/// The transparency settings of the Transparency tool's bar: the kind
/// (none, uniform, fountain, pattern, texture), the amount or the
/// fountain's angle and kind, the merge mode and what it applies to, in
/// one row; a change applies to the selection.
pub fn transparency_part(app: &mut App, ui: &mut Ui) {
    transparency_controls(app, ui, false);
}

/// The same settings for the Properties docker, in rows that keep to the
/// docker's width: the kind and what it applies to, then the amount (or
/// the fountain's angle and kind), then the merge mode.
pub fn transparency_section(app: &mut App, ui: &mut Ui) {
    ui.vertical(|ui| transparency_controls(app, ui, true));
}

/// `f` in a row of its own when `stacked`, else straight in `ui`.
fn part_row(ui: &mut Ui, stacked: bool, f: impl FnOnce(&mut Ui)) {
    if stacked {
        ui.horizontal(f);
    } else {
        f(ui);
    }
}

fn transparency_controls(app: &mut App, ui: &mut Ui, stacked: bool) {
    use crate::mixed::{transparency_mixed, TransparencyField as F};
    let kinds = [
        tr("transparency.none"),
        tr("transparency.uniform"),
        tr("transparency.fountain"),
        tr("transparency.pattern"),
        tr("transparency.texture"),
    ];
    // Reflect the first selected object in the bar; settings that differ
    // across the selection show as mixed, and a change sends only that
    // setting to each object.
    let ts = app.selection_transparency();
    let mixed = transparency_mixed(&ts);
    let has_selection = !ts.is_empty();
    let (mut set, current) = match ts.first() {
        Some(t) => {
            let mut set = app.transparency_default.clone();
            if t.kind >= 1 {
                set.kind = (t.kind - 1) as u8;
            }
            if t.kind == 1 {
                set.amount = t.amount.round();
            }
            if let Some(m) = &t.mask {
                set.mask = m.clone();
                set.merge = t.merge;
                set.target = t.target;
            }
            (set, (!mixed.contains(&F::Kind)).then_some(t.kind))
        }
        None => (app.transparency_default.clone(), Some(0)),
    };
    let mut field = None;
    let targets = [
        tr("transparency.target_fill"),
        tr("transparency.target_outline"),
        tr("transparency.target_all"),
    ];
    let target_shown = !mixed.contains(&F::Target);
    let target_list =
        |ui: &mut Ui, set: &mut crate::effects_ui::TransparencySettings, field: &mut Option<F>| {
            egui::ComboBox::from_id_salt("transparency_target")
                .selected_text(if target_shown {
                    targets[(set.target as usize).min(2)].clone()
                } else {
                    String::new()
                })
                .width(80.0)
                .show_ui(ui, |ui| {
                    for (i, t) in targets.iter().enumerate() {
                        if ui
                            .selectable_label(target_shown && set.target as usize == i, t)
                            .clicked()
                        {
                            set.target = i as u8;
                            *field = Some(F::Target);
                        }
                    }
                });
        };
    part_row(ui, stacked, |ui| {
        egui::ComboBox::from_id_salt("transparency_kind")
            .selected_text(
                current
                    .and_then(|c| kinds.get(c).cloned())
                    .unwrap_or_default(),
            )
            .width(110.0)
            .show_ui(ui, |ui| {
                for (i, k) in kinds.iter().enumerate() {
                    if ui.selectable_label(current == Some(i), k).clicked() {
                        if i == 0 {
                            app.clear_transparency();
                        } else {
                            set.set_kind((i - 1) as u8);
                            field = Some(F::Kind);
                        }
                    }
                }
            });
        // In the docker, what it applies to goes beside the kind.
        if stacked {
            target_list(ui, &mut set, &mut field);
        }
    });
    part_row(ui, stacked, |ui| {
        let kind_mixed = mixed.contains(&F::Kind);
        if set.kind == 0 && !kind_mixed {
            let amount_mixed = mixed.contains(&F::Amount);
            let mut amount = set.amount;
            let mut slider = egui::Slider::new(&mut amount, 0.0..=100.0).suffix(" %");
            if amount_mixed {
                slider = slider.text(tr("docker.mixed"));
            }
            if ui
                .add_enabled(has_selection, crate::ui::Rail(slider))
                .changed()
            {
                set.amount = amount;
                field = Some(F::Amount);
            }
        } else if set.kind == 1 && !kind_mixed {
            if let Fill::Fountain(f) = &mut set.mask {
                let mut angle = f.angle;
                if ui
                    .add(
                        crate::ui::field::NumField::new(&mut angle)
                            .suffix("°")
                            .speed(1.0)
                            .mixed(mixed.contains(&F::MaskAngle)),
                    )
                    .changed()
                {
                    f.angle = angle;
                    field = Some(F::MaskAngle);
                }
                let kinds = tracedraw_core::style::FountainKind::ALL
                    .map(|k| (k, crate::ui::dockers::fountain_kind_key(k)));
                let kind_shown = !mixed.contains(&F::MaskKind);
                let name = kinds
                    .iter()
                    .find(|(k, _)| kind_shown && *k == f.kind)
                    .map(|(_, n)| tr(n))
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("transparency_fountain_kind")
                    .selected_text(name)
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for (k, n) in kinds {
                            if ui
                                .selectable_label(kind_shown && f.kind == k, tr(n))
                                .clicked()
                            {
                                f.kind = k;
                                field = Some(F::MaskKind);
                            }
                        }
                    });
            }
        }
    });
    part_row(ui, stacked, |ui| {
        if !stacked {
            vsep(ui);
        }
        ui.add(
            egui::Label::new(
                egui::RichText::new(tr("transparency.merge_mode"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            )
            .extend(),
        );
        let merge_shown = !mixed.contains(&F::Merge);
        egui::ComboBox::from_id_salt("transparency_merge")
            .selected_text(if merge_shown {
                set.merge.name().to_string()
            } else {
                String::new()
            })
            .width(100.0)
            .show_ui(ui, |ui| {
                for m in tracedraw_core::live::MergeMode::ALL {
                    if ui
                        .selectable_label(merge_shown && set.merge == m, m.name())
                        .clicked()
                    {
                        set.merge = m;
                        field = Some(F::Merge);
                    }
                }
            });
        if !stacked {
            target_list(ui, &mut set, &mut field);
        }
    });
    if let Some(f) = field {
        app.transparency_default = set.clone();
        if has_selection {
            app.patch_transparency(&set, &[f]);
        }
    }
}

/// The font size: a field and an arrow dropping the usual sizes.
pub(crate) fn text_size_list(app: &mut App, ui: &mut Ui) -> bool {
    let mut size = app.text_size_pt;
    let mut picked: Option<f64> = None;
    let ctx = ui.ctx().clone();
    let popup_id = ui.make_persistent_id("text_size_list");
    let inner = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let field = ui
            .add(
                crate::ui::field::NumField::new(&mut size)
                    .speed(0.5)
                    .range(1.0..=3000.0)
                    .suffix(" pt")
                    .width(62.0),
            )
            .rich_tip(tr("toolbar.font_size"));
        let (arrow_rect, arrow) =
            ui.allocate_exact_size(egui::vec2(16.0, field.rect.height()), egui::Sense::click());
        if arrow.hovered() {
            ui.painter()
                .rect_filled(arrow_rect.shrink(1.0), 1.0, Tokens::tool_hover());
        }
        let c = arrow_rect.center();
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + egui::vec2(-4.0, -2.0),
                c + egui::vec2(4.0, -2.0),
                c + egui::vec2(0.0, 3.0),
            ],
            Tokens::icon(),
            egui::Stroke::NONE,
        ));
        if arrow.clicked() {
            egui::Popup::toggle_id(&ctx, popup_id);
        }
        let anchor = field.rect.union(arrow_rect);
        (field, anchor)
    });
    let (field, anchor) = inner.inner;
    let current = app.text_size_pt;
    egui::Popup::new(popup_id, ctx.clone(), anchor, ui.layer_id())
        .open_memory(None)
        .width(anchor.width())
        .show(|ui| {
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for s in TEXT_SIZES {
                        let label = format!("{s} pt");
                        if ui
                            .selectable_label((current - s).abs() < 1e-6, label)
                            .clicked()
                        {
                            picked = Some(s);
                        }
                    }
                });
        });
    if let Some(s) = picked {
        app.text_size_pt = s;
        egui::Popup::close_id(&ctx, popup_id);
        return true;
    }
    if field.changed() {
        app.text_size_pt = size;
        return true;
    }
    false
}

/// The alignment: a button with the alignment's picture dropping the
/// alignments.
pub(crate) fn text_align_list(app: &mut App, ui: &mut Ui) -> bool {
    use crate::ui::propbar::{draw_pic, pic_button, Pic};
    let ctx = ui.ctx().clone();
    let popup_id = ui.make_persistent_id("text_align_list");
    let pic = TEXT_ALIGNS
        .iter()
        .find(|(a, _, _)| *a == app.text_align)
        .map_or(Pic::TextLeft, |(_, p, _)| *p);
    let button = pic_button(ui, pic, &tr("toolbar.text_alignment"), true, false);
    if button.clicked() {
        egui::Popup::toggle_id(&ctx, popup_id);
    }
    let mut picked = None;
    let current = app.text_align;
    egui::Popup::new(popup_id, ctx.clone(), button.rect, ui.layer_id())
        .open_memory(None)
        .width(170.0)
        .show(|ui| {
            for (align, pic, key) in TEXT_ALIGNS {
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(170.0, 24.0), egui::Sense::click());
                if align == current {
                    ui.painter().rect_filled(rect, 2.0, Tokens::tool_active());
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
                }
                draw_pic(
                    ui.painter(),
                    egui::Rect::from_center_size(
                        egui::pos2(rect.left() + 14.0, rect.center().y),
                        egui::vec2(16.0, 16.0),
                    ),
                    pic,
                    Tokens::icon(),
                );
                ui.painter().text(
                    egui::pos2(rect.left() + 30.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    tr(key),
                    egui::FontId::proportional(13.0),
                    Tokens::text(),
                );
                if resp.clicked() {
                    picked = Some(align);
                }
            }
        });
    match picked {
        Some(a) => {
            app.text_align = a;
            egui::Popup::close_id(&ctx, popup_id);
            true
        }
        None => false,
    }
}

fn shape_tool_bar(app: &mut App, ui: &mut Ui) {
    use crate::node_edit::NodeTransformMode;
    use crate::ui::propbar::{pic_button, sep, Pic};
    use tracedraw_core::nodes::NodeType;
    // Rectangles, ellipses and polygons: the Shape tool edits them through
    // their own controls (corners, pie and arc, points), as with their tools.
    let shapes = app.selected_shapes();
    // The kind of the object being edited first, so a rectangle still
    // being one reads apart from a curve.
    let kind_label = |ui: &mut Ui, key: &str| {
        ui.label(
            egui::RichText::new(tr(key))
                .strong()
                .color(Tokens::text_dim()),
        );
    };
    if let Some(first) = shapes.first() {
        let key = match first.kind {
            ShapeKind::Rect { .. } => Some("kind.rectangle"),
            ShapeKind::Ellipse { .. } => Some("kind.ellipse"),
            ShapeKind::Polygon { .. } => Some("toolbar.kind_polygon"),
            ShapeKind::Path { .. } => Some("kind.curve"),
            _ => None,
        };
        let same = shapes
            .iter()
            .all(|s| std::mem::discriminant(&s.kind) == std::mem::discriminant(&first.kind));
        if let (Some(key), true) = (key, same) {
            kind_label(ui, key);
            crate::ui::propbar::sep(ui);
        }
    }
    if !shapes.is_empty()
        && shapes.iter().all(|s| {
            matches!(
                s.kind,
                ShapeKind::Rect { .. } | ShapeKind::Ellipse { .. } | ShapeKind::Polygon { .. }
            )
        })
        && crate::ui::propbar::kind_part(app, ui)
    {
        return;
    }
    // Text: the chosen characters' offsets and angle.
    if !shapes.is_empty()
        && shapes
            .iter()
            .all(|s| matches!(s.kind, ShapeKind::Text { on_path: None, .. }))
    {
        crate::ui::propbar::char_offset_part(app, ui);
        crate::ui::propbar::sep(ui);
        ui.label(
            egui::RichText::new(tr("toolbar.text_nodes_hint"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        return;
    }
    let has_nodes = !app.node_selection.is_empty();
    let two_nodes = app.node_selection.len() >= 2;
    let has_curve = shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Path { .. }));
    // Selection mode: rectangular or freehand marquee.
    ui.label(tr("toolbar.selection_mode"));
    let mode_text = |lasso: bool| {
        if lasso {
            tr("toolbar.freehand")
        } else {
            tr("toolbar.rectangular")
        }
    };
    egui::ComboBox::from_id_salt("node_selection_mode")
        .selected_text(mode_text(app.node_lasso))
        .width(96.0)
        .show_ui(ui, |ui| {
            for lasso in [false, true] {
                ui.selectable_value(&mut app.node_lasso, lasso, mode_text(lasso));
            }
        });
    sep(ui);
    let button = |ui: &mut Ui, pic: Pic, tip: &str, enabled: bool, pressed: bool| -> bool {
        pic_button(ui, pic, &tr(tip), enabled, pressed).clicked()
    };
    if button(ui, Pic::AddNode, "toolbar.add_node_tip", has_nodes, false) {
        app.add_node_midpoints();
    }
    if button(
        ui,
        Pic::DeleteNode,
        "toolbar.delete_node_tip",
        has_nodes,
        false,
    ) {
        app.delete_selected_nodes();
    }
    sep(ui);
    if button(
        ui,
        Pic::JoinNodes,
        "toolbar.join_nodes_tip",
        app.node_selection.len() == 2,
        false,
    ) {
        app.join_selected_nodes();
    }
    if button(ui, Pic::BreakCurve, "toolbar.break_tip", has_nodes, false) {
        app.break_selected_nodes();
    }
    sep(ui);
    if button(ui, Pic::ToLine, "toolbar.to_line_tip", has_nodes, false) {
        app.selected_segments_to_line();
    }
    if button(ui, Pic::ToCurve, "toolbar.to_curve_tip", has_nodes, false) {
        app.selected_segments_to_curve();
    }
    sep(ui);
    let current = app.current_node_type();
    for (ty, pic, tip) in [
        (NodeType::Cusp, Pic::NodeCusp, "toolbar.cusp_tip"),
        (NodeType::Smooth, Pic::NodeSmooth, "toolbar.smooth_tip"),
        (
            NodeType::Symmetrical,
            Pic::NodeSymmetrical,
            "toolbar.symm_tip",
        ),
    ] {
        if button(ui, pic, tip, has_nodes, has_nodes && current == Some(ty)) {
            app.set_selected_node_type(ty);
        }
    }
    sep(ui);
    if button(
        ui,
        Pic::ReverseDirection,
        "toolbar.reverse_tip",
        has_curve,
        false,
    ) {
        app.reverse_selected_curves();
    }
    if button(
        ui,
        Pic::ExtendToClose,
        "toolbar.extend_close_tip",
        two_nodes,
        false,
    ) {
        app.extend_curve_to_close();
    }
    if button(
        ui,
        Pic::ExtractSubpath,
        "toolbar.extract_subpath_tip",
        has_nodes,
        false,
    ) {
        app.extract_subpath();
    }
    if button(ui, Pic::CloseCurve, "toolbar.close_tip", has_curve, false) {
        app.close_selected_curves();
    }
    sep(ui);
    for (mode, pic, tip) in [
        (
            NodeTransformMode::StretchScale,
            Pic::StretchNodes,
            "toolbar.stretch_nodes_tip",
        ),
        (
            NodeTransformMode::RotateSkew,
            Pic::RotateNodes,
            "toolbar.rotate_nodes_tip",
        ),
    ] {
        let on = app.node_transform == mode;
        if button(ui, pic, tip, has_curve, on) {
            app.node_transform = if on { NodeTransformMode::None } else { mode };
        }
    }
    if button(
        ui,
        Pic::AlignNodes,
        "toolbar.align_nodes_tip",
        two_nodes,
        false,
    ) {
        app.dialog = crate::ui::dialogs::Dialog::NodeAlign {
            horizontal: true,
            vertical: true,
        };
    }
    sep(ui);
    let (rh, rv) = app.reflect_nodes;
    if button(ui, Pic::ReflectH, "toolbar.reflect_h_tip", has_curve, rh) {
        app.reflect_nodes.0 = !rh;
    }
    if button(ui, Pic::ReflectV, "toolbar.reflect_v_tip", has_curve, rv) {
        app.reflect_nodes.1 = !rv;
    }
    if button(
        ui,
        Pic::Elastic,
        "toolbar.elastic_tip",
        has_curve,
        app.elastic_mode,
    ) {
        app.elastic_mode = !app.elastic_mode;
    }
    sep(ui);
    if button(
        ui,
        Pic::SelectAllNodes,
        "toolbar.select_all_nodes_tip",
        has_curve,
        false,
    ) {
        app.select_all_nodes();
    }
    sep(ui);
    if button(
        ui,
        Pic::ReduceNodes,
        "toolbar.reduce_nodes_tip",
        has_curve,
        false,
    ) {
        app.reduce_selected_nodes();
    }
    // Curve smoothness: dragging removes nodes from the curves as they
    // were when the drag began, in one undo step.
    let mut v = app.curve_smoothness;
    let r = ui
        .add_enabled(
            has_curve,
            crate::ui::Rail(
                egui::Slider::new(&mut v, 0.0..=100.0)
                    .show_value(true)
                    .integer(),
            ),
        )
        .rich_tip(tr("toolbar.curve_smoothness"));
    if r.changed() {
        let first = app.smoothing.is_none();
        if first {
            app.smoothing = Some(app.smoothing_curves());
        }
        app.curve_smoothness = v;
        let start = app.smoothing.clone().unwrap_or_default();
        app.smooth_curves(&start, v, !first);
    } else if !r.dragged() && app.smoothing.is_some() {
        // Let go: the next change starts a new step from the new curves.
        app.smoothing = None;
        app.curve_smoothness = 0.0;
    }
    if !has_curve && !app.selection.is_empty() {
        sep(ui);
        if pic_button(
            ui,
            Pic::ToCurves,
            &format!("{} (Ctrl+Q)", tr("menu.object.convert_to_curves")),
            app.can_convert_to_curves(),
            false,
        )
        .clicked()
        {
            app.convert_to_curves();
        }
        ui.label(
            egui::RichText::new(tr("toolbar.convert_to_curves_hint"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
    }
}

/// Interactive Fill property bar: fill type, then the fields of that type
/// (colour; fountain kind, start and end colours, angle, edge pad). With a
/// selection the fields edit it; otherwise they set the default fill.
fn interactive_fill_bar(app: &mut App, ui: &mut Ui, shapes: &[tracedraw_core::document::Shape]) {
    use tracedraw_core::patch::FillField;
    use tracedraw_core::style::FountainKind;
    // The first object's fill; values that differ across the selection
    // show as mixed, and a change goes only into what was changed.
    let base = app.paint_targets().into_iter().next();
    let old = base
        .as_ref()
        .map(|s| s.fill.clone())
        .unwrap_or_else(|| app.default_fill());
    let mix = if shapes.is_empty() {
        tracedraw_core::patch::FillMix::default()
    } else {
        app.selection_fill_mix()
    };
    let mut fill = old.clone();
    let kind = tracedraw_core::patch::fill_kind(&fill);
    let shown_kind = (!mix.kind).then_some(kind);
    let mut touched: Vec<FillField> = Vec::new();
    let mut changed = false;
    for (i, key) in [
        "docker.fill_none",
        "docker.fill_uniform",
        "docker.fill_fountain",
        "docker.fill_pattern",
        "docker.fill_texture",
        "docker.fill_mesh",
    ]
    .iter()
    .enumerate()
    {
        let clicked = ui
            .selectable_label(shown_kind == Some(i), tr(key))
            .clicked();
        if clicked && kind == i && mix.kind {
            // The first object's type, picked for objects of other types.
            touched.push(FillField::All);
            changed = true;
        }
        if clicked && kind != i {
            let base = fill
                .preview_color()
                .unwrap_or(tracedraw_core::Color::cmyk_pct(0.0, 0.0, 0.0, 20.0));
            fill = match i {
                0 => Fill::None,
                1 => Fill::Solid(base),
                2 => Fill::linear(base, tracedraw_core::Color::WHITE, 0.0),
                3 => Fill::Pattern(tracedraw_core::Pattern::TwoColor {
                    tile: tracedraw_core::style::PatternTile::Checker,
                    front: base,
                    back: tracedraw_core::Color::WHITE,
                    size_mm: 10.0,
                }),
                4 => Fill::Texture(tracedraw_core::style::Texture {
                    kind: tracedraw_core::style::TextureKind::Clouds,
                    color_a: base,
                    color_b: tracedraw_core::Color::WHITE,
                    scale: 20.0,
                    seed: 1,
                }),
                _ => Fill::Mesh(tracedraw_core::Mesh::new(
                    shapes
                        .first()
                        .map(|s| s.bounds())
                        .unwrap_or(Rect::new(0.0, 0.0, 100.0, 100.0)),
                    2,
                    2,
                    base,
                )),
            };
            changed = true;
        }
    }
    vsep(ui);
    let color_button = |ui: &mut Ui, c: &mut tracedraw_core::Color, mixed: bool| -> bool {
        let mut rgb = c.to_rgb8();
        let r = ui.color_edit_button_srgb(&mut rgb);
        if mixed {
            crate::ui::field::mixed_swatch(ui, r.rect);
        }
        if r.changed() {
            *c = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
            true
        } else {
            false
        }
    };
    // A setting the user set; when it was mixed it applies to every
    // object even if it equals the value shown.
    let mut set = |field: FillField, changed: &mut bool| {
        *changed = true;
        if mix.has(field) {
            touched.push(field);
        }
    };
    match &mut fill {
        _ if mix.kind => {
            crate::ui::field::mixed_note(ui);
        }
        Fill::Solid(c) => {
            ui.label(
                egui::RichText::new(tr("docker.colour"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
            if color_button(ui, c, mix.has(FillField::All)) {
                set(FillField::All, &mut changed);
            }
        }
        Fill::Fountain(f) => {
            let kind_shown = !mix.has(FillField::FountainKind);
            for k in FountainKind::ALL {
                let key = crate::ui::dockers::fountain_kind_key(k);
                if ui
                    .selectable_label(kind_shown && f.kind == k, tr(key))
                    .clicked()
                    && (f.kind != k || !kind_shown)
                {
                    f.kind = k;
                    set(FillField::FountainKind, &mut changed);
                }
            }
            vsep(ui);
            if let Some(first) = f.stops.first_mut() {
                if color_button(ui, &mut first.color, mix.has(FillField::StartColor)) {
                    set(FillField::StartColor, &mut changed);
                }
            }
            if let Some(last) = f.stops.last_mut() {
                if color_button(ui, &mut last.color, mix.has(FillField::EndColor)) {
                    set(FillField::EndColor, &mut changed);
                }
            }
            ui.label(
                egui::RichText::new(tr("toolbar.angle"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut f.angle)
                        .speed(1.0)
                        .range(-360.0..=360.0)
                        .fixed_decimals(1)
                        .suffix("°")
                        .mixed(mix.has(FillField::Angle)),
                )
                .changed()
            {
                set(FillField::Angle, &mut changed);
            }
            ui.label(
                egui::RichText::new(tr("docker.edge_pad"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
            let mut pad = f.edge_pad * 100.0;
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut pad)
                        .speed(1.0)
                        .range(0.0..=49.0)
                        .suffix(" %")
                        .mixed(mix.has(FillField::EdgePad)),
                )
                .changed()
            {
                f.edge_pad = pad / 100.0;
                set(FillField::EdgePad, &mut changed);
            }
        }
        _ => {}
    }
    if changed {
        if shapes.is_empty() {
            app.apply_fill_or_default(fill);
        } else {
            app.patch_selection_fill(&old, &fill, &touched);
        }
    }
    vsep(ui);
    ui.label(
        egui::RichText::new(tr("toolbar.fill_hint"))
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// The Common Shapes tool's bar: the shape picked from a grid of
/// thumbnails, set by set.
fn common_shapes_bar(app: &mut App, ui: &mut Ui) {
    use crate::common_shapes::{Category, CommonShape};
    ui.label(
        egui::RichText::new(tr("toolbar.shape"))
            .color(Tokens::text_dim())
            .size(11.0),
    );
    let popup = egui::Id::new("common_shapes_popup");
    let resp = ui
        .horizontal(|ui| {
            let thumb = shape_thumbnail(ui, app.common_shape, false, true);
            let name = ui.add(
                egui::Button::new(format!("{} \u{25BE}", app.common_shape.name())).frame(false),
            );
            thumb.union(name)
        })
        .inner;
    egui::Popup::menu(&resp)
        .id(popup)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            let shown = ui.data_mut(|d| {
                *d.get_temp_mut_or_insert_with(popup, || app.common_shape.category())
            });
            ui.horizontal(|ui| {
                for c in Category::ALL {
                    if ui.selectable_label(shown == c, tr(c.key())).clicked() {
                        ui.data_mut(|d| d.insert_temp(popup, c));
                    }
                }
            });
            ui.separator();
            egui::Grid::new("common_shapes_grid")
                .spacing([2.0, 2.0])
                .show(ui, |ui| {
                    for (i, s) in CommonShape::of(shown).enumerate() {
                        let picked =
                            shape_thumbnail(ui, s, app.common_shape == s, false).rich_tip(s.name());
                        if picked.clicked() {
                            app.common_shape = s;
                            egui::Popup::close_id(ui.ctx(), popup);
                        }
                        if i % 9 == 8 {
                            ui.end_row();
                        }
                    }
                });
        });
    ui.label(
        egui::RichText::new(tr("toolbar.drag_to_draw"))
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// A shape's outline drawn small, as a button (`on`: the one picked).
fn shape_thumbnail(
    ui: &mut Ui,
    shape: crate::common_shapes::CommonShape,
    on: bool,
    small: bool,
) -> egui::Response {
    use tracedraw_core::geometry::PathEl;
    let side = if small { 22.0 } else { 34.0 };
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::click());
    let painter = ui.painter();
    if on || resp.hovered() {
        let fill = if on {
            Tokens::tool_active()
        } else {
            Tokens::tool_hover()
        };
        painter.rect_filled(rect, 2.0, fill);
    }
    let inner = rect.shrink(if small { 3.0 } else { 6.0 });
    // Page y runs up: the box's bottom is the thumbnail's bottom.
    let path = shape.path(Rect::new(0.0, 0.0, 1.0, 1.0));
    let to_screen = |x: f64, y: f64| {
        egui::pos2(
            inner.left() + x as f32 * inner.width(),
            inner.bottom() - y as f32 * inner.height(),
        )
    };
    let stroke = egui::Stroke::new(1.0, Tokens::icon());
    let mut run: Vec<egui::Pos2> = Vec::new();
    let flush = |run: &mut Vec<egui::Pos2>, closed: bool| {
        if run.len() > 1 {
            let mut pts = std::mem::take(run);
            if closed {
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
            }
            painter.add(egui::Shape::line(pts, stroke));
        }
        run.clear();
    };
    tracedraw_core::geometry::flatten(&path, 0.005, &mut |el| match el {
        PathEl::MoveTo(p) => {
            flush(&mut run, false);
            run.push(to_screen(p.x, p.y));
        }
        PathEl::LineTo(p) => run.push(to_screen(p.x, p.y)),
        PathEl::ClosePath => flush(&mut run, true),
        _ => {}
    });
    flush(&mut run, false);
    resp
}

/// The Drop Shadow tool's property bar: presets, opacity, feathering and
/// its direction and edges, merge mode, the flat offset or the
/// perspective angle, stretch and fade, colour, Break Apart and Clear.
fn shadow_bar(app: &mut App, ui: &mut Ui, shapes: &[tracedraw_core::Shape]) {
    use tracedraw_core::shadow::{FeatherDirection, FeatherEdges, Perspective, ShadowEdge};
    let dim = |ui: &mut Ui, key: &str| {
        ui.label(
            egui::RichText::new(tr(key))
                .color(Tokens::text_dim())
                .size(11.0),
        );
    };
    let first = shapes.first().and_then(|s| s.shadow);
    let mut sh = first.unwrap_or(app.shadow_default);
    let mut changed = false;
    dim(ui, "toolbar.drop_shadow");
    if ui
        .add_enabled(
            !shapes.is_empty() && first.is_none(),
            egui::Button::new(tr("docker.apply")),
        )
        .clicked()
    {
        app.set_shadow(Some(sh), false);
    }
    let mut preset = None;
    egui::ComboBox::from_id_salt("shadow_presets")
        .selected_text(tr("shadow.presets"))
        .width(110.0)
        .show_ui(ui, |ui| {
            for (key, _) in tracedraw_core::Shadow::presets() {
                if ui
                    .selectable_label(false, tr(&format!("shadow.preset.{key}")))
                    .clicked()
                {
                    preset = Some(key);
                }
            }
        });
    if let Some(key) = preset {
        app.apply_shadow_preset(key);
        return;
    }
    vsep(ui);
    // Flat or standing on an edge.
    let kind_key = |p: Option<ShadowEdge>| match p {
        None => "shadow.flat",
        Some(ShadowEdge::Bottom) => "shadow.edge_bottom",
        Some(ShadowEdge::Top) => "shadow.edge_top",
        Some(ShadowEdge::Left) => "shadow.edge_left",
        Some(ShadowEdge::Right) => "shadow.edge_right",
    };
    let current = sh.perspective.map(|p| p.edge);
    egui::ComboBox::from_id_salt("shadow_kind")
        .selected_text(tr(kind_key(current)))
        .width(110.0)
        .show_ui(ui, |ui| {
            let mut options = vec![None];
            options.extend(ShadowEdge::ALL.iter().map(|e| Some(*e)));
            for o in options {
                if ui.selectable_label(current == o, tr(kind_key(o))).clicked() {
                    sh.perspective = o.map(|e| {
                        let mut per = Perspective::new(e);
                        if let Some(old) = sh.perspective {
                            per.fade = old.fade;
                            per.stretch = old.stretch;
                        }
                        per
                    });
                    changed = true;
                }
            }
        });
    dim(ui, "toolbar.opacity");
    let mut opacity = (sh.opacity * 100.0).round();
    if ui
        .add(
            crate::ui::field::NumField::new(&mut opacity)
                .range(0.0..=100.0)
                .suffix(" %"),
        )
        .changed()
    {
        sh.opacity = opacity / 100.0;
        changed = true;
    }
    dim(ui, "toolbar.feathering");
    let mut blur = sh.blur;
    if ui
        .add(
            crate::ui::field::NumField::new(&mut blur)
                .speed(0.1)
                .range(0.0..=50.0)
                .suffix(" mm"),
        )
        .changed()
    {
        sh.blur = blur;
        changed = true;
    }
    let dir_key = |d: FeatherDirection| match d {
        FeatherDirection::Inside => "shadow.feather_inside",
        FeatherDirection::Middle => "shadow.feather_middle",
        FeatherDirection::Outside => "shadow.feather_outside",
        FeatherDirection::Average => "shadow.feather_average",
    };
    egui::ComboBox::from_id_salt("shadow_feather_dir")
        .selected_text(tr(dir_key(sh.feather_direction)))
        .width(80.0)
        .show_ui(ui, |ui| {
            for d in FeatherDirection::ALL {
                if ui
                    .selectable_label(sh.feather_direction == d, tr(dir_key(d)))
                    .clicked()
                {
                    sh.feather_direction = d;
                    changed = true;
                }
            }
        })
        .response
        .rich_tip(tr("shadow.feather_direction"));
    let edge_key = |e: FeatherEdges| match e {
        FeatherEdges::Linear => "shadow.edges_linear",
        FeatherEdges::Squared => "shadow.edges_squared",
        FeatherEdges::InvertedSquared => "shadow.edges_inverted_squared",
        FeatherEdges::Flat => "shadow.edges_flat",
    };
    egui::ComboBox::from_id_salt("shadow_feather_edges")
        .selected_text(tr(edge_key(sh.feather_edges)))
        .width(96.0)
        .show_ui(ui, |ui| {
            for e in FeatherEdges::ALL {
                if ui
                    .selectable_label(sh.feather_edges == e, tr(edge_key(e)))
                    .clicked()
                {
                    sh.feather_edges = e;
                    changed = true;
                }
            }
        })
        .response
        .rich_tip(tr("shadow.feather_edges"));
    dim(ui, "transparency.merge_mode");
    egui::ComboBox::from_id_salt("shadow_merge")
        .selected_text(sh.merge.name())
        .width(90.0)
        .show_ui(ui, |ui| {
            for m in tracedraw_core::live::MergeMode::ALL {
                if ui.selectable_label(sh.merge == m, m.name()).clicked() {
                    sh.merge = m;
                    changed = true;
                }
            }
        });
    vsep(ui);
    match sh.perspective.as_mut() {
        None => {
            let mut dx = sh.offset.x;
            let mut dy = sh.offset.y;
            dim(ui, "docker.offset");
            changed |= ui
                .add(
                    crate::ui::field::NumField::new(&mut dx)
                        .speed(0.1)
                        .suffix(" mm"),
                )
                .changed();
            changed |= ui
                .add(
                    crate::ui::field::NumField::new(&mut dy)
                        .speed(0.1)
                        .suffix(" mm"),
                )
                .changed();
            sh.offset = tracedraw_core::geometry::Vec2::new(dx, dy);
        }
        Some(per) => {
            dim(ui, "shadow.angle");
            changed |= ui
                .add(
                    crate::ui::field::NumField::new(&mut per.angle)
                        .range(-360.0..=360.0)
                        .suffix("°"),
                )
                .changed();
            dim(ui, "shadow.stretch");
            let mut stretch = (per.stretch * 100.0).round();
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut stretch)
                        .range(0.0..=1000.0)
                        .suffix(" %"),
                )
                .changed()
            {
                per.stretch = stretch / 100.0;
                changed = true;
            }
            dim(ui, "shadow.fade");
            let mut fade = (per.fade * 100.0).round();
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut fade)
                        .range(0.0..=100.0)
                        .suffix(" %"),
                )
                .changed()
            {
                per.fade = fade / 100.0;
                changed = true;
            }
        }
    }
    let [r, g, b] = sh.color.to_rgb8();
    let mut rgb = [r, g, b];
    if ui.color_edit_button_srgb(&mut rgb).changed() {
        sh.color = tracedraw_core::Color::rgb8(rgb[0], rgb[1], rgb[2]);
        changed = true;
    }
    if changed {
        app.shadow_default = sh;
        if first.is_some() {
            app.set_shadow(Some(sh), true);
        }
    }
    vsep(ui);
    if ui
        .add_enabled(
            app.can_break_shadow_apart(),
            egui::Button::new(tr("shadow.break_apart")),
        )
        .rich_tip(tr("shadow.break_apart_tip"))
        .clicked()
    {
        app.break_shadow_apart();
    }
    if ui
        .add_enabled(
            first.is_some(),
            egui::Button::new(tr("toolbar.clear_shadow")),
        )
        .clicked()
    {
        app.set_shadow(None, false);
    }
    if shapes.is_empty() {
        dim(ui, "toolbar.shadow_hint");
    }
}
