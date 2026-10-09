//! The colour palette docked at the bottom: a row
//! of swatches with scroll arrows, and the document palette row under it.
//! Left click sets the fill, right click sets the outline, the X removes it.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::{Rect, Sense, Stroke, Ui, Vec2};

pub fn palette_row(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(1.0, 0.0);
        let sw = Vec2::new(Tokens::SWATCH, Tokens::SWATCH);
        scroll_arrow(ui, "◂");
        no_color_swatch(app, ui, sw);
        let colors = app.palette.clone();
        egui::ScrollArea::horizontal()
            .id_salt("palette_scroll")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(1.0, 0.0);
                    for c in colors {
                        let (r, resp) = ui.allocate_exact_size(sw, Sense::click());
                        let c32 = crate::canvas::to_color32(c);
                        ui.painter().rect_filled(r, 0.0, c32);
                        if resp.hovered() {
                            ui.painter().rect_stroke(
                                r.expand(1.0),
                                0.0,
                                Stroke::new(1.5, Tokens::ACCENT),
                                egui::epaint::StrokeKind::Outside,
                            );
                        }
                        let resp = resp.on_hover_text(crate::app::color_description(c));
                        if resp.clicked() {
                            app.apply_fill(tracedraw_core::Fill::Solid(c));
                        }
                        if resp.secondary_clicked() {
                            app.apply_outline_color(Some(c));
                        }
                    }
                });
            });
        scroll_arrow(ui, "▸");
        scroll_arrow(ui, "»");
    });
}

pub fn document_palette_row(app: &mut App, ui: &mut Ui) {
    let _ = app;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(1.0, 0.0);
        scroll_arrow(ui, "◂");
        let sw = Vec2::new(Tokens::SWATCH, Tokens::SWATCH);
        let (r, _) = ui.allocate_exact_size(sw, Sense::hover());
        ui.painter().rect_stroke(
            r,
            0.0,
            Stroke::new(1.0, Tokens::TEXT_DIM),
            egui::epaint::StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [r.left_bottom(), r.right_top()],
            Stroke::new(1.0, Tokens::TEXT_DIM),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(tr("palette.document_hint"))
                .italics()
                .color(Tokens::TEXT_DIM)
                .size(11.0),
        );
    });
}

fn no_color_swatch(app: &mut App, ui: &mut Ui, sw: Vec2) {
    let (r, resp) = ui.allocate_exact_size(sw, Sense::click());
    ui.painter().rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::TEXT_DIM),
        egui::epaint::StrokeKind::Inside,
    );
    ui.painter().line_segment(
        [r.left_bottom(), r.right_top()],
        Stroke::new(1.0, Tokens::TEXT_DIM),
    );
    let resp = resp.on_hover_text(tr("palette.no_color_tip"));
    if resp.clicked() {
        app.apply_fill(tracedraw_core::Fill::None);
    }
    if resp.secondary_clicked() {
        app.apply_outline_color(None);
    }
}

fn scroll_arrow(ui: &mut Ui, glyph: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(12.0, Tokens::SWATCH), Sense::click());
    let c = r.center();
    let tri = |dx: f32| {
        vec![
            c + Vec2::new(dx * 2.5, 0.0),
            c + Vec2::new(-dx * 2.0, -4.0),
            c + Vec2::new(-dx * 2.0, 4.0),
        ]
    };
    match glyph {
        "◂" => ui.painter().add(egui::epaint::PathShape::convex_polygon(
            tri(-1.0),
            Tokens::TEXT_DIM,
            Stroke::NONE,
        )),
        "▸" => ui.painter().add(egui::epaint::PathShape::convex_polygon(
            tri(1.0),
            Tokens::TEXT_DIM,
            Stroke::NONE,
        )),
        _ => {
            let a = vec![
                c + Vec2::new(-4.0, -4.0),
                c + Vec2::new(0.0, 0.0),
                c + Vec2::new(-4.0, 4.0),
            ];
            let b = vec![
                c + Vec2::new(0.0, -4.0),
                c + Vec2::new(4.0, 0.0),
                c + Vec2::new(0.0, 4.0),
            ];
            ui.painter().add(egui::epaint::PathShape::line(
                a,
                Stroke::new(1.2, Tokens::TEXT_DIM),
            ));
            ui.painter().add(egui::epaint::PathShape::line(
                b,
                Stroke::new(1.2, Tokens::TEXT_DIM),
            ))
        }
    };
    let _ = Rect::ZERO;
}
