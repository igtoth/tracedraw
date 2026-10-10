//! The PictureMosaic docker (Effects > PictureMosaic): the image library
//! folder and its picture count, Keep original, columns with the rows they
//! give, blending, duplicates and their spacing, the advanced options
//! (composition, edges, output priority with its value), the mosaic's
//! pixel size, and Apply.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::picture_mosaic::{grid, tile_pixels, Composition, Edges, Priority};
use crate::theme::Tokens;
use egui::{RichText, Ui, Vec2};

fn heading(ui: &mut Ui, key: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(tr(key)).strong());
}

/// The library part: the folder, Browse, and the count or the progress.
fn library(app: &mut App, ui: &mut Ui) {
    heading(ui, "docker.picture_mosaic_library");
    if crate::files::WEB {
        ui.label(
            RichText::new(tr("docker.picture_mosaic_web"))
                .color(Tokens::TEXT_DIM)
                .small(),
        );
        return;
    }
    ui.horizontal(|ui| {
        let busy = app.picture_mosaic_job.is_some();
        if ui
            .add_enabled(!busy, egui::Button::new(tr("docker.picture_mosaic_browse")))
            .on_hover_text(tr("docker.picture_mosaic_browse_tip"))
            .clicked()
        {
            if let Some(dir) = crate::files::pick_folder() {
                app.picture_mosaic_load_library(dir);
            }
        }
        let name = app
            .picture_mosaic
            .library
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| tr("docker.picture_mosaic_no_folder"));
        let full = app
            .picture_mosaic
            .library
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let r = ui.add(egui::Label::new(name).truncate());
        if !full.is_empty() {
            r.on_hover_text(full);
        }
    });
    match &app.picture_mosaic_job {
        Some(job) => {
            let done = job.done();
            let total = job.total.max(1);
            ui.add(
                egui::ProgressBar::new(done as f32 / total as f32)
                    .desired_width(ui.available_width().min(220.0))
                    .text(trf(
                        "docker.picture_mosaic_indexing",
                        &[("n", &done.to_string()), ("total", &job.total.to_string())],
                    )),
            );
        }
        None => {
            let n = app.picture_mosaic.images.len();
            ui.label(
                RichText::new(trf("docker.picture_mosaic_images", &[("n", &n.to_string())]))
                    .color(Tokens::TEXT_DIM),
            );
        }
    }
}

pub fn picture_mosaic(app: &mut App, ui: &mut Ui) {
    ui.spacing_mut().slider_width = 130.0;
    library(app, ui);
    ui.add_space(4.0);
    let bounds = app.selection_bounds();
    let dpi = app.document_dpi();
    let s = &mut app.picture_mosaic;
    ui.checkbox(&mut s.keep_original, tr("docker.picture_mosaic_keep_original"));
    let (cols, rows, _) = match bounds {
        Some(b) => grid(s.columns, b.width(), b.height(), s.edges),
        None => (s.columns, 0, 1.0),
    };
    egui::Grid::new("picture_mosaic_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("docker.picture_mosaic_columns"));
            ui.add(egui::DragValue::new(&mut s.columns).range(2..=300));
            ui.end_row();
            ui.label(tr("docker.picture_mosaic_rows"));
            ui.label(if rows > 0 {
                rows.to_string()
            } else {
                String::from("-")
            });
            ui.end_row();
            ui.label(tr("docker.picture_mosaic_blending"));
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut s.blending, 0.0..=100.0)
                    .suffix(" %")
                    .max_decimals(0),
            ));
            ui.end_row();
        });
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.duplicates, tr("docker.picture_mosaic_duplicates"));
    });
    ui.horizontal(|ui| {
        ui.add_enabled_ui(s.duplicates, |ui| {
            ui.label(tr("docker.picture_mosaic_spacing"));
            ui.add(egui::DragValue::new(&mut s.spacing).range(0..=20))
                .on_hover_text(tr("docker.picture_mosaic_spacing_tip"));
        });
    });
    egui::CollapsingHeader::new(tr("docker.picture_mosaic_advanced"))
        .id_salt("picture_mosaic_advanced")
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new("picture_mosaic_advanced_grid")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label(tr("docker.picture_mosaic_composition"));
                    egui::ComboBox::from_id_salt("picture_mosaic_composition")
                        .selected_text(tr(s.composition.key()))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for c in Composition::ALL {
                                ui.selectable_value(&mut s.composition, c, tr(c.key()));
                            }
                        });
                    ui.end_row();
                    ui.label(tr("docker.picture_mosaic_edges"));
                    egui::ComboBox::from_id_salt("picture_mosaic_edges")
                        .selected_text(tr(s.edges.key()))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for e in Edges::ALL {
                                ui.selectable_value(&mut s.edges, e, tr(e.key()));
                            }
                        });
                    ui.end_row();
                    ui.label(tr("docker.picture_mosaic_priority"));
                    egui::ComboBox::from_id_salt("picture_mosaic_priority")
                        .selected_text(tr(s.priority.key()))
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for p in Priority::ALL {
                                ui.selectable_value(&mut s.priority, p, tr(p.key()));
                            }
                        });
                    ui.end_row();
                    match s.priority {
                        Priority::DocumentDpi => {
                            ui.label(tr("docker.picture_mosaic_resolution"));
                            ui.label(format!("{dpi:.0} dpi"));
                        }
                        Priority::CustomDpi => {
                            ui.label(tr("docker.picture_mosaic_resolution"));
                            ui.add(
                                egui::DragValue::new(&mut s.dpi)
                                    .range(10.0..=2400.0)
                                    .suffix(" dpi"),
                            );
                        }
                        Priority::TileSize => {
                            ui.label(tr("docker.picture_mosaic_tile_width"));
                            ui.add(
                                egui::DragValue::new(&mut s.tile_px)
                                    .range(4..=2000)
                                    .suffix(" px"),
                            );
                        }
                        Priority::OutputSize => {
                            ui.label(tr("docker.picture_mosaic_output_width"));
                            ui.add(
                                egui::DragValue::new(&mut s.output_px)
                                    .range(16..=crate::picture_mosaic::MAX_SIDE)
                                    .suffix(" px"),
                            );
                        }
                    }
                    ui.end_row();
                });
        });
    // The mosaic's size for the current selection.
    if let Some(b) = bounds {
        if rows > 0 {
            let (tw, th) = tile_pixels(s, b.width(), b.height(), cols, rows, dpi);
            ui.label(
                RichText::new(trf(
                    "docker.picture_mosaic_output",
                    &[
                        ("w", &(tw * cols).to_string()),
                        ("h", &(th * rows).to_string()),
                    ],
                ))
                .color(Tokens::TEXT_DIM),
            );
        }
    }
    ui.add_space(8.0);
    let ready = bounds.is_some() && !app.picture_mosaic.images.is_empty() && app.picture_mosaic_job.is_none();
    if ui
        .add_enabled(
            ready,
            egui::Button::new(tr("docker.apply")).min_size(Vec2::new(90.0, 24.0)),
        )
        .clicked()
    {
        let s = app.picture_mosaic.clone();
        app.apply_picture_mosaic(&s);
    }
}
