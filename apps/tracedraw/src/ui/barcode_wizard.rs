//! Object > Insert > Barcode: the Barcode Wizard. Three steps beside a
//! picture: the symbology and the data; the printer and the bar sizes;
//! the human-readable text. A sample preview under each step shows the
//! symbol as it will be made. Finish places it at the page centre as one
//! group (bars, text, the white background of the quiet zone).
//! See docs/behavior/barcode.md.

use crate::app::{App, Units};
use crate::barcode::{self, BarcodeSpec, Symbology, TextAlignment, TextOption};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::{enter_pressed, window};
use crate::ui::tips::RichTip;
use egui::{Color32, Context, Pos2, Rect as UiRect, Stroke, Ui, Vec2};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{BezPath, Point, Rect},
    Color, Command, Fill, TextAlign, TextSpan,
};

/// The printer resolutions offered.
pub const DPI: [u32; 6] = [150, 203, 300, 600, 1200, 2400];
/// The units the bar height is given in.
pub const UNITS: [Units; 4] = [
    Units::Inches,
    Units::Millimeters,
    Units::Centimeters,
    Units::Points,
];

#[derive(Debug, Clone, PartialEq)]
pub struct BarcodeState {
    /// 0, 1 or 2.
    pub step: u8,
    pub spec: BarcodeSpec,
    pub units: Units,
    /// The advanced settings of the second step are shown.
    pub advanced: bool,
    pub error: String,
}

impl Default for BarcodeState {
    fn default() -> Self {
        BarcodeState {
            step: 0,
            spec: BarcodeSpec {
                symbology: Symbology::Codabar,
                data: Symbology::Codabar.sample().into(),
                ..Default::default()
            },
            units: Units::Inches,
            advanced: false,
            error: String::new(),
        }
    }
}

const STEPS: u8 = 3;
const PANEL: Vec2 = Vec2::new(150.0, 330.0);
const FORM_W: f32 = 380.0;

/// The wizard window.
pub fn dialog(app: &mut App, ctx: &Context, st: &mut BarcodeState, close: &mut bool) {
    let mut finish = false;
    let mut help = false;
    window(ctx, tr("barcode.title")).show(ctx, |ui| {
        ui.horizontal_top(|ui| {
            picture(ui, st.step);
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.set_width(FORM_W);
                match st.step {
                    0 => step_standard(ui, st),
                    1 => step_printer(ui, st),
                    _ => step_text(ui, st),
                }
                ui.add_space(8.0);
                ui.label(tr("barcode.sample_preview"));
                preview(ui, &st.spec);
                if !st.error.is_empty() {
                    ui.colored_label(Color32::from_rgb(0xC4, 0x2B, 0x1C), &st.error);
                }
            });
        });
        ui.add_space(6.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button = |ui: &mut Ui, text: String, enabled: bool, tip: String| {
                    ui.add_enabled(
                        enabled,
                        egui::Button::new(text).min_size(egui::vec2(90.0, 27.0)),
                    )
                    .rich_tip_always(tip)
                    .clicked()
                };
                if button(
                    ui,
                    tr("dialog.help"),
                    true,
                    crate::ui::tips::keyed("dialog.help_button"),
                ) {
                    help = true;
                }
                if button(
                    ui,
                    tr("dialog.cancel"),
                    true,
                    crate::ui::tips::keyed("dialog.cancel"),
                ) {
                    *close = true;
                }
                let last = st.step + 1 == STEPS;
                let next = if last {
                    "barcode.finish"
                } else {
                    "barcode.next"
                };
                if button(ui, tr(next), true, crate::ui::tips::keyed(next)) || enter_pressed(ui) {
                    match barcode::build(&st.spec) {
                        Ok(_) if last => finish = true,
                        Ok(_) => {
                            st.error.clear();
                            st.step += 1;
                        }
                        Err(key) => st.error = tr(key),
                    }
                }
                if button(
                    ui,
                    tr("barcode.back"),
                    st.step > 0,
                    crate::ui::tips::keyed("barcode.back"),
                ) {
                    st.error.clear();
                    st.step -= 1;
                }
            });
        });
    });
    if help {
        app.open_dialog_help();
    }
    if finish {
        match insert(app, &st.spec) {
            Ok(()) => *close = true,
            Err(key) => st.error = tr(key),
        }
    }
}

/// Step 1: the industry standard and the data.
fn step_standard(ui: &mut Ui, st: &mut BarcodeState) {
    ui.label(tr("barcode.welcome"));
    ui.add_space(10.0);
    ui.label(tr("barcode.select_standard"));
    let before = st.spec.symbology;
    egui::ComboBox::from_id_salt("barcode_sym")
        .width(220.0)
        .selected_text(st.spec.symbology.name())
        .show_ui(ui, |ui| {
            for s in Symbology::ALL {
                ui.selectable_value(&mut st.spec.symbology, s, s.name());
            }
        })
        .response
        .rich_tip(crate::ui::tips::pair(
            "barcode.standard",
            "barcode.standard_desc",
        ));
    if st.spec.symbology != before {
        // Data of one symbology rarely suits another: start from a sample.
        st.spec.data = st.spec.symbology.sample().into();
        st.error.clear();
    }
    ui.add_space(10.0);
    ui.label(tr(st.spec.symbology.hint_key()));
    if ui
        .add(egui::TextEdit::singleline(&mut st.spec.data).desired_width(FORM_W))
        .rich_tip_with(|| crate::ui::tips::pair("barcode.data", st.spec.symbology.hint_key()))
        .changed()
    {
        st.error.clear();
    }
}

/// Step 2: the printer and the bar sizes.
fn step_printer(ui: &mut Ui, st: &mut BarcodeState) {
    ui.label(tr("barcode.adjust_standard"));
    ui.add_space(6.0);
    let units = st.units;
    let spec = &mut st.spec;
    egui::Grid::new("barcode_printer")
        .num_columns(3)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("barcode.printer"));
            egui::ComboBox::from_id_salt("barcode_dpi")
                .width(70.0)
                .selected_text(spec.dpi.to_string())
                .show_ui(ui, |ui| {
                    for d in DPI {
                        ui.selectable_value(&mut spec.dpi, d, d.to_string());
                    }
                })
                .response
                .rich_tip(crate::ui::tips::keyed("barcode.printer"));
            ui.label(tr("barcode.dpi"));
            ui.end_row();
            ui.label(tr("barcode.units"));
            egui::ComboBox::from_id_salt("barcode_units")
                .width(70.0)
                .selected_text(st.units.label())
                .show_ui(ui, |ui| {
                    for u in UNITS {
                        ui.selectable_value(&mut st.units, u, u.label());
                    }
                })
                .response
                .rich_tip(crate::ui::tips::keyed("barcode.units"));
            ui.label("");
            ui.end_row();
            ui.label(tr("barcode.reduction"));
            ui.add(
                crate::ui::field::NumField::new(&mut spec.reduction_px)
                    .range(0.0..=10.0)
                    .speed(0.1),
            )
            .rich_tip(crate::ui::tips::keyed("barcode.reduction"));
            ui.label(tr("barcode.pixels"));
            ui.end_row();
            ui.label(tr("barcode.magnification"));
            ui.add(
                crate::ui::field::NumField::new(&mut spec.magnification)
                    .range(25.0..=1000.0)
                    .speed(1.0)
                    .max_decimals(1),
            )
            .rich_tip(crate::ui::tips::keyed("barcode.magnification"));
            ui.label("%");
            ui.end_row();
            ui.label(tr("barcode.bar"));
            let mut h = units.from_mm(spec.bar_height_mm);
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut h)
                        .speed(0.01)
                        .max_decimals(3)
                        .range(units.from_mm(1.0)..=units.from_mm(500.0)),
                )
                .rich_tip(crate::ui::tips::keyed("barcode.bar"))
                .changed()
            {
                spec.bar_height_mm = units.to_mm(h);
            }
            let width = barcode::build(spec).map(|(b, _)| b.width).unwrap_or(0.0);
            ui.label(trf(
                "barcode.symbol_width",
                &[("w", &format!("{:.4}", units.from_mm(width)))],
            ));
            ui.end_row();
            ui.label(tr("barcode.ratio"));
            ui.add_enabled_ui(spec.symbology.two_widths(), |ui| {
                ui.add(
                    crate::ui::field::NumField::new(&mut spec.ratio)
                        .range(2.0..=3.0)
                        .speed(0.05)
                        .max_decimals(2),
                )
                .rich_tip_always(crate::ui::tips::keyed("barcode.ratio"));
            });
            ui.horizontal(|ui| {
                ui.label(tr("barcode.to_one"));
                if ui
                    .button(tr("barcode.advanced"))
                    .rich_tip(crate::ui::tips::keyed("barcode.advanced"))
                    .clicked()
                {
                    st.advanced = !st.advanced;
                }
            });
            ui.end_row();
        });
    if st.advanced {
        ui.group(|ui| {
            ui.add_enabled_ui(st.spec.symbology.optional_check(), |ui| {
                ui.checkbox(&mut st.spec.check_digit, tr("barcode.check_digit"))
                    .rich_tip_always(crate::ui::tips::keyed("barcode.check_digit"));
            });
            ui.horizontal(|ui| {
                ui.label(tr("barcode.quiet_zone"));
                ui.add(crate::ui::field::NumField::new(&mut st.spec.quiet_zone).range(0..=50))
                    .rich_tip(crate::ui::tips::keyed("barcode.quiet_zone"));
                ui.label(tr("barcode.modules"));
            });
        });
    }
}

/// Step 3: the human-readable text.
fn step_text(ui: &mut Ui, st: &mut BarcodeState) {
    ui.label(tr("barcode.adjust_text"));
    ui.add_space(6.0);
    let spec = &mut st.spec;
    egui::Grid::new("barcode_text")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label(tr("barcode.font"));
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("barcode_font")
                    .width(170.0)
                    .selected_text(spec.font.clone())
                    .show_ui(ui, |ui| {
                        for f in tracedraw_text::fonts().families() {
                            let on = spec.font == f;
                            if ui.selectable_label(on, &f).clicked() {
                                spec.font = f;
                            }
                        }
                    })
                    .response
                    .rich_tip(crate::ui::tips::keyed("barcode.font"));
                ui.label(tr("barcode.size"));
                ui.add(
                    crate::ui::field::NumField::new(&mut spec.size_pt)
                        .range(4.0..=72.0)
                        .speed(0.5),
                )
                .rich_tip(crate::ui::tips::keyed("barcode.size"));
                ui.label(tr("barcode.pts"));
            });
            ui.end_row();
            ui.label(tr("barcode.weight"));
            let weights = [
                ((false, false), "barcode.weight_normal"),
                ((false, true), "barcode.weight_normal_italic"),
                ((true, false), "barcode.weight_bold"),
                ((true, true), "barcode.weight_bold_italic"),
            ];
            let current = weights
                .iter()
                .find(|(w, _)| *w == (spec.bold, spec.italic))
                .map_or("barcode.weight_normal", |(_, k)| k);
            egui::ComboBox::from_id_salt("barcode_weight")
                .width(170.0)
                .selected_text(tr(current))
                .show_ui(ui, |ui| {
                    for ((b, i), key) in weights {
                        if ui
                            .selectable_label((spec.bold, spec.italic) == (b, i), tr(key))
                            .clicked()
                        {
                            spec.bold = b;
                            spec.italic = i;
                        }
                    }
                })
                .response
                .rich_tip(crate::ui::tips::keyed("barcode.weight"));
            ui.end_row();
            ui.label(tr("barcode.alignment"));
            let aligns = [
                (TextAlignment::Left, "barcode.align_left"),
                (TextAlignment::Center, "barcode.align_center"),
                (TextAlignment::Right, "barcode.align_right"),
            ];
            let current = aligns
                .iter()
                .find(|(a, _)| *a == spec.alignment)
                .map_or("barcode.align_center", |(_, k)| k);
            egui::ComboBox::from_id_salt("barcode_align")
                .width(170.0)
                .selected_text(tr(current))
                .show_ui(ui, |ui| {
                    for (a, key) in aligns {
                        ui.selectable_value(&mut spec.alignment, a, tr(key));
                    }
                })
                .response
                .rich_tip(crate::ui::tips::keyed("barcode.alignment"));
            ui.end_row();
        });
    ui.add_space(4.0);
    let sym = spec.symbology;
    let check = spec.check_digit;
    let readable = spec.human_readable;
    let option = |ui: &mut Ui, on: &mut bool, o: TextOption, key: &str| {
        ui.add_enabled_ui(readable && sym.supports(o, check), |ui| {
            ui.checkbox(on, tr(key))
                .rich_tip_always(crate::ui::tips::keyed(key));
        });
    };
    egui::Grid::new("barcode_options")
        .num_columns(2)
        .spacing([20.0, 2.0])
        .show(ui, |ui| {
            option(
                ui,
                &mut spec.center_sys_check,
                TextOption::CenterSysCheck,
                "barcode.center_sys_check",
            );
            option(
                ui,
                &mut spec.text_above,
                TextOption::TextAbove,
                "barcode.text_above",
            );
            ui.end_row();
            option(
                ui,
                &mut spec.show_addon_text,
                TextOption::AddOnText,
                "barcode.addon_text",
            );
            option(
                ui,
                &mut spec.fact_data,
                TextOption::FactData,
                "barcode.fact_data",
            );
            ui.end_row();
            option(
                ui,
                &mut spec.addon_text_bottom,
                TextOption::AddOnBottom,
                "barcode.addon_bottom",
            );
            option(
                ui,
                &mut spec.asterisks,
                TextOption::Asterisks,
                "barcode.asterisks",
            );
            ui.end_row();
            option(
                ui,
                &mut spec.quiet_marks,
                TextOption::QuietMarks,
                "barcode.quiet_marks",
            );
            option(
                ui,
                &mut spec.start_stop,
                TextOption::StartStop,
                "barcode.start_stop",
            );
            ui.end_row();
        });
    ui.add_space(4.0);
    ui.checkbox(&mut spec.human_readable, tr("barcode.human_readable"))
        .rich_tip(crate::ui::tips::pair(
            "barcode.human_readable_title",
            "barcode.human_readable_desc",
        ));
}

/// The picture on the left: digits running up a page, with a small symbol.
fn picture(ui: &mut Ui, step: u8) {
    let (rect, _) = ui.allocate_exact_size(PANEL, egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect(
        rect,
        0.0,
        Color32::WHITE,
        Stroke::new(1.0, Color32::from_gray(0x90)),
        egui::StrokeKind::Inside,
    );
    let page = rect.shrink2(egui::vec2(18.0, 22.0));
    p.rect_stroke(
        page,
        0.0,
        Stroke::new(1.0, Color32::from_gray(0x70)),
        egui::StrokeKind::Inside,
    );
    // The digits, rotated a quarter turn.
    let galley = p.layout_no_wrap(
        "6523654789690".into(),
        egui::FontId::proportional(28.0),
        Color32::from_gray(0x50),
    );
    let at = Pos2::new(page.left() + 8.0, page.bottom() - 6.0);
    p.add(
        egui::epaint::TextShape::new(at, galley, Color32::from_gray(0x50))
            .with_angle(-std::f32::consts::FRAC_PI_2),
    );
    // A small symbol, placed differently on each step.
    let origin = match step {
        0 => Pos2::new(page.left() + 50.0, page.bottom() - 70.0),
        1 => Pos2::new(page.left() + 30.0, page.center().y),
        _ => Pos2::new(page.left() + 10.0, page.top() + 60.0),
    };
    let modules = barcode::encode(Symbology::Code128, "2026").unwrap_or_default();
    for (start, width) in barcode::bars(&modules) {
        let x = origin.x + start as f32 * 0.9;
        p.rect_filled(
            UiRect::from_min_size(Pos2::new(x, origin.y), egui::vec2(width as f32 * 0.9, 34.0)),
            0.0,
            Color32::from_gray(0x20),
        );
    }
}

/// The sample preview: the symbol scaled to fit, with its text.
fn preview(ui: &mut Ui, spec: &BarcodeSpec) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(FORM_W, 96.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect(
        rect,
        0.0,
        Color32::WHITE,
        Stroke::new(1.0, Color32::from_gray(0x90)),
        egui::StrokeKind::Inside,
    );
    let Ok((built, _)) = barcode::build(spec) else {
        return;
    };
    // Everything to show, bars and text, in symbol millimetres.
    let text_mm = spec.size_pt * 25.4 / 72.0;
    let mut bounds = Rect::new(0.0, 0.0, built.width, spec.bar_height_mm.max(1.0));
    for l in &built.labels {
        bounds = bounds.union(Rect::new(
            l.at.x - 2.0,
            l.at.y - text_mm * 0.3,
            l.at.x + 2.0,
            l.at.y + text_mm,
        ));
    }
    if let Some(bg) = built.background {
        bounds = bounds.union(bg);
    }
    let room = rect.shrink(8.0);
    let scale = (room.width() as f64 / bounds.width().max(1e-6))
        .min(room.height() as f64 / bounds.height().max(1e-6));
    let ox = room.center().x as f64 - bounds.center().x * scale;
    let oy = room.center().y as f64 + bounds.center().y * scale;
    let at = |x: f64, y: f64| Pos2::new((ox + x * scale) as f32, (oy - y * scale) as f32);
    if let Some(bg) = built.background {
        p.rect_stroke(
            UiRect::from_two_pos(at(bg.x0, bg.y0), at(bg.x1, bg.y1)),
            0.0,
            Stroke::new(1.0, Color32::from_gray(0xD0)),
            egui::StrokeKind::Inside,
        );
    }
    for b in &built.bars {
        p.rect_filled(
            UiRect::from_two_pos(at(b.x0, b.y0), at(b.x1, b.y1)),
            0.0,
            Color32::BLACK,
        );
    }
    let font = egui::FontId::proportional(((text_mm * scale) as f32).clamp(6.0, 30.0));
    for l in &built.labels {
        let align = match l.alignment {
            TextAlignment::Left => egui::Align2::LEFT_BOTTOM,
            TextAlignment::Center => egui::Align2::CENTER_BOTTOM,
            TextAlignment::Right => egui::Align2::RIGHT_BOTTOM,
        };
        p.text(
            at(l.at.x, l.at.y),
            align,
            &l.text,
            font.clone(),
            Tokens::text(),
        );
    }
}

/// The shapes of `spec` at the page centre on the active layer, grouped,
/// one undo step, then selected.
pub fn insert(app: &mut App, spec: &BarcodeSpec) -> Result<(), &'static str> {
    let (built, _) = barcode::build(spec)?;
    let Some(layer) = app.active_layer() else {
        return Ok(());
    };
    let c = app.page_rect().center();
    let ox = c.x - built.width / 2.0;
    let oy = c.y - spec.bar_height_mm / 2.0;
    let mut path = BezPath::new();
    for b in &built.bars {
        path.extend(tracedraw_core::geometry::rect_path(
            Rect::new(b.x0 + ox, b.y0 + oy, b.x1 + ox, b.y1 + oy),
            0.0,
        ));
    }
    let mut shapes = Vec::new();
    if let Some(bg) = built.background {
        let id = app.engine.new_shape_id();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(bg.x0 + ox, bg.y0 + oy, bg.x1 + ox, bg.y1 + oy),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::WHITE);
        s.stroke = None;
        shapes.push(s);
    }
    let id = app.engine.new_shape_id();
    let mut bars = Shape::new(id, ShapeKind::Path { path, closed: true });
    bars.fill = Fill::Solid(Color::BLACK);
    bars.stroke = None;
    bars.name = Some(spec.symbology.name().to_string());
    shapes.push(bars);
    for l in &built.labels {
        let id = app.engine.new_shape_id();
        let mut span = TextSpan::new(l.text.clone(), spec.font.clone(), spec.size_pt);
        span.bold = spec.bold;
        span.italic = spec.italic;
        let mut s = Shape::new(
            id,
            ShapeKind::Text {
                spans: vec![span],
                origin: Point::new(l.at.x + ox, l.at.y + oy),
                frame: None,
                align: match l.alignment {
                    TextAlignment::Left => TextAlign::Left,
                    TextAlignment::Center => TextAlign::Center,
                    TextAlignment::Right => TextAlign::Right,
                },
                para: Default::default(),
                on_path: None,
            },
        );
        s.fill = Fill::Solid(Color::BLACK);
        s.stroke = None;
        shapes.push(s);
    }
    let ids: Vec<_> = shapes.iter().map(|s| s.id).collect();
    let mut cmds: Vec<Command> = shapes
        .into_iter()
        .map(|shape| Command::AddShape { layer, shape })
        .collect();
    if ids.len() > 1 {
        cmds.push(Command::Group {
            shapes: ids.clone(),
        });
    }
    if let Err(e) = app.engine.run_batch("Insert Barcode", &cmds) {
        app.status = e.to_string();
        return Ok(());
    }
    let last = app
        .doc()
        .page(app.page)
        .ok()
        .and_then(|p| p.layers.iter().find(|l| l.id == layer))
        .and_then(|l| l.shapes.last())
        .map(|s| s.id);
    if let Some(id) = last {
        app.select(vec![id]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;

    fn frame(ctx: &Context, app: &mut App, enter: bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(UiRect::from_min_size(Pos2::ZERO, egui::vec2(1400.0, 900.0))),
            ..Default::default()
        };
        if enter {
            input.events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, &ui.ctx().clone()));
        out.textures_delta.clear();
    }

    fn state(app: &App) -> BarcodeState {
        match &app.dialog {
            Dialog::Barcode(s) => s.clone(),
            d => panic!("{d:?}"),
        }
    }

    #[test]
    fn the_wizard_walks_its_steps_and_inserts_a_group() {
        let mut app = App::headless();
        app.dialog = Dialog::Barcode(BarcodeState::default());
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        // Enter is Next on the first two steps, Finish on the last.
        frame(&ctx, &mut app, true);
        assert_eq!(state(&app).step, 1);
        frame(&ctx, &mut app, true);
        assert_eq!(state(&app).step, 2);
        frame(&ctx, &mut app, true);
        assert!(matches!(app.dialog, Dialog::None), "{:?}", app.dialog);
        let group = app
            .doc()
            .find_shape(app.selection[0])
            .expect("inserted")
            .clone();
        let ShapeKind::Group { children } = &group.kind else {
            panic!("{:?}", group.kind);
        };
        // Bars and the readable text.
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(
            |c| matches!(&c.kind, ShapeKind::Text { spans, .. } if spans[0].text == "1234567890123")
        ));
    }

    #[test]
    fn bad_data_stays_on_the_first_step_with_a_message() {
        let mut app = App::headless();
        let mut st = BarcodeState::default();
        st.spec.data = "12X".into();
        app.dialog = Dialog::Barcode(st);
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        let st = state(&app);
        assert_eq!(st.step, 0);
        assert!(!st.error.is_empty());
    }

    #[test]
    fn the_text_takes_the_font_weight_and_size() {
        let mut app = App::headless();
        let spec = BarcodeSpec {
            symbology: Symbology::Code39,
            data: "AB".into(),
            font: "DejaVu Sans".into(),
            size_pt: 9.0,
            bold: true,
            italic: true,
            quiet_zone: 10,
            ..Default::default()
        };
        insert(&mut app, &spec).expect("inserted");
        let group = app
            .doc()
            .find_shape(app.selection[0])
            .expect("group")
            .clone();
        let ShapeKind::Group { children } = &group.kind else {
            panic!();
        };
        // Background, bars, text.
        assert_eq!(children.len(), 3);
        let span = children
            .iter()
            .find_map(|c| match &c.kind {
                ShapeKind::Text { spans, .. } => Some(spans[0].clone()),
                _ => None,
            })
            .expect("text");
        assert_eq!(
            (
                span.font_family.as_str(),
                span.size_pt,
                span.bold,
                span.italic
            ),
            ("DejaVu Sans", 9.0, true, true)
        );
        assert_eq!(span.text, "*AB*");
    }
}
