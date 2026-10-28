//! The Picture Mosaic docker (Effects > Picture Mosaic), in sections:
//! Reference (what is selected, Keep original source), Library (the
//! pictures and their folder, Browse), Grid (columns, the rows they give,
//! blending), Duplicates with their spacing, Output (composition, edges,
//! priority, resolution, the artwork's and a tile's pixel size), and a
//! full-width Apply.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::picture_mosaic::{grid, tile_pixels, Composition, Edges, Priority};
use crate::theme::Tokens;
use crate::ui::docker_form::{indented, row, section};
use crate::ui::tips::RichTip;
use egui::{RichText, Ui};

/// The library section: the picture count, the folder, Browse and the
/// progress while the folder is read.
fn library(app: &mut App, ui: &mut Ui) {
    section(ui, Some(&tr("docker.picture_mosaic_library")));
    if crate::files::WEB {
        ui.label(
            RichText::new(tr("docker.picture_mosaic_web"))
                .color(Tokens::text_dim())
                .small(),
        );
        return;
    }
    let none = tr("docker.form_none");
    let n = app.picture_mosaic.images.len();
    row(ui, &tr("docker.picture_mosaic_images_label"), |ui| {
        ui.label(if n == 0 {
            none.clone()
        } else {
            trf("docker.picture_mosaic_images", &[("n", &n.to_string())])
        });
    });
    let name = app
        .picture_mosaic
        .library
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| none.clone());
    let full = app
        .picture_mosaic
        .library
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    row(ui, &tr("docker.picture_mosaic_path"), |ui| {
        let r = ui.add(egui::Label::new(name).truncate());
        if !full.is_empty() {
            r.rich_tip(full);
        }
    });
    let busy = app.picture_mosaic_job.is_some();
    indented(ui, |ui| {
        if ui
            .add_enabled(
                !busy,
                egui::Button::new(tr("docker.picture_mosaic_browse"))
                    .min_size(egui::vec2(100.0, 24.0)),
            )
            .rich_tip(tr("docker.picture_mosaic_browse_tip"))
            .clicked()
        {
            if let Some(dir) = crate::files::pick_folder() {
                app.picture_mosaic_load_library(dir);
            }
        }
    });
    if let Some(job) = &app.picture_mosaic_job {
        let done = job.done();
        let total = job.total.max(1);
        ui.add(
            egui::ProgressBar::new(done as f32 / total as f32)
                .desired_width(ui.available_width().min(240.0))
                .text(trf(
                    "docker.picture_mosaic_indexing",
                    &[("n", &done.to_string()), ("total", &job.total.to_string())],
                )),
        );
    }
}

/// A read-only or editable pixel count.
fn pixels(ui: &mut Ui, label: &str, v: &mut u32, editable: bool, max: u32) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add_enabled(
            editable,
            crate::ui::field::NumField::new(v)
                .range(1..=max)
                .suffix(" px"),
        )
        .rich_tip(crate::ui::tips::pair(
            "docker.picture_mosaic_pixels",
            "docker.picture_mosaic_pixels_desc",
        ))
        .rich_disabled_tip(crate::ui::tips::pair(
            "docker.picture_mosaic_pixels",
            "docker.picture_mosaic_pixels_desc",
        ))
        .changed()
    })
    .inner
}

pub fn picture_mosaic(app: &mut App, ui: &mut Ui) {
    ui.spacing_mut().slider_width = 110.0;
    let bounds = app.selection_bounds();
    let selected = app.selection.len();
    // Reference.
    ui.label(RichText::new(tr("docker.form_reference")).strong());
    row(ui, &tr("docker.form_selection"), |ui| {
        ui.label(match selected {
            0 => tr("docker.form_nothing_selected"),
            1 => tr("docker.form_one_selected"),
            n => trf("docker.form_n_selected", &[("n", &n.to_string())]),
        });
    });
    indented(ui, |ui| {
        ui.checkbox(
            &mut app.picture_mosaic.keep_original,
            tr("docker.picture_mosaic_keep_original"),
        )
        .rich_tip(crate::ui::tips::keyed(
            "docker.picture_mosaic_keep_original",
        ));
    });
    library(app, ui);
    let dpi = app.document_dpi();
    let s = &mut app.picture_mosaic;
    let (cols, rows, _) = match bounds {
        Some(b) => grid(s.columns, b.width(), b.height(), s.edges),
        None => (s.columns, 0, 1.0),
    };
    section(ui, Some(&tr("docker.form_grid")));
    row(ui, &tr("docker.picture_mosaic_columns"), |ui| {
        ui.add(crate::ui::field::NumField::new(&mut s.columns).range(2..=300))
            .rich_tip(crate::ui::tips::keyed("docker.picture_mosaic_columns"));
    });
    row(ui, &tr("docker.picture_mosaic_rows"), |ui| {
        let mut r = rows;
        ui.add_enabled(false, crate::ui::field::NumField::new(&mut r))
            .rich_disabled_tip(crate::ui::tips::keyed("docker.picture_mosaic_rows"));
    });
    row(ui, &tr("docker.picture_mosaic_blending"), |ui| {
        ui.add(crate::ui::Rail(
            egui::Slider::new(&mut s.blending, 0.0..=100.0)
                .suffix(" %")
                .max_decimals(0),
        ))
        .rich_tip(crate::ui::tips::keyed("docker.picture_mosaic_blending"));
    });
    section(ui, None);
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.duplicates, tr("docker.picture_mosaic_duplicates"))
            .rich_tip(crate::ui::tips::keyed("docker.picture_mosaic_duplicates"));
        ui.add_enabled(
            s.duplicates,
            crate::ui::field::NumField::new(&mut s.spacing).range(0..=20),
        )
        .rich_tip(tr("docker.picture_mosaic_spacing_tip"));
    });
    section(ui, Some(&tr("docker.form_output")));
    let combo = |ui: &mut Ui, id: &str, text: String, add: &mut dyn FnMut(&mut Ui)| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(text)
            .width(ui.available_width().clamp(120.0, 180.0))
            .show_ui(ui, |ui| add(ui))
            .response
            .rich_tip(crate::ui::tips::keyed(&format!("docker.{id}")));
    };
    row(ui, &tr("docker.picture_mosaic_composition"), |ui| {
        combo(
            ui,
            "picture_mosaic_composition",
            tr(s.composition.key()),
            &mut |ui| {
                for c in Composition::ALL {
                    ui.selectable_value(&mut s.composition, c, tr(c.key()));
                }
            },
        );
    });
    row(ui, &tr("docker.picture_mosaic_edges"), |ui| {
        combo(ui, "picture_mosaic_edges", tr(s.edges.key()), &mut |ui| {
            for e in Edges::ALL {
                ui.selectable_value(&mut s.edges, e, tr(e.key()));
            }
        });
    });
    row(ui, &tr("docker.picture_mosaic_priority"), |ui| {
        combo(
            ui,
            "picture_mosaic_priority",
            tr(s.priority.key()),
            &mut |ui| {
                for p in Priority::ALL {
                    ui.selectable_value(&mut s.priority, p, tr(p.key()));
                }
            },
        );
    });
    row(ui, &tr("docker.form_dpi"), |ui| {
        let custom = s.priority == Priority::CustomDpi;
        let mut shown = if custom { s.dpi } else { dpi };
        if ui
            .add_enabled(
                custom,
                crate::ui::field::NumField::new(&mut shown).range(10.0..=2400.0),
            )
            .rich_tip(crate::ui::tips::pair(
                "docker.form_dpi",
                "docker.picture_mosaic_dpi_desc",
            ))
            .rich_disabled_tip(crate::ui::tips::pair(
                "docker.form_dpi",
                "docker.picture_mosaic_dpi_desc",
            ))
            .changed()
        {
            s.dpi = shown;
        }
    });
    // The artwork's and one tile's pixel size: the output size is set in
    // Custom output dimensions, the tile size in Custom tile dimensions;
    // otherwise they follow from the resolution.
    let (tw, th) = match bounds {
        Some(b) if rows > 0 => tile_pixels(s, b.width(), b.height(), cols, rows, dpi),
        _ => (0, 0),
    };
    let max = crate::picture_mosaic::MAX_SIDE;
    ui.columns(2, |c| {
        c[0].label(tr("docker.form_artwork"));
        let (mut aw, mut ah) = (tw * cols, th * rows);
        let out = s.priority == Priority::OutputSize;
        if pixels(&mut c[0], &tr("docker.form_w"), &mut aw, out, max) {
            s.output_px = aw.clamp(16, max);
        }
        pixels(&mut c[0], &tr("docker.form_h"), &mut ah, false, max);
        c[1].label(tr("docker.form_tile"));
        let (mut w, mut h) = (tw, th);
        let tile = s.priority == Priority::TileSize;
        if pixels(&mut c[1], &tr("docker.form_w"), &mut w, tile, 2000) {
            s.tile_px = w.clamp(4, 2000);
        }
        pixels(&mut c[1], &tr("docker.form_h"), &mut h, false, 2000);
    });
    let ready = bounds.is_some()
        && !app.picture_mosaic.images.is_empty()
        && app.picture_mosaic_job.is_none();
    let tip = crate::ui::tips::pair("docker.apply", "docker.picture_mosaic_apply_desc");
    if crate::ui::docker_form::wide_button(ui, &tr("docker.apply"), ready, &tip) {
        let s = app.picture_mosaic.clone();
        app.apply_picture_mosaic(&s);
    }
}
