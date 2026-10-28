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
    let shapes = app.selected_rc();
    // An object inside a group is on its group's layer.
    let layer_of = |id| {
        let doc = app.doc();
        doc.top_level_of(id)
            .and_then(|top| doc.shape(top).ok())
            .map(|(l, _)| l.name.clone())
            .unwrap_or_default()
    };
    match shapes.as_slice() {
        [] => String::new(),
        [s] => {
            if app.tool == Tool::Shape {
                if let ShapeKind::Path { path, .. } = &s.kind {
                    // The nodes the Shape tool shows (a closed curve's start
                    // and the end lying on it count once).
                    let n = tracedraw_core::nodes::nodes(path).len();
                    return trf("status.curve_nodes", &[("n", &n.to_string())]);
                }
                // A rectangle, ellipse or polygon is still that object,
                // not a curve: say so, and what its nodes do.
                let hint = match &s.kind {
                    ShapeKind::Rect { .. } => Some("status.shape_rectangle"),
                    ShapeKind::Ellipse { .. } => Some("status.shape_ellipse"),
                    ShapeKind::Polygon { .. } => Some("status.shape_polygon"),
                    _ => None,
                };
                if let Some(key) = hint {
                    return tr(key);
                }
            }
            let mut kind = match &s.kind {
                ShapeKind::Path { .. } => tr("kind.curve"),
                k => kind_name(k),
            };
            if app.is_effect_clone(s.id) {
                kind = format!("{kind} ({})", tr("status.effect_clone"));
            }
            if app.doc().is_nested(s.id) {
                kind = trf("status.in_group", &[("k", &kind)]);
            }
            trf(
                "status.object_on_layer",
                &[("k", &kind), ("l", &layer_of(s.id))],
            )
        }
        many => {
            // The layer of each object on a layer, from one pass (a search
            // of the drawing for each made Select All slow, #367).
            let mut on_layer: std::collections::HashMap<tracedraw_core::ShapeId, &str> =
                std::collections::HashMap::new();
            for l in app.doc().all_layers() {
                for s in &l.shapes {
                    on_layer.entry(s.id).or_insert(l.name.as_str());
                }
            }
            let layer_of = |id| match on_layer.get(&id) {
                Some(name) => name.to_string(),
                None => layer_of(id),
            };
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
    let f = |mm: f64| format!("{:.*}", d, u.world_from_mm(mm));
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

/// While the Pick tool moves, scales, rotates or skews the selection:
/// how far, how much or by what angle, and whether a copy takes it.
pub(crate) fn drag_info(app: &App) -> Option<String> {
    use crate::app::{Drag, Handle};
    if !app.transforming() {
        return None;
    }
    let u = app.units;
    let d = app.settings.precision.min(10) as usize;
    let len = |mm: f64| format!("{:.*} {}", d, u.world_from_mm(mm), u.label());
    let text = match &app.drag {
        Drag::Move { total, .. } => trf(
            "status.drag_move",
            &[
                ("dx", &len(total.x)),
                ("dy", &len(total.y)),
                ("d", &len(total.hypot())),
                ("a", &format!("{:.1}", total.y.atan2(total.x).to_degrees())),
            ],
        ),
        drag @ Drag::Scale { .. } => {
            let t = app.preview_transform_of(drag)?.as_coeffs();
            trf(
                "status.drag_scale",
                &[
                    ("sx", &format!("{:.1}", t[0] * 100.0)),
                    ("sy", &format!("{:.1}", t[3] * 100.0)),
                ],
            )
        }
        Drag::Rotate {
            start_angle,
            current_angle,
            ..
        } => {
            // Counter-clockwise positive, within -180 to 180.
            let mut a = (current_angle - start_angle).to_degrees();
            a = (a + 180.0).rem_euclid(360.0) - 180.0;
            trf("status.drag_rotate", &[("a", &format!("{a:.1}"))])
        }
        drag @ Drag::Skew { handle, .. } => {
            let t = app.preview_transform_of(drag)?.as_coeffs();
            let k = if matches!(handle, Handle::N | Handle::S) {
                t[2]
            } else {
                t[1]
            };
            trf(
                "status.drag_skew",
                &[("a", &format!("{:.1}", k.atan().to_degrees()))],
            )
        }
        _ => return None,
    };
    Some(if app.drag_copy {
        format!("{text}    {}", tr("status.drag_copy_short"))
    } else {
        text
    })
}

fn cursor_coordinates(app: &App) -> String {
    match app.pointer_page.map(|p| app.to_ruler(p)) {
        Some(p) => {
            let u = app.units;
            let d = app.settings.precision.min(10) as usize;
            format!(
                "({:.*}, {:.*}) {}",
                d,
                u.world_from_mm(p.x),
                d,
                u.world_from_mm(p.y),
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
    // The separator between the bar and the drawing window, the colour of
    // the palette lines: over the bar at the bottom of the window, under
    // it at the top.
    let at_top = app.settings.bars.dock(crate::ui::bars::Bar::Status) == crate::ui::bars::Dock::Top;
    let top = full.top().round();
    let line = if at_top {
        (full.bottom() - 1.0).round()
    } else {
        top
    };
    ui.painter().rect_filled(
        Rect::from_min_size(Pos2::new(full.left(), line), Vec2::new(full.width(), 1.0)),
        0.0,
        crate::theme::Tokens::border(),
    );
    let row = if at_top {
        Rect::from_min_max(
            Pos2::new(full.left() + 4.0, top),
            Pos2::new(full.right() - 4.0, line),
        )
    } else {
        Rect::from_min_max(
            Pos2::new(full.left() + 4.0, top + 1.0),
            Pos2::new(full.right() - 4.0, full.bottom().max(top + BAR_H)),
        )
    };
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
        painter.rect_filled(gear, 2.0, Tokens::tool_hover());
    }
    crate::ui::icons::draw_action(
        &painter,
        gear.shrink(3.0),
        crate::ui::icons::Action::Options,
        Tokens::icon(),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            gear.right_bottom() + Vec2::new(-1.0, -1.0),
            gear.right_bottom() + Vec2::new(-5.0, -1.0),
            gear.right_bottom() + Vec2::new(-1.0, -5.0),
        ],
        Tokens::text_dim(),
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

    // Fill and outline of the selection, or of new objects.
    let shapes = app.selected_rc();
    // A clip frame shows its frame's, a group its first member's.
    let (fill, stroke) = match app.first_paint_target().as_ref() {
        Some(s) => (s.fill.clone(), s.stroke.clone()),
        None => (app.default_fill(), app.default_stroke()),
    };
    // Values that differ across the selection read "Mixed", as in the
    // Properties panel.
    let several = shapes.len() > 1;
    let fill_mixed = several && !app.selection_fill_mix().is_empty();
    let stroke_mixed = several && !app.selection_stroke_mix().is_empty();
    let (fill_text, fill_swatch, outline_label, outline_swatch) = if doc {
        (
            if fill_mixed {
                tr("docker.mixed")
            } else {
                crate::app::fill_description(&fill)
            },
            if fill_mixed {
                None
            } else {
                crate::app::fill_preview_color(&fill)
            },
            if stroke_mixed {
                tr("docker.mixed")
            } else {
                outline_text(app, &stroke)
            },
            if stroke_mixed {
                None
            } else {
                stroke.as_ref().map(|s| crate::canvas::to_color32(s.color))
            },
        )
    } else {
        (
            tr("status.fill_color"),
            None,
            tr("status.outline_color"),
            None,
        )
    };
    let proof = Rect::from_center_size(Pos2::new(row.max.x - 17.0, cy), Vec2::splat(26.0));
    // The light and dark interface button, left of it.
    let theme_btn = proof.translate(Vec2::new(-30.0, 0.0));
    // A line sets the two buttons apart.
    let sep_x = (theme_btn.min.x - 8.0).round() + 0.5;
    painter.line_segment(
        [
            Pos2::new(sep_x, row.min.y + 7.0),
            Pos2::new(sep_x, row.max.y - 7.0),
        ],
        Stroke::new(1.0, Tokens::border()),
    );

    // Positions of the fill and outline parts: fractions of the bar (fill
    // about 70 %, outline about 82 %) while their swatches and values fit
    // there, else further left, so a narrow window cuts the hints before
    // them (#1004).
    let outline_end = theme_btn.min.x - 8.0;
    let gear_end = gear.max.x + 3.0;
    let (fill_x, outline_x) = part_positions(
        row,
        gear_end,
        outline_end,
        Indicator::width_for(&painter, &fill_text),
        Indicator::width_for(&painter, &outline_label),
    );

    // The status message can be a link to Options > General (the note
    // that hardware acceleration is off), while it is the one shown.
    let drag = if doc { drag_info(app) } else { None };
    let linked = doc
        && drag.is_none()
        && app.settings.status_info == StatusInfo::ToolHints
        && !app.status.is_empty()
        && app.status_opens_options.as_deref() == Some(app.status.as_str());
    // Left field and object information.
    let (left, info) = if !doc {
        (tr("status.tool_hints"), tr("status.object_information"))
    } else if let Some(drag) = drag {
        (drag, object_info(app))
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
    let text_x = gear_end;
    let info_w = if info.is_empty() {
        0.0
    } else {
        painter
            .layout_no_wrap(info.clone(), font.clone(), Tokens::text())
            .size()
            .x
            .min((fill_x - text_x) * 0.45)
    };
    let left_max = fill_x - 16.0 - if info_w > 0.0 { info_w + 24.0 } else { 0.0 };
    let left_galley = elided(&painter, left, left_max - text_x);
    let left_w = left_galley.size().x.min(left_max - text_x).max(0.0);
    let clip = Rect::from_min_max(Pos2::new(text_x, row.min.y), Pos2::new(left_max, row.max.y));
    painter.with_clip_rect(clip).galley(
        Pos2::new(text_x, cy - left_galley.size().y / 2.0),
        left_galley,
        Tokens::text(),
    );
    if linked {
        let r = Rect::from_min_max(
            Pos2::new(text_x, row.min.y),
            Pos2::new(text_x + left_w, row.max.y),
        );
        let resp = ui
            .interact(r, egui::Id::new("status_opens_options"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.clicked() {
            app.status_opens_options = None;
            app.status.clear();
            app.open_general_options();
        }
    }
    if info_w > 0.0 {
        // After the left field, never closer than a placeholder's width.
        let x = (text_x + left_w + 24.0)
            .max(text_x + 156.0)
            .min(left_max + 24.0);
        let g = elided(&painter, info, fill_x - 8.0 - x);
        let clip = Rect::from_min_max(Pos2::new(x, row.min.y), Pos2::new(fill_x - 8.0, row.max.y));
        painter
            .with_clip_rect(clip)
            .galley(Pos2::new(x, cy - g.size().y / 2.0), g, Tokens::text());
    }

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
    // A double click opens the dialog box, as in the reference program
    // (#465): Edit Fill for the fill, Outline Pen for the outline.
    if fill_hit && doc {
        app.open_edit_fill();
    }
    let outline_part = Rect::from_min_max(
        Pos2::new(outline_x, row.min.y),
        Pos2::new(theme_btn.min.x - 8.0, row.max.y),
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
        app.open_outline_pen();
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
            Tokens::tool_active()
        } else {
            Tokens::tool_hover()
        };
        painter.rect_filled(proof, 2.0, fill);
    }
    proof_icon(&painter, proof.shrink(2.0));

    // Light or dark interface: a moon while light (click for dark), a sun
    // while dark.
    let dark = crate::theme::theme().is_dark();
    let theme_resp = ui
        .interact(theme_btn, egui::Id::new("status_theme"), Sense::click())
        .on_hover_text(if dark {
            tr("status.theme_to_light")
        } else {
            tr("status.theme_to_dark")
        });
    if theme_resp.hovered() {
        painter.rect_filled(theme_btn, 2.0, Tokens::tool_hover());
    }
    let behind = if theme_resp.hovered() {
        Tokens::tool_hover()
    } else {
        Tokens::panel()
    };
    theme_icon(&painter, theme_btn.shrink(5.0), dark, behind);
    if theme_resp.clicked() {
        app.settings.theme = crate::theme::toggled(crate::theme::theme())
            .id()
            .to_string();
    }

    if proof_resp.clicked() && doc {
        app.proof_colors = !app.proof_colors;
        app.status = if app.proof_colors {
            tr("status.proof_turned_on")
        } else {
            tr("status.proof_turned_off")
        };
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
        crate::ui::icons::draw(painter, icon_r, self.icon, Tokens::icon());
        let sw = Rect::from_min_size(
            Pos2::new(icon_r.max.x + 3.0, (cy - 12.5).round()),
            Vec2::new(26.0, 25.0),
        );
        draw_swatch(painter, sw, self.swatch);
        let clip = Rect::from_min_max(Pos2::new(sw.max.x + 5.0, part.min.y), part.max);
        let g = elided(painter, self.text.to_owned(), part.max.x - sw.max.x - 5.0);
        painter.with_clip_rect(clip).galley(
            Pos2::new(sw.max.x + 5.0, cy - g.size().y / 2.0),
            g,
            Tokens::text(),
        );
        resp.double_clicked()
    }

    /// The width of the part without its text: icon, swatch and gaps.
    const MIN_W: f32 = 22.0 + 3.0 + 26.0 + 5.0;

    /// The width a part needs to show `text` whole.
    fn width_for(painter: &egui::Painter, text: &str) -> f32 {
        let w = painter
            .layout_no_wrap(
                text.to_owned(),
                egui::FontId::proportional(TEXT),
                Tokens::text(),
            )
            .size()
            .x;
        Self::MIN_W + w.ceil() + 2.0
    }
}

/// Where the fill and outline parts of bar `row` start: at about 70 %
/// and 82 % of it, or further left when the part (`fill_w`, `outline_w`
/// wide with its text) would not fit before the next one or before
/// `outline_end`; never left of `gear_end`, the end of the settings button.
fn part_positions(
    row: Rect,
    gear_end: f32,
    outline_end: f32,
    fill_w: f32,
    outline_w: f32,
) -> (f32, f32) {
    let outline_x = (row.min.x + row.width() * 0.818)
        .min(outline_end - outline_w)
        .max(gear_end + Indicator::MIN_W + 8.0);
    let fill_x = (row.min.x + row.width() * 0.685)
        .min(outline_x - 8.0 - fill_w)
        .max(gear_end);
    (fill_x, outline_x)
}

/// `text` on one line in the bar's font, cut with an ellipsis when it is
/// wider than `width`.
fn elided(painter: &egui::Painter, text: String, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(
        text,
        egui::FontId::proportional(TEXT),
        Tokens::text(),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(0.0));
    painter.layout_job(job)
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

/// A sun (`sun`) or a crescent moon in `r`, in the icon colour; `behind`
/// is the colour under it (cut out of the moon).
fn theme_icon(painter: &egui::Painter, r: Rect, sun: bool, behind: Color32) {
    let c = r.center();
    let rad = r.width().min(r.height()) / 2.0;
    let color = Tokens::icon();
    if sun {
        painter.circle_filled(c, rad * 0.45, color);
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::TAU / 8.0;
            let d = Vec2::new(a.cos(), a.sin());
            painter.line_segment([c + d * rad * 0.65, c + d * rad], Stroke::new(1.4, color));
        }
    } else {
        // A disc with a disc of the bar's colour cut out of its right.
        painter.circle_filled(c, rad * 0.85, color);
        painter.circle_filled(c + Vec2::new(rad * 0.45, -rad * 0.3), rad * 0.7, behind);
    }
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

    /// On a wide bar the fill and outline sit at their usual places; on a
    /// narrow one they move left so their swatches and values stay whole
    /// before the buttons, and the hints give way (#1004).
    #[test]
    fn fill_and_outline_stay_whole_on_a_narrow_bar() {
        let bar = |w: f32| Rect::from_min_size(Pos2::new(4.0, 0.0), Vec2::new(w, 41.0));
        let (fill_w, outline_w) = (110.0, 190.0);
        let wide = bar(1600.0);
        let end = wide.max.x - 68.0;
        let (f, o) = part_positions(wide, 31.0, end, fill_w, outline_w);
        assert!((f - (4.0 + 1600.0 * 0.685)).abs() < 1e-3);
        assert!((o - (4.0 + 1600.0 * 0.818)).abs() < 1e-3);
        for w in [500.0, 700.0, 900.0] {
            let row = bar(w);
            let end = row.max.x - 68.0;
            let (f, o) = part_positions(row, 31.0, end, fill_w, outline_w);
            assert!(o + outline_w <= end + 1e-3, "{w}: outline cut");
            assert!(f + fill_w <= o - 8.0 + 1e-3, "{w}: fill cut");
            assert!(f >= 31.0, "{w}");
        }
        // Too narrow for both: they keep at least their swatches, in order.
        let row = bar(200.0);
        let (f, o) = part_positions(row, 31.0, row.max.x - 68.0, fill_w, outline_w);
        assert!(f >= 31.0 && o >= f + Indicator::MIN_W, "{f} {o}");
    }

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
