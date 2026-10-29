//! The Crop tool's property bar and the crop area on the page. Bar (with
//! an area): its position and size (the X and Y of the object origin's
//! point, as for objects), Crop and Clear Crop Area; then what to do next.
//! On the page: the drawing outside the area dimmed, the area's dashed
//! edge and its eight handles.

use crate::app::App;
use crate::crop_tool::{handles, moved};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::propbar::{distance, label_text, origin_point, row, sep, stacked};
use egui::{Color32, Painter, Stroke, Ui};
use tracedraw_core::geometry::{Point, Rect};

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// `r` sized `w` by `h` about its point at the object origin.
fn sized(r: Rect, origin: (u8, u8), w: f64, h: f64) -> Rect {
    let p = origin_point(r, origin);
    let fx = match origin.0 {
        0 => 0.0,
        1 => 0.5,
        _ => 1.0,
    };
    // Row 0 is the top.
    let fy = match origin.1 {
        0 => 1.0,
        1 => 0.5,
        _ => 0.0,
    };
    let (w, h) = (w.max(0.01), h.max(0.01));
    let x0 = p.x - w * fx;
    let y0 = p.y - h * fy;
    Rect::new(x0, y0, x0 + w, y0 + h)
}

/// The bar.
pub fn crop_bar(app: &mut App, ui: &mut Ui) {
    let Some(r) = app.crop_area else {
        dim(ui, tr("crop.draw_hint"));
        return;
    };
    let o = app.ruler_origin();
    let refp = origin_point(r, app.object_origin);
    let mut next = None;
    stacked(ui, |ui| {
        row(ui, |ui| {
            label_text(ui, "X:");
            if let Some(x) = distance(ui, app, refp.x - o.x, 100.0, None) {
                next = Some(moved(r, Point::new(x + o.x, refp.y) - refp));
            }
        });
        row(ui, |ui| {
            label_text(ui, "Y:");
            if let Some(y) = distance(ui, app, refp.y - o.y, 100.0, None) {
                next = Some(moved(r, Point::new(refp.x, y + o.y) - refp));
            }
        });
    });
    stacked(ui, |ui| {
        row(ui, |ui| {
            label_text(ui, "W:");
            if let Some(w) = distance(ui, app, r.width(), 100.0, None) {
                next = Some(sized(r, app.object_origin, w, r.height()));
            }
        });
        row(ui, |ui| {
            label_text(ui, "H:");
            if let Some(h) = distance(ui, app, r.height(), 100.0, None) {
                next = Some(sized(r, app.object_origin, r.width(), h));
            }
        });
    });
    if let Some(n) = next {
        if n.x0.is_finite() && n.y0.is_finite() && n.x1.is_finite() && n.y1.is_finite() {
            app.crop_area = Some(n);
        }
    }
    sep(ui);
    if ui.button(tr("crop.apply")).clicked() {
        app.apply_crop();
    }
    if ui.button(tr("crop.clear")).clicked() {
        app.crop_area = None;
    }
    sep(ui);
    dim(ui, tr("crop.apply_hint"));
}

/// Where the Crop and Clear buttons sit on the canvas: just below the
/// area's bottom-left corner, kept inside the canvas (#1202).
pub fn buttons_anchor(app: &App, canvas: egui::Rect) -> Option<egui::Pos2> {
    if app.tool != crate::tools::Tool::Crop {
        return None;
    }
    let r = app.crop_area?;
    let bottom_left = app.view.to_screen(Point::new(r.x0, r.y0));
    let p = bottom_left + egui::vec2(0.0, 8.0);
    Some(egui::pos2(
        p.x.clamp(
            canvas.min.x + 4.0,
            (canvas.max.x - 160.0).max(canvas.min.x + 4.0),
        ),
        p.y.clamp(
            canvas.min.y + 4.0,
            (canvas.max.y - 32.0).max(canvas.min.y + 4.0),
        ),
    ))
}

/// Crop and Clear, floating under the area on the canvas, so the area is
/// applied or dropped without reaching the bar (#1202). Their places are
/// kept in `crop_button_rects` for the tests.
pub fn canvas_buttons(app: &mut App, ui: &mut Ui, canvas: egui::Rect) {
    let Some(anchor) = buttons_anchor(app, canvas) else {
        app.crop_button_rects = None;
        return;
    };
    let mut apply = false;
    let mut clear = false;
    let mut rects = None;
    egui::Area::new(egui::Id::new("crop_canvas_buttons"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let a = ui.button(format!("\u{2714} {}", tr("crop.apply")));
                        let c = ui.button(format!("\u{2716} {}", tr("crop.clear_button")));
                        apply = a.clicked();
                        clear = c.clicked();
                        rects = Some((a.rect, c.rect));
                    });
                });
        });
    app.crop_button_rects = rects;
    if apply {
        app.apply_crop();
    } else if clear {
        app.crop_area = None;
        app.crop_drag = None;
    }
}

/// The area: the drawing outside it dimmed, its edge and handles.
pub fn draw_area(app: &App, painter: &Painter, canvas: egui::Rect) {
    if app.tool != crate::tools::Tool::Crop {
        return;
    }
    let Some(r) = app.crop_area else {
        return;
    };
    let a = app.view.to_screen(Point::new(r.x0, r.y1));
    let b = app.view.to_screen(Point::new(r.x1, r.y0));
    let area = egui::Rect::from_two_pos(a, b);
    let shade = Color32::from_black_alpha(90);
    let c = canvas;
    for band in [
        egui::Rect::from_min_max(c.min, egui::pos2(c.max.x, area.min.y.max(c.min.y))),
        egui::Rect::from_min_max(egui::pos2(c.min.x, area.max.y.min(c.max.y)), c.max),
        egui::Rect::from_min_max(
            egui::pos2(c.min.x, area.min.y.max(c.min.y)),
            egui::pos2(area.min.x.max(c.min.x), area.max.y.min(c.max.y)),
        ),
        egui::Rect::from_min_max(
            egui::pos2(area.max.x.min(c.max.x), area.min.y.max(c.min.y)),
            egui::pos2(c.max.x, area.max.y.min(c.max.y)),
        ),
    ] {
        if band.width() > 0.0 && band.height() > 0.0 {
            painter.rect_filled(band, 0.0, shade);
        }
    }
    let corners = [
        area.left_top(),
        area.right_top(),
        area.right_bottom(),
        area.left_bottom(),
        area.left_top(),
    ];
    painter.add(egui::Shape::dashed_line(
        &corners,
        Stroke::new(1.0, Color32::WHITE),
        5.0,
        0.0,
    ));
    painter.add(egui::Shape::dashed_line(
        &corners,
        Stroke::new(1.0, Color32::BLACK),
        5.0,
        5.0,
    ));
    for h in handles(r) {
        let p = app.view.to_screen(h);
        let hr = egui::Rect::from_center_size(p, egui::vec2(7.0, 7.0));
        painter.rect_filled(hr, 0.0, Color32::WHITE);
        painter.rect_stroke(
            hr,
            0.0,
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Middle,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_and_the_area_draw() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.tool = crate::tools::Tool::Crop;
        for area in [None, Some(Rect::new(10.0, 10.0, 50.0, 40.0))] {
            app.crop_area = area;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| crop_bar(&mut app, ui));
                draw_area(&app, ui.painter(), ui.max_rect());
            });
            out.textures_delta.clear();
        }
        // Sized about the origin point: the top left stays.
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(sized(r, (0, 0), 20.0, 5.0), Rect::new(0.0, 5.0, 20.0, 10.0));
        assert_eq!(sized(r, (1, 1), 4.0, 4.0), Rect::new(3.0, 3.0, 7.0, 7.0));
    }
}
