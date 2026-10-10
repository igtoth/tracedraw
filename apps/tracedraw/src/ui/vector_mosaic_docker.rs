//! The VectorMosaic docker (Effects > VectorMosaic): density, scale and
//! screen angle, Keep original, Limit colors, the tracking options
//! (method, merge adjacent, weld adjacent overlap), the tile shape with
//! its preview and Select for a custom curve, and Apply.

use crate::app::App;
use crate::i18n::tr;
use crate::vector_mosaic::{unit_tile, TileShape, TrackMethod};
use crate::theme::Tokens;
use egui::{Color32, Sense, Stroke, Ui, Vec2};
use tracedraw_core::geometry::{Affine, Shape as _};

fn heading(ui: &mut Ui, key: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr(key)).strong());
}

/// The tile's preview: the shape at the screen angle in a small box.
fn preview(ui: &mut Ui, app: &App) {
    let s = &app.vector_mosaic;
    let (r, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(220.0), 80.0),
        Sense::hover(),
    );
    let painter = ui.painter_at(r);
    painter.rect_filled(r, 0.0, Color32::WHITE);
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::BORDER),
        egui::StrokeKind::Inside,
    );
    let side = r.height() * 0.6;
    let c = r.center();
    let fill = Color32::from_gray(0x40);
    match (s.shape, &s.custom) {
        (TileShape::Circle, _) => {
            painter.circle_filled(c, side / 2.0, fill);
        }
        (TileShape::Square, _) => {
            let a = -(s.angle as f32).to_radians();
            let pts: Vec<egui::Pos2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|(x, y)| {
                    let (x, y) = (x * side / 2.0, y * side / 2.0);
                    c + Vec2::new(x * a.cos() - y * a.sin(), x * a.sin() + y * a.cos())
                })
                .collect();
            painter.add(egui::Shape::convex_polygon(pts, fill, Stroke::NONE));
        }
        (TileShape::Custom, Some(unit)) => {
            // The unit tile (y up) drawn y down, turned by the angle.
            let m = Affine::translate((c.x as f64, c.y as f64))
                * Affine::scale_non_uniform(side as f64, -(side as f64))
                * Affine::rotate(s.angle.to_radians());
            let mut pts = Vec::new();
            tracedraw_core::geometry::flatten(&(m * unit.clone()), 0.2, &mut |el| match el {
                tracedraw_core::geometry::PathEl::MoveTo(p)
                | tracedraw_core::geometry::PathEl::LineTo(p) => {
                    pts.push(egui::pos2(p.x as f32, p.y as f32))
                }
                _ => {}
            });
            if pts.len() > 2 {
                painter.add(egui::Shape::closed_line(pts, Stroke::new(1.5, fill)));
            }
        }
        (TileShape::Custom, None) => {
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                tr("docker.vector_mosaic_no_custom"),
                egui::FontId::proportional(11.0),
                Tokens::TEXT_DIM,
            );
        }
    }
}

pub fn vector_mosaic(app: &mut App, ui: &mut Ui) {
    ui.spacing_mut().slider_width = 130.0;
    egui::Grid::new("vector_mosaic_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            let s = &mut app.vector_mosaic;
            ui.label(tr("docker.vector_mosaic_density"));
            ui.add(egui::Slider::new(&mut s.density, 1.0..=100.0).max_decimals(1));
            ui.end_row();
            ui.label(tr("docker.vector_mosaic_scale"));
            ui.add(
                egui::Slider::new(&mut s.scale, 0.1..=5.0)
                    .step_by(0.05)
                    .max_decimals(2),
            );
            ui.end_row();
            ui.label(tr("docker.vector_mosaic_angle"));
            ui.add(
                egui::Slider::new(&mut s.angle, -90.0..=90.0)
                    .suffix("\u{b0}")
                    .max_decimals(1),
            );
            ui.end_row();
        });
    let s = &mut app.vector_mosaic;
    ui.checkbox(
        &mut s.keep_original,
        tr("docker.vector_mosaic_keep_original"),
    );
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.limit_colors, tr("docker.vector_mosaic_limit_colors"));
        ui.add_enabled(
            s.limit_colors,
            egui::DragValue::new(&mut s.colors).range(2..=256),
        );
    });
    heading(ui, "docker.vector_mosaic_tracking");
    egui::Grid::new("vector_mosaic_tracking")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("docker.vector_mosaic_method"));
            egui::ComboBox::from_id_salt("vector_mosaic_method")
                .selected_text(tr(s.method.key()))
                .width(170.0)
                .show_ui(ui, |ui| {
                    for m in TrackMethod::ALL {
                        ui.selectable_value(&mut s.method, m, tr(m.key()));
                    }
                });
            ui.end_row();
            ui.label(tr("docker.vector_mosaic_merge"));
            ui.add(egui::DragValue::new(&mut s.merge).range(1..=36));
            ui.end_row();
        });
    ui.checkbox(&mut s.weld, tr("docker.vector_mosaic_weld"));
    heading(ui, "docker.vector_mosaic_shape");
    egui::ComboBox::from_id_salt("vector_mosaic_shape")
        .selected_text(tr(s.shape.key()))
        .width(170.0)
        .show_ui(ui, |ui| {
            for t in TileShape::ALL {
                ui.selectable_value(&mut s.shape, t, tr(t.key()));
            }
        });
    preview(ui, app);
    // Select: the first selected closed curve becomes the custom tile.
    let custom = app.vector_mosaic.shape == TileShape::Custom;
    if ui
        .add_enabled(
            custom && !app.selection.is_empty(),
            egui::Button::new(tr("docker.vector_mosaic_select")),
        )
        .on_hover_text(tr("docker.vector_mosaic_select_tip"))
        .clicked()
    {
        let tile = app
            .selected_shapes()
            .first()
            .map(|sh| sh.page_path())
            .filter(|p| p.bounding_box().area() > 0.0)
            .and_then(|p| unit_tile(&p));
        match tile {
            Some(t) => app.vector_mosaic.custom = Some(t),
            None => app.status = tr("status.vector_mosaic_no_curve"),
        }
    }
    ui.add_space(8.0);
    let ready = !app.selection.is_empty() && (!custom || app.vector_mosaic.custom.is_some());
    if ui
        .add_enabled(
            ready,
            egui::Button::new(tr("docker.apply")).min_size(Vec2::new(90.0, 24.0)),
        )
        .clicked()
    {
        let s = app.vector_mosaic.clone();
        app.apply_vector_mosaic(&s);
    }
}
