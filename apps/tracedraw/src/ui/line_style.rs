//! Edit Line Style: make and change custom dash patterns on a grid of
//! cells, one cell per outline width. The patterns are kept in the
//! settings (`Settings::custom_dashes`) and listed after the built-in
//! dash styles in the Properties docker and on the property bar. See
//! docs/behavior/outline-pen.md.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::window;
use egui::{Context, Sense, Ui};

/// The most cells a pattern has.
pub const MAX_CELLS: usize = 32;
/// The most custom patterns kept.
pub const MAX_PATTERNS: usize = 64;

/// What the outline editors offer beyond the presets: custom arrowheads
/// and custom dash patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineLib {
    pub arrows: Vec<tracedraw_core::Arrowhead>,
    pub dashes: Vec<Vec<f64>>,
    /// The unit outline widths are shown in.
    pub units: crate::app::Units,
    /// The colour an outline turned on gets: black in the drawing's
    /// primary colour mode.
    pub black: tracedraw_core::Color,
}

impl Default for OutlineLib {
    fn default() -> Self {
        OutlineLib {
            arrows: Vec::new(),
            dashes: Vec::new(),
            units: Default::default(),
            black: tracedraw_core::document::PrimaryColorMode::default().black(),
        }
    }
}

impl OutlineLib {
    pub fn of(app: &App) -> Self {
        OutlineLib {
            arrows: app.settings.custom_arrowheads.clone(),
            dashes: app.settings.custom_dashes.clone(),
            units: app.outline_units(),
            black: app.doc().metadata.primary_color_mode.black(),
        }
    }
}

fn units_request_id() -> egui::Id {
    egui::Id::new("outline_units_choice")
}

/// Ask for another outline unit from inside a docker (applied on the
/// next pass through `dialogs::show`).
pub fn request_units(ui: &Ui, u: crate::app::Units) {
    ui.data_mut(|d| d.insert_temp(units_request_id(), u.id().to_string()));
}

/// Apply an outline unit asked for with [`request_units`].
pub fn apply_units_request(app: &mut App, ctx: &Context) {
    if let Some(id) = ctx.data_mut(|d| d.remove_temp::<String>(units_request_id())) {
        app.set_outline_units(crate::app::Units::from_id(&id));
    }
}

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// On (dash) or off (gap) for each cell of the pattern.
    pub cells: Vec<bool>,
    /// The custom pattern being changed, by index.
    pub editing: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        State {
            cells: cells_from_dash(&[4.0, 2.0], 12),
            editing: None,
        }
    }
}

fn request_id() -> egui::Id {
    egui::Id::new("open_line_style_editor")
}

/// Ask for the dialog from inside a docker or bar (it opens on the next
/// pass through `dialogs::show`).
pub fn request(ui: &Ui) {
    ui.data_mut(|d| d.insert_temp(request_id(), true));
}

/// Whether the dialog was asked for since the last call.
pub fn take_request(ctx: &Context) -> bool {
    ctx.data_mut(|d| d.remove_temp::<bool>(request_id()))
        .unwrap_or(false)
}

/// The dash pattern (on, off, ... lengths in outline widths) the cells
/// draw, starting at the first dash after a gap. Empty when the cells
/// have no dash or no gap (no pattern to repeat).
pub fn dash_from_cells(cells: &[bool]) -> Vec<f64> {
    let n = cells.len();
    if n == 0 || cells.iter().all(|c| *c) || cells.iter().all(|c| !*c) {
        return Vec::new();
    }
    // Start at a dash that follows a gap, so the pattern wraps cleanly.
    let start = (0..n)
        .find(|&i| cells[i] && !cells[(i + n - 1) % n])
        .unwrap_or(0);
    let mut out: Vec<f64> = Vec::new();
    let mut run = 0.0;
    let mut state = true;
    for k in 0..n {
        let c = cells[(start + k) % n];
        if c == state {
            run += 1.0;
        } else {
            out.push(run);
            run = 1.0;
            state = c;
        }
    }
    out.push(run);
    out
}

/// Cells drawing `dash` (each entry rounded to at least one cell), at
/// most `max` of them; an empty pattern gives one dash and one gap.
pub fn cells_from_dash(dash: &[f64], max: usize) -> Vec<bool> {
    let max = max.clamp(2, MAX_CELLS);
    let mut cells = Vec::new();
    for (i, d) in dash.iter().enumerate() {
        let k = if d.is_finite() {
            d.round().clamp(1.0, max as f64) as usize
        } else {
            1
        };
        for _ in 0..k {
            if cells.len() < max {
                cells.push(i % 2 == 0);
            }
        }
    }
    if dash_from_cells(&cells).is_empty() {
        cells = vec![true, false];
    }
    cells
}

/// A short drawing of a pattern, for lists.
pub fn preview(ui: &mut Ui, dash: &[f64], width: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 10.0), Sense::click());
    let y = rect.center().y;
    let unit = 2.0f32;
    let color = Tokens::text();
    if dash.is_empty() {
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(unit, color),
        );
        return resp;
    }
    let mut x = rect.left();
    let mut i = 0usize;
    while x < rect.right() && i < 400 {
        let len = (dash[i % dash.len()].max(0.0) as f32 * unit).max(0.5);
        if i.is_multiple_of(2) {
            let x1 = (x + len).min(rect.right());
            ui.painter().line_segment(
                [egui::pos2(x, y), egui::pos2(x1, y)],
                egui::Stroke::new(unit, color),
            );
        }
        x += len;
        i += 1;
    }
    resp
}

/// Edit Line Style.
pub fn dialog(app: &mut App, ctx: &Context, st: &mut State, close: &mut bool) {
    window(ctx, tr("line_style.title")).show(ctx, |ui| {
        ui.set_max_width(460.0);
        ui.label(tr("line_style.help"));
        ui.add_space(6.0);
        // The grid: click a cell to switch it between dash and gap.
        let cell = 12.0f32;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            for i in 0..st.cells.len() {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(cell, cell), Sense::click());
                let fill = if st.cells[i] {
                    Tokens::text()
                } else {
                    Tokens::field()
                };
                ui.painter().rect_filled(rect, 0.0, fill);
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.0, Tokens::border()),
                    egui::StrokeKind::Inside,
                );
                if resp.clicked() {
                    st.cells[i] = !st.cells[i];
                }
            }
        });
        let mut n = st.cells.len();
        ui.horizontal(|ui| {
            ui.label(tr("line_style.length"));
            if ui.add(egui::Slider::new(&mut n, 2..=MAX_CELLS)).changed() {
                st.cells.resize(n, false);
            }
        });
        let dash = dash_from_cells(&st.cells);
        ui.add_space(4.0);
        preview(ui, &dash, 300.0);
        if dash.is_empty() {
            ui.label(
                egui::RichText::new(tr("line_style.needs_dash_and_gap"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
        }
        ui.separator();
        ui.label(tr("line_style.saved"));
        let mut pick = None;
        egui::ScrollArea::vertical()
            .max_height(140.0)
            .show(ui, |ui| {
                for (i, d) in app.settings.custom_dashes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let sel = st.editing == Some(i);
                        if ui.selectable_label(sel, format!("{}", i + 1)).clicked() {
                            pick = Some(i);
                        }
                        if preview(ui, d, 200.0).clicked() {
                            pick = Some(i);
                        }
                    });
                }
            });
        if let Some(i) = pick {
            if let Some(d) = app.settings.custom_dashes.get(i) {
                st.cells = cells_from_dash(d, MAX_CELLS);
                st.editing = Some(i);
            }
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let full = app.settings.custom_dashes.len() >= MAX_PATTERNS;
            if ui
                .add_enabled(
                    !dash.is_empty() && !full,
                    egui::Button::new(tr("line_style.add")),
                )
                .clicked()
            {
                app.add_custom_dash(dash.clone());
                st.editing = Some(app.settings.custom_dashes.len().saturating_sub(1));
            }
            if ui
                .add_enabled(
                    !dash.is_empty() && st.editing.is_some(),
                    egui::Button::new(tr("line_style.replace")),
                )
                .clicked()
            {
                if let Some(i) = st.editing {
                    app.replace_custom_dash(i, dash.clone());
                }
            }
            if ui
                .add_enabled(
                    st.editing.is_some(),
                    egui::Button::new(tr("line_style.delete")),
                )
                .clicked()
            {
                if let Some(i) = st.editing.take() {
                    app.delete_custom_dash(i);
                }
            }
            if ui.button(tr("dialog.close")).clicked() {
                *close = true;
            }
        });
    });
}

impl App {
    /// The unit outline widths are shown and typed in (points unless
    /// chosen otherwise).
    pub fn outline_units(&self) -> crate::app::Units {
        crate::app::Units::from_id(&self.settings.outline_units)
    }

    /// Show outline widths in `u` from now on (kept in the settings).
    pub fn set_outline_units(&mut self, u: crate::app::Units) {
        if self.settings.outline_units != u.id() {
            self.settings.outline_units = u.id().to_string();
            self.save_settings();
        }
    }

    /// Keep a new custom dash pattern (Edit Line Style's Add).
    pub fn add_custom_dash(&mut self, dash: Vec<f64>) {
        if dash.is_empty() || self.settings.custom_dashes.len() >= MAX_PATTERNS {
            return;
        }
        if !self.settings.custom_dashes.contains(&dash) {
            self.settings.custom_dashes.push(dash);
            self.save_settings();
        }
    }

    /// Change custom dash pattern `i`.
    pub fn replace_custom_dash(&mut self, i: usize, dash: Vec<f64>) {
        if dash.is_empty() {
            return;
        }
        if let Some(d) = self.settings.custom_dashes.get_mut(i) {
            *d = dash;
            self.save_settings();
        }
    }

    /// Forget custom dash pattern `i`; outlines that use it keep it.
    pub fn delete_custom_dash(&mut self, i: usize) {
        if i < self.settings.custom_dashes.len() {
            self.settings.custom_dashes.remove(i);
            self.save_settings();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_and_patterns_convert_both_ways() {
        let cells = [true, true, true, false, true, false, false];
        assert_eq!(dash_from_cells(&cells), vec![3.0, 1.0, 1.0, 2.0]);
        // A pattern starting with a gap wraps to start at a dash.
        let cells = [false, true, true, false];
        assert_eq!(dash_from_cells(&cells), vec![2.0, 2.0]);
        assert!(dash_from_cells(&[true, true]).is_empty());
        assert!(dash_from_cells(&[false, false]).is_empty());
        assert!(dash_from_cells(&[]).is_empty());
        for d in [vec![4.0, 2.0], vec![6.0, 2.0, 1.0, 2.0], vec![1.0, 1.0]] {
            assert_eq!(dash_from_cells(&cells_from_dash(&d, MAX_CELLS)), d);
        }
        // Odd values are rounded and capped, nothing is lost to a panic.
        let c = cells_from_dash(&[f64::NAN, 0.2, 100.0], 8);
        assert!(c.len() <= 8);
        assert!(!dash_from_cells(&cells_from_dash(&[], 8)).is_empty());
    }

    #[test]
    fn custom_patterns_are_added_replaced_and_deleted_in_the_settings() {
        let mut app = App::headless();
        app.settings.custom_dashes.clear();
        app.add_custom_dash(vec![3.0, 1.0]);
        app.add_custom_dash(vec![3.0, 1.0]);
        app.add_custom_dash(Vec::new());
        assert_eq!(app.settings.custom_dashes, vec![vec![3.0, 1.0]]);
        app.replace_custom_dash(0, vec![5.0, 1.0]);
        app.replace_custom_dash(7, vec![5.0, 1.0]);
        assert_eq!(app.settings.custom_dashes, vec![vec![5.0, 1.0]]);
        assert_eq!(OutlineLib::of(&app).dashes, vec![vec![5.0, 1.0]]);
        app.delete_custom_dash(3);
        app.delete_custom_dash(0);
        assert!(app.settings.custom_dashes.is_empty());
    }

    #[test]
    fn a_request_opens_the_dialog_and_it_draws() {
        let mut app = App::headless();
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..Default::default()
        };
        ctx.data_mut(|d| d.insert_temp(request_id(), true));
        ctx.data_mut(|d| d.insert_temp(units_request_id(), "mm".to_string()));
        for _ in 0..2 {
            let mut out = ctx.run_ui(input.clone(), |ui| {
                crate::ui::dialogs::show(&mut app, ui.ctx())
            });
            out.textures_delta.clear();
        }
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::LineStyle(_)
        ));
        assert_eq!(app.outline_units(), crate::app::Units::Millimeters);
    }
}
