//! The property bars of the Mesh Fill and Smart Fill tools, and the
//! colour button they use: the colour as a swatch with an arrow, whose
//! list holds the colour palette's swatches and a picker for any other
//! colour.
//!
//! Mesh Fill: Rows, Columns (new meshes take them, the selected mesh
//! changes to them), Smooth colour, the selected node's transparency,
//! Clear Mesh. Smart Fill: the fill (the default fill, a colour of its
//! own or none) and the outline (the default outline, one of its own
//! width and colour, or none) of the objects it makes.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::tips::RichTip;
use egui::{epaint, vec2, Color32, Rect, Sense, Stroke, Ui};
use tracedraw_core::Color;

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

fn rgb32(c: Color) -> Color32 {
    let [r, g, b] = c.to_rgb8();
    Color32::from_rgb(r, g, b)
}

/// A colour button; true when the colour changed.
pub fn color_pick(
    ui: &mut Ui,
    palette: &[(String, Color)],
    id_salt: &str,
    color: &mut Color,
    tip: &str,
    enabled: bool,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        vec2(36.0, 22.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let painter = ui.painter();
    if enabled && resp.hovered() {
        painter.rect_filled(rect, 2.0, crate::theme::Tokens::hovered_row());
    }
    let swatch = Rect::from_min_size(rect.min + vec2(3.0, 4.0), vec2(20.0, 14.0));
    painter.rect_filled(swatch, 0.0, rgb32(*color));
    painter.rect_stroke(
        swatch,
        0.0,
        Stroke::new(
            1.0,
            if enabled {
                Tokens::icon()
            } else {
                Tokens::border()
            },
        ),
        egui::StrokeKind::Inside,
    );
    let c = rect.right_center() - vec2(6.5, 0.0);
    painter.add(epaint::PathShape::convex_polygon(
        vec![
            c + vec2(-3.0, -1.5),
            c + vec2(3.0, -1.5),
            c + vec2(0.0, 2.0),
        ],
        if enabled {
            Tokens::icon()
        } else {
            Tokens::border()
        },
        Stroke::NONE,
    ));
    let resp = resp.rich_tip(tip);
    if !enabled {
        return false;
    }
    let mut changed = false;
    egui::Popup::menu(&resp)
        .id(egui::Id::new(("color_pick", id_salt)))
        .show(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            for row in palette.chunks(10) {
                ui.horizontal(|ui| {
                    for (name, c) in row {
                        let (r, s) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                        ui.painter().rect_filled(r, 0.0, rgb32(*c));
                        let edge = if *c == *color {
                            Stroke::new(2.0, Tokens::icon())
                        } else {
                            Stroke::new(1.0, Color32::from_gray(0xC0))
                        };
                        ui.painter()
                            .rect_stroke(r, 0.0, edge, egui::StrokeKind::Inside);
                        let s = s.rich_tip(crate::ui::palette::swatch_tip(name, *c));
                        if s.clicked() {
                            *color = *c;
                            changed = true;
                            ui.close();
                        }
                    }
                });
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(tr("toolbar.other_colour"));
                if crate::ui::color_picker::color_button(ui, color) {
                    changed = true;
                }
            });
        });
    changed
}

/// The Mesh Fill tool's bar.
pub fn mesh_bar(app: &mut App, ui: &mut Ui) {
    let mesh = app.selected_mesh();
    // The grid shown: the selected mesh's, else the one new meshes take.
    let (mut rows, mut cols) = mesh
        .as_ref()
        .map(|(_, m)| (m.rows, m.cols))
        .unwrap_or((app.mesh_rows, app.mesh_cols));
    dim(ui, tr("toolbar.rows"));
    let r1 = ui.add(NumField::new(&mut rows).range(1..=50));
    dim(ui, tr("toolbar.columns"));
    let r2 = ui.add(NumField::new(&mut cols).range(1..=50));
    if r1.changed() || r2.changed() {
        app.set_mesh_grid(rows, cols);
    }
    crate::ui::propbar::sep(ui);
    let mut smooth = mesh.as_ref().is_some_and(|(_, m)| m.smooth);
    if ui
        .add_enabled(
            mesh.is_some(),
            egui::Checkbox::new(&mut smooth, tr("docker.smooth_colour")),
        )
        .changed()
    {
        app.set_mesh_smooth(smooth);
    }
    crate::ui::propbar::sep(ui);
    let node = app.selected_mesh_node();
    let alpha = node
        .and_then(|(_, i)| mesh.as_ref().and_then(|(_, m)| m.nodes.get(i)))
        .map(|n| n.alpha)
        .unwrap_or(1.0);
    let mut transparency = ((1.0 - alpha) * 100.0).round() as i32;
    dim(ui, tr("docker.transparency"));
    let r = ui
        .add_enabled(
            node.is_some(),
            egui::Slider::new(&mut transparency, 0..=100).suffix(" %"),
        )
        .rich_tip(tr("toolbar.mesh_node_transparency"));
    if r.changed() {
        app.set_mesh_node_alpha(1.0 - transparency as f32 / 100.0);
    }
    crate::ui::propbar::sep(ui);
    if ui
        .add_enabled(mesh.is_some(), egui::Button::new(tr("toolbar.clear_mesh")))
        .clicked()
    {
        app.clear_mesh();
    }
    crate::ui::propbar::sep(ui);
    dim(
        ui,
        if mesh.is_some() {
            tr("docker.mesh_hint")
        } else {
            tr("status_hint.mesh_fill")
        },
    );
}

/// What Smart Fill gives the fill or the outline of the objects it makes.
pub const SMART_CHOICES: [&str; 3] = [
    "toolbar.smart_default",
    "toolbar.smart_specify",
    "toolbar.smart_none",
];

/// The Smart Fill tool's bar.
pub fn smart_fill_bar(app: &mut App, ui: &mut Ui) {
    let palette = app.palette.clone();
    dim(ui, tr("toolbar.fill_options"));
    let mut mode = app.area_fill_mode.min(2);
    egui::ComboBox::from_id_salt("smart_fill_mode")
        .selected_text(tr(SMART_CHOICES[mode as usize]))
        .width(110.0)
        .show_ui(ui, |ui| {
            for (i, key) in SMART_CHOICES.iter().enumerate() {
                ui.selectable_value(&mut mode, i as u8, tr(key));
            }
        })
        .response
        .rich_tip(crate::ui::tips::pair(
            "toolbar.smart_fill_mode",
            "toolbar.smart_fill_mode_desc",
        ));
    app.area_fill_mode = mode;
    color_pick(
        ui,
        &palette,
        "smart_fill",
        &mut app.area_fill_color,
        &tr("toolbar.fill_color"),
        mode == 1,
    );
    crate::ui::propbar::sep(ui);
    dim(ui, tr("toolbar.outline_options"));
    let mut omode = app.area_outline_mode.min(2);
    egui::ComboBox::from_id_salt("smart_outline_mode")
        .selected_text(tr(SMART_CHOICES[omode as usize]))
        .width(110.0)
        .show_ui(ui, |ui| {
            for (i, key) in SMART_CHOICES.iter().enumerate() {
                ui.selectable_value(&mut omode, i as u8, tr(key));
            }
        })
        .response
        .rich_tip(crate::ui::tips::pair(
            "toolbar.smart_outline_mode",
            "toolbar.smart_outline_mode_desc",
        ));
    app.area_outline_mode = omode;
    let mut w = app.units.from_mm(app.area_outline_width);
    let r = ui.add_enabled(
        omode == 1,
        NumField::new(&mut w)
            .speed(0.05)
            .range(0.0..=1000.0)
            .suffix(format!(" {}", app.units.short())),
    );
    let tip = crate::ui::tips::pair("toolbar.outline_width", "toolbar.smart_outline_width_desc");
    let r = r.rich_tip(&tip).rich_disabled_tip(&tip);
    if r.changed() {
        app.area_outline_width = app.units.to_mm(w).max(0.0);
    }
    color_pick(
        ui,
        &palette,
        "smart_outline",
        &mut app.area_outline_color,
        &tr("toolbar.outline_color"),
        omode == 1,
    );
    crate::ui::propbar::sep(ui);
    dim(ui, tr("toolbar.smart_fill_hint"));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smart Fill makes the object its bar says: the default fill and
    /// outline, its own colour and outline, or neither.
    #[test]
    fn smart_fill_takes_the_bars_fill_and_outline() {
        use tracedraw_core::{Fill, ShapeKind};
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: tracedraw_core::Rect::new(20.0, 20.0, 60.0, 60.0),
            radius: 0.0,
            corners: None,
        })
        .expect("shape");
        let made = |app: &mut App| {
            app.area_fill_at(tracedraw_core::Point::new(40.0, 40.0));
            let id = app.selection[0];
            let s = app.doc().find_shape(id).cloned().expect("filled area");
            app.undo();
            s
        };
        let s = made(&mut app);
        assert!(matches!(s.fill, Fill::Solid(_)));
        assert_eq!(s.stroke, app.default_stroke());
        let b = s.bounds();
        assert!((b.width() - 40.0).abs() < 1.0, "{b:?}");
        app.area_fill_mode = 1;
        app.area_fill_color = Color::rgb8(10, 200, 30);
        app.area_outline_mode = 1;
        app.area_outline_width = 1.5;
        app.area_outline_color = Color::rgb8(200, 0, 0);
        let s = made(&mut app);
        assert_eq!(s.fill, Fill::Solid(Color::rgb8(10, 200, 30)));
        let st = s.stroke.expect("outline");
        assert_eq!((st.width, st.color), (1.5, Color::rgb8(200, 0, 0)));
        app.area_fill_mode = 2;
        app.area_outline_mode = 2;
        let s = made(&mut app);
        assert_eq!(s.fill, Fill::None);
        assert!(s.stroke.is_none());
    }

    /// Smart Fill fills an area enclosed by a curve crossing itself, off
    /// the page on the desktop and in a drawing far larger than the page;
    /// a click in an open area makes nothing (#499).
    #[test]
    fn smart_fill_works_off_the_page_in_a_large_drawing() {
        use tracedraw_core::geometry::BezPath;
        use tracedraw_core::ShapeKind;
        let mut app = App::headless();
        // A bow tie 2 m wide, right of an A4 page: two triangles meeting
        // where the curve crosses itself at (1500, 500).
        let mut path = BezPath::new();
        path.move_to((500.0, 0.0));
        path.line_to((2500.0, 1000.0));
        path.line_to((2500.0, 0.0));
        path.line_to((500.0, 1000.0));
        path.close_path();
        app.new_shape(ShapeKind::Path { path, closed: true })
            .expect("curve");
        let before = app
            .doc()
            .all_layers()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        app.area_fill_at(tracedraw_core::Point::new(2200.0, 500.0));
        let id = *app.selection.first().expect("an area");
        let b = app.doc().find_shape(id).expect("area").bounds();
        // The right triangle: from the crossing to the right edge.
        assert!(
            (b.x0 - 1500.0).abs() < 5.0 && (b.x1 - 2500.0).abs() < 5.0,
            "{b:?}"
        );
        assert!(
            (b.y0 - 0.0).abs() < 5.0 && (b.y1 - 1000.0).abs() < 5.0,
            "{b:?}"
        );
        let count = |app: &App| {
            app.doc()
                .all_layers()
                .map(|l| l.shapes.len())
                .sum::<usize>()
        };
        assert_eq!(count(&app), before + 1);
        // Above the tie, open to the outside: nothing.
        app.area_fill_at(tracedraw_core::Point::new(1500.0, 900.0));
        assert_eq!(count(&app), before + 1);
    }

    /// Both bars draw, with and without a mesh and a chosen node.
    #[test]
    fn the_fill_tool_bars_draw() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 40.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        for step in 0..3 {
            if step == 1 {
                app.select(vec![id]);
                app.apply_mesh_fill(id);
            }
            if step == 2 {
                app.selected_effect_node = Some((id, crate::mesh_tool::MESH_NODE_BASE + 4));
                app.area_fill_mode = 1;
                app.area_outline_mode = 1;
            }
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1600.0, 200.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| mesh_bar(&mut app, ui));
                ui.horizontal(|ui| smart_fill_bar(&mut app, ui));
            });
            out.textures_delta.clear();
        }
    }
}
