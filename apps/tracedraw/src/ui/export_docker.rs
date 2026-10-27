//! The Export docker (Window > Dockers > Export): Add Pages and Add
//! Selection fill the list of assets; each row has a tick, its name, its
//! file format, the resolution of raster formats and a remove button.
//! Below, the folder the files go to and Export, which writes every
//! ticked asset at once.

use crate::app::App;
use crate::export_assets::AssetSource;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::EXPORT_FORMATS;
use crate::ui::field::NumField;
use egui::Ui;

/// Formats drawn as pixels, which take a resolution.
fn raster(format: usize) -> bool {
    matches!(
        EXPORT_FORMATS.get(format).map(|(_, e)| *e),
        Some("png" | "jpg" | "bmp" | "tif" | "webp" | "gif" | "psd")
    )
}

fn format_combo(ui: &mut Ui, salt: impl std::hash::Hash + std::fmt::Debug, format: &mut usize) {
    egui::ComboBox::from_id_salt(salt)
        .selected_text(EXPORT_FORMATS.get(*format).map(|(n, _)| *n).unwrap_or("?"))
        .width(70.0)
        .show_ui(ui, |ui| {
            for (i, (name, _)) in EXPORT_FORMATS.iter().enumerate() {
                ui.selectable_value(format, i, *name);
            }
        });
}

pub fn export_docker(app: &mut App, ui: &mut Ui) {
    app.prune_export_assets();
    let drawing = app.has_document() && !app.show_welcome;
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(drawing, egui::Button::new(tr("export_docker.add_pages")))
            .on_hover_text(tr("export_docker.add_pages_tip"))
            .clicked()
        {
            app.add_page_assets();
        }
        if ui
            .add_enabled(
                drawing && !app.selection.is_empty(),
                egui::Button::new(tr("export_docker.add_selection")),
            )
            .on_hover_text(tr("export_docker.add_selection_tip"))
            .clicked()
        {
            app.add_selection_asset();
        }
    });
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr("export_docker.new_assets"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        format_combo(ui, "export_assets_default", &mut app.export_assets.format);
        if raster(app.export_assets.format) {
            ui.add(
                NumField::new(&mut app.export_assets.dpi)
                    .range(crate::ui::dialogs::MIN_EXPORT_DPI..=2400.0)
                    .suffix(" dpi")
                    .width(64.0),
            );
        }
    });
    ui.separator();

    if app.export_assets.assets.is_empty() {
        ui.label(
            egui::RichText::new(tr("export_docker.empty"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
    }
    let page_names: Vec<(tracedraw_core::PageId, usize)> = app
        .doc()
        .pages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id, i + 1))
        .collect();
    let mut remove = None;
    let mut show = None;
    egui::ScrollArea::vertical()
        .max_height((ui.available_height() - 70.0).max(80.0))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for (i, asset) in app.export_assets.assets.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut asset.checked, "");
                        ui.add(egui::TextEdit::singleline(&mut asset.name).desired_width(110.0));
                        if ui
                            .small_button("\u{00D7}")
                            .on_hover_text(tr("export_docker.remove"))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(24.0);
                        format_combo(ui, ("asset_format", i), &mut asset.format);
                        if raster(asset.format) {
                            ui.add(
                                NumField::new(&mut asset.dpi)
                                    .range(crate::ui::dialogs::MIN_EXPORT_DPI..=2400.0)
                                    .suffix(" dpi")
                                    .width(64.0),
                            );
                        }
                        let what = match &asset.source {
                            AssetSource::Page(p) => trf(
                                "export_docker.page_n",
                                &[(
                                    "n",
                                    &page_names
                                        .iter()
                                        .find(|(id, _)| id == p)
                                        .map(|(_, n)| n.to_string())
                                        .unwrap_or_default(),
                                )],
                            ),
                            AssetSource::Objects { shapes, .. } => trf(
                                "export_docker.objects_n",
                                &[("n", &shapes.len().to_string())],
                            ),
                        };
                        if ui
                            .link(egui::RichText::new(what).size(11.0))
                            .on_hover_text(tr("export_docker.show_tip"))
                            .clicked()
                        {
                            show = Some(asset.source.clone());
                        }
                    });
                    ui.add_space(4.0);
                });
            }
        });
    if let Some(i) = remove {
        if i < app.export_assets.assets.len() {
            app.export_assets.assets.remove(i);
        }
    }
    match show {
        Some(AssetSource::Page(p)) => {
            if let Some(idx) = app.doc().pages.iter().position(|pg| pg.id == p) {
                app.goto_page(idx);
            }
        }
        Some(AssetSource::Objects { page, shapes }) => {
            if let Some(idx) = app.doc().pages.iter().position(|pg| pg.id == page) {
                app.goto_page(idx);
            }
            app.select(shapes);
        }
        None => {}
    }

    ui.separator();
    if !crate::files::WEB {
        ui.horizontal(|ui| {
            let folder = app
                .export_assets
                .folder
                .as_ref()
                .map(|f| f.display().to_string())
                .unwrap_or_else(|| tr("export_docker.no_folder"));
            ui.label(egui::RichText::new(folder).size(11.0))
                .on_hover_text(tr("export_docker.folder"));
            if ui.button(tr("export_docker.browse")).clicked() {
                if let Some(f) = crate::files::pick_folder() {
                    app.export_assets.folder = Some(f);
                }
            }
        });
    }
    let count = app
        .export_assets
        .assets
        .iter()
        .filter(|a| a.checked)
        .count();
    if ui
        .add_enabled(
            count > 0,
            egui::Button::new(trf("export_docker.export", &[("n", &count.to_string())])),
        )
        .clicked()
    {
        app.export_assets();
    }
}
