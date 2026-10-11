//! The status bar: a settings button
//! whose menu picks what the left field shows (tool hints, object details,
//! cursor coordinates or the document colour settings), the object
//! information, the fill and outline of the selection (or of new objects)
//! with their swatches, and the proof colours button at the right end.
//! Double-clicking the fill or outline part opens its editor.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::dockers::kind_name;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use serde::{Deserialize, Serialize};
use tracedraw_core::document::ShapeKind;

/// Height of the bar: a 1 px line, then 41 px.
pub const BAR_H: f32 = 42.0;
/// Text size of the bar.
const TEXT: f32 = 13.0;

/// What the status bar's first field shows (the settings button's menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StatusInfo {
    #[default]
    ToolHints,
    ObjectDetails,
    CursorCoordinates,
    DocumentColor,
}

impl StatusInfo {
    pub const ALL: [StatusInfo; 4] = [
        StatusInfo::ToolHints,
        StatusInfo::ObjectDetails,
        StatusInfo::CursorCoordinates,
        StatusInfo::DocumentColor,
    ];

    fn label(self) -> String {
        tr(match self {
            StatusInfo::ToolHints => "status.tool_hints",
            StatusInfo::ObjectDetails => "status.object_details",
            StatusInfo::CursorCoordinates => "status.cursor_coordinates",
            StatusInfo::DocumentColor => "status.document_color_settings",
        })
    }
}

/// The active tool's hint; the Pick tool's depends on the selection.
pub fn tool_hint(app: &App) -> String {
    tool_hint_for(app.tool, app.selection.is_empty())
}

/// A tool's hint, with or without a selection.
pub fn tool_hint_for(tool: Tool, nothing_selected: bool) -> String {
    if tool == Tool::Pick && !nothing_selected {
        return tr("status_hint.pick_selected");
    }
    tr(&format!("status_hint.{}", tool.id()))
}

/// "Rectangle on Layer 1", "3 Objects Selected on Layer 1", and with the
/// Shape tool "Curve: 17 Nodes". Empty with nothing selected.
pub fn object_info(app: &App) -> String {
    if app.text_edit.is_some() {
        return tr("status.editing_text");
    }
    let shapes = app.selected_shapes();
    let layer_of = |id| {
        app.doc()
            .shape(id)
            .map(|(l, _)| l.name.clone())
            .unwrap_or_default()
    };
    match shapes.as_slice() {
        [] => String::new(),
        [s] => {
            if app.tool == Tool::Shape {
                if let ShapeKind::Path { path, .. } = &s.kind {
                    let n = path
                        .elements()
                        .iter()
                        .filter(|e| !matches!(e, tracedraw_core::geometry::PathEl::ClosePath))
                        .count();
                    return trf("status.curve_nodes", &[("n", &n.to_string())]);
                }
            }
            let mut kind = match &s.kind {
                ShapeKind::Path { .. } => tr("kind.curve"),
                k => kind_name(k),
            };
            if app.is_effect_clone(s.id) {
                kind = format!("{kind} ({})", tr("status.effect_clone"));
            }
            trf(
                "status.object_on_layer",
                &[("k", &kind), ("l", &layer_of(s.id))],
            )
        }
        many => {
            let first = layer_of(many[0].id);
            let same = many.iter().all(|s| layer_of(s.id) == first);
            let n = many.len().to_string();
            if same {
                trf("status.n_selected_on_layer", &[("n", &n), ("l", &first)])
            } else {
                trf("status.n_selected_on_layers", &[("n", &n)])
            }
        }
    }
}

/// The selection's size and centre in the drawing units.
fn object_details(app: &App) -> String {
    let Some(b) = app.selection_bounds() else {
        return String::new();
    };
    let u = app.units;
    let d = app.settings.precision.min(10) as usize;
    let f = |mm: f64| format!("{:.*}", d, u.from_mm(mm));
    let c = app.to_ruler(b.center());
    trf(
        "status.details",
        &[
            ("w", &f(b.width())),
            ("h", &f(b.height())),
            ("x", &f(c.x)),
            ("y", &f(c.y)),
            ("u", &u.label()),
        ],
    )
}

fn cursor_coordinates(app: &App) -> String {
    match app.pointer_page.map(|p| app.to_ruler(p)) {
        Some(p) => {
            let u = app.units;
            let d = app.settings.precision.min(10) as usize;
            format!(
                "({:.*}, {:.*}) {}",
                d,
                u.from_mm(p.x),
                d,
                u.from_mm(p.y),
                u.label()
            )
        }
        None => String::new(),
    }
}

fn document_color(app: &App) -> String {
    let m = &app.doc().metadata;
    let pick = |doc: &str, default: &str| {
        if doc.is_empty() {
            default.to_string()
        } else {
            doc.to_string()
        }
    };
    trf(
        "status.document_colors",
        &[
            (
                "rgb",
                &pick(&m.rgb_profile, &app.settings.color.rgb_profile),
            ),
            (
                "cmyk",
                &pick(&m.cmyk_profile, &app.settings.color.cmyk_profile),
            ),
            ("gray", &app.settings.new_document.gray_profile),
        ],
    )
}

/// The outline as the status bar describes it: colour and width in the
/// drawing units (or "Hairline").
fn outline_text(app: &App, stroke: &Option<tracedraw_core::Stroke>) -> String {
    match stroke {
        None => tr("status.none"),
        Some(s) => {
            let width = if s.width <= tracedraw_core::Stroke::HAIRLINE + 1e-9 {
                tr("status.hairline")
            } else {
                let u = app.units;
                let decimals = if u == crate::app::Units::Pixels { 2 } else { 3 };
                format!("{:.*} {}", decimals, u.from_mm(s.width), u.short())
            };
            format!("{}  {}", crate::app::color_description(s.color), width)
        }
    }
}

pub fn status_bar(app: &mut App, ui: &mut Ui) {
    let full = ui.available_rect_before_wrap();
    // The separator over the bar, the colour of the palette lines.
    let top = full.top().round();
    ui.painter().rect_filled(
        Rect::from_min_size(Pos2::new(full.left(), top), Vec2::new(full.width(), 1.0)),
        0.0,
        egui::Color32::from_gray(0xD8),
    );
    let row = Rect::from_min_max(
        Pos2::new(full.left() + 4.0, top + 1.0),
        Pos2::new(full.right() - 4.0, full.bottom().max(top + BAR_H)),
    );
    ui.allocate_rect(full, Sense::hover());
    let painter = ui.painter_at(row);
    let cy = row.center().y;
    let font = egui::FontId::proportional(TEXT);
    let doc = app.has_document() && !app.show_welcome;

    // The settings button and its menu: a gear with a small corner arrow.
    let gear = Rect::from_center_size(Pos2::new(row.min.x + 14.0, cy), Vec2::splat(26.0));
    let gear_resp = ui
        .interact(gear, egui::Id::new("status_settings"), Sense::click())
        .on_hover_text(tr("status.bar_options"));
    if gear_resp.hovered() {
        painter.rect_filled(gear, 2.0, Tokens::TOOL_HOVER);
    }
    crate::ui::icons::draw_action(
        &painter,
        gear.shrink(3.0),
        crate::ui::icons::Action::Options,
        Tokens::ICON,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            gear.right_bottom() + Vec2::new(-1.0, -1.0),
            gear.right_bottom() + Vec2::new(-5.0, -1.0),
            gear.right_bottom() + Vec2::new(-1.0, -5.0),
        ],
        Tokens::TEXT_DIM,
        Stroke::NONE,
    ));
    egui::Popup::menu(&gear_resp)
        .id(egui::Id::new("status_settings_menu"))
        .show(|ui| {
            for mode in StatusInfo::ALL {
                if ui
                    .radio(app.settings.status_info == mode, mode.label())
                    .clicked()
                {
                    app.settings.status_info = mode;
                    ui.close();
                }
            }
        });

    // Positions of the right-hand parts, as fractions of the bar (fill
    // about 70 %, outline about 82 %).
    let fill_x = row.min.x + row.width() * 0.685;
    let outline_x = row.min.x + row.width() * 0.818;
    let proof = Rect::from_center_size(Pos2::new(row.max.x - 17.0, cy), Vec2::splat(26.0));
    // A line sets the proof colours button apart.
    let sep_x = (proof.min.x - 8.0).round() + 0.5;
    painter.line_segment(
        [
            Pos2::new(sep_x, row.min.y + 7.0),
            Pos2::new(sep_x, row.max.y - 7.0),
        ],
        Stroke::new(1.0, Tokens::BORDER),
    );

    // Left field and object information.
    let (left, info) = if !doc {
        (tr("status.tool_hints"), tr("status.object_information"))
    } else {
        let left = match app.settings.status_info {
            StatusInfo::ToolHints => {
                let hint = tool_hint(app);
                if app.status.is_empty() {
                    hint
                } else {
                    format!("{}    {hint}", app.status)
                }
            }
            StatusInfo::ObjectDetails => object_details(app),
            StatusInfo::CursorCoordinates => cursor_coordinates(app),
            StatusInfo::DocumentColor => document_color(app),
        };
        (left, object_info(app))
    };
    let text_x = gear.max.x + 3.0;
    let info_w = if info.is_empty() {
        0.0
    } else {
        painter
            .layout_no_wrap(info.clone(), font.clone(), Tokens::TEXT)
            .size()
            .x
            .min((fill_x - text_x) * 0.45)
    };
    let left_max = fill_x - 16.0 - if info_w > 0.0 { info_w + 24.0 } else { 0.0 };
    let left_galley = painter.layout_no_wrap(left, font.clone(), Tokens::TEXT);
    let left_w = left_galley.size().x.min(left_max - text_x).max(0.0);
    let clip = Rect::from_min_max(Pos2::new(text_x, row.min.y), Pos2::new(left_max, row.max.y));
    painter.with_clip_rect(clip).galley(
        Pos2::new(text_x, cy - left_galley.size().y / 2.0),
        left_galley,
        Tokens::TEXT,
    );
    if info_w > 0.0 {
        // After the left field, never closer than a placeholder's width.
        let x = (text_x + left_w + 24.0)
            .max(text_x + 156.0)
            .min(left_max + 24.0);
        let g = painter.layout_no_wrap(info, font.clone(), Tokens::TEXT);
        let clip = Rect::from_min_max(Pos2::new(x, row.min.y), Pos2::new(fill_x - 8.0, row.max.y));
        painter
            .with_clip_rect(clip)
            .galley(Pos2::new(x, cy - g.size().y / 2.0), g, Tokens::TEXT);
    }

    // Fill and outline of the selection, or of new objects.
    let shapes = app.selected_shapes();
    let (fill, stroke) = match shapes.first() {
        Some(s) => (s.fill.clone(), s.stroke.clone()),
        None => (app.default_fill.clone(), app.default_stroke.clone()),
    };
    let (fill_text, fill_swatch, outline_label, outline_swatch) = if doc {
        (
            crate::app::fill_description(&fill),
            crate::app::fill_preview_color(&fill),
            outline_text(app, &stroke),
            stroke.as_ref().map(|s| crate::canvas::to_color32(s.color)),
        )
    } else {
        (
            tr("status.fill_color"),
            None,
            tr("status.outline_color"),
            None,
        )
    };
    let fill_part = Rect::from_min_max(
        Pos2::new(fill_x, row.min.y),
        Pos2::new(outline_x - 8.0, row.max.y),
    );
    let fill_hit = Indicator {
        id: "status_fill",
        icon: Tool::InteractiveFill,
        swatch: fill_swatch,
        text: &fill_text,
        tip: &tr("status.swatch_fill"),
    }
    .show(ui, &painter, fill_part);
    if fill_hit && doc {
        app.open_fill_editor();
    }
    let outline_part = Rect::from_min_max(
        Pos2::new(outline_x, row.min.y),
        Pos2::new(proof.min.x - 8.0, row.max.y),
    );
    let outline_hit = Indicator {
        id: "status_outline",
        icon: Tool::OutlinePen,
        swatch: outline_swatch,
        text: &outline_label,
        tip: &tr("status.swatch_outline"),
    }
    .show(ui, &painter, outline_part);
    if outline_hit && doc {
        app.open_outline_editor();
    }

    // Proof colours.
    let proof_resp = ui
        .interact(proof, egui::Id::new("status_proof"), Sense::click())
        .on_hover_text(if app.proof_colors {
            tr("status.proof_on")
        } else {
            tr("status.proof_off")
        });
    if proof_resp.hovered() || app.proof_colors {
        let fill = if app.proof_colors {
            Tokens::TOOL_ACTIVE
        } else {
            Tokens::TOOL_HOVER
        };
        painter.rect_filled(proof, 2.0, fill);
    }
    proof_icon(&painter, proof.shrink(2.0));
    if proof_resp.clicked() && doc {
        app.proof_colors = !app.proof_colors;
        app.raster.borrow_mut().invalidate();
    }
}

/// The fill or outline part: tool icon, swatch, description.
struct Indicator<'a> {
    id: &'a str,
    icon: Tool,
    swatch: Option<Color32>,
    text: &'a str,
    tip: &'a str,
}

impl Indicator<'_> {
    /// Draw it; true when double-clicked.
    fn show(&self, ui: &mut Ui, painter: &egui::Painter, part: Rect) -> bool {
        let resp = ui
            .interact(part, egui::Id::new(self.id), Sense::click())
            .on_hover_text(self.tip);
        let cy = part.center().y;
        let icon_r = Rect::from_center_size(Pos2::new(part.min.x + 11.0, cy), Vec2::splat(20.0));
        crate::ui::icons::draw(painter, icon_r, self.icon, Tokens::ICON);
        let sw = Rect::from_min_size(
            Pos2::new(icon_r.max.x + 3.0, (cy - 12.5).round()),
            Vec2::new(26.0, 25.0),
        );
        draw_swatch(painter, sw, self.swatch);
        let clip = Rect::from_min_max(Pos2::new(sw.max.x + 5.0, part.min.y), part.max);
        painter.with_clip_rect(clip).text(
            Pos2::new(sw.max.x + 5.0, cy),
            egui::Align2::LEFT_CENTER,
            self.text,
            egui::FontId::proportional(TEXT),
            Tokens::TEXT,
        );
        resp.double_clicked()
    }
}

/// A colour swatch; no colour is a white box crossed by a red line.
pub fn draw_swatch(painter: &egui::Painter, r: Rect, color: Option<Color32>) {
    match color {
        Some(c) => {
            painter.rect_filled(r, 0.0, c);
        }
        None => {
            painter.rect_filled(r, 0.0, Color32::WHITE);
            painter.line_segment(
                [
                    r.left_bottom() + Vec2::new(1.5, -1.5),
                    r.right_top() + Vec2::new(-1.5, 1.5),
                ],
                Stroke::new(1.5, Color32::from_rgb(0xE0, 0x20, 0x20)),
            );
        }
    };
    painter.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Color32::from_gray(0xAA)),
        egui::StrokeKind::Inside,
    );
}

/// The proof colours picture: a monitor whose screen is red over sky
/// blue with a lens in the middle, on a stand.
fn proof_icon(painter: &egui::Painter, r: Rect) {
    // Drawn on a 20 x 20 grid centred in `r`.
    let o = r.center() - Vec2::splat(10.0);
    let at = |x: f32, y: f32| o + Vec2::new(x, y);
    let dark = Color32::from_gray(0x33);
    // Frame and screen.
    painter.rect_filled(Rect::from_min_max(at(0.0, 0.0), at(20.0, 15.0)), 0.0, dark);
    painter.rect_filled(
        Rect::from_min_max(at(1.0, 1.0), at(19.0, 14.0)),
        0.0,
        Color32::from_gray(0xC8),
    );
    painter.rect_filled(
        Rect::from_min_max(at(2.0, 2.0), at(18.0, 7.5)),
        0.0,
        Color32::from_rgb(0xFF, 0x10, 0x10),
    );
    painter.rect_filled(
        Rect::from_min_max(at(2.0, 7.5), at(18.0, 13.0)),
        0.0,
        Color32::from_rgb(0x12, 0xAE, 0xF0),
    );
    // The lens.
    painter.circle_filled(at(10.0, 7.5), 3.6, Color32::WHITE);
    painter.circle_filled(at(10.0, 7.5), 2.0, Color32::from_gray(0x70));
    // Stand.
    painter.rect_filled(
        Rect::from_min_max(at(7.0, 15.0), at(13.0, 17.0)),
        0.0,
        Color32::from_gray(0x99),
    );
    painter.rect_filled(Rect::from_min_max(at(3.0, 18.0), at(17.0, 19.5)), 0.0, dark);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect as PRect;

    #[test]
    fn every_tool_has_its_own_hint() {
        crate::i18n::set_language("en");
        for t in Tool::ALL {
            let key = format!("status_hint.{}", t.id());
            assert_ne!(tr(&key), key, "missing status hint for {t:?}");
        }
    }

    #[test]
    fn object_information_names_the_object_and_its_layer() {
        crate::i18n::set_language("en");
        let mut app = App::headless();
        assert_eq!(object_info(&app), "");
        let a = app
            .new_shape(ShapeKind::Rect {
                rect: PRect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("layer");
        app.select(vec![a]);
        assert_eq!(object_info(&app), "Rectangle on Layer 1");
        let mut path = tracedraw_core::geometry::BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((5.0, 5.0));
        path.line_to((9.0, 1.0));
        let b = app
            .new_shape(ShapeKind::Path {
                path,
                closed: false,
            })
            .expect("layer");
        app.select(vec![b]);
        assert_eq!(object_info(&app), "Curve on Layer 1");
        app.tool = Tool::Shape;
        assert_eq!(object_info(&app), "Curve: 3 Nodes");
        app.tool = Tool::Pick;
        app.select(vec![a, b]);
        assert_eq!(object_info(&app), "2 Objects Selected on Layer 1");
        assert!(tool_hint(&app).contains("twice"));
    }

    #[test]
    fn outline_width_is_shown_in_the_drawing_units() {
        crate::i18n::set_language("en");
        let mut app = App::headless();
        let s = Some(tracedraw_core::Stroke {
            width: 0.2,
            ..Default::default()
        });
        app.units = crate::app::Units::Millimeters;
        assert!(outline_text(&app, &s).ends_with("0.200 mm"));
        app.units = crate::app::Units::Points;
        assert!(outline_text(&app, &s).ends_with("0.567 pt"));
        assert_eq!(outline_text(&app, &None), "None");
    }
}
