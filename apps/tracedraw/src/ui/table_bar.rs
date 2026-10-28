//! The Table tool's property bar (rows and columns of new tables, the
//! selected cells' size, background, borders, margins, separated borders
//! and text) and the dialog asking how many rows or columns to insert or
//! split into.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::{ok_cancel, window};
use crate::ui::tips::RichTip;
use egui::{Context, Ui};
use tracedraw_core::{document::ShapeKind, BorderSet, CellMargins, Color, Command, Fill, Stroke};

fn dim(ui: &mut Ui, key: &str) {
    ui.label(
        egui::RichText::new(tr(key))
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

fn vsep(ui: &mut Ui) {
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}

/// A length field in the document's units; returns the new millimetres
/// when it changed.
fn length(ui: &mut Ui, app: &App, mm: f64, min_mm: f64) -> Option<f64> {
    let units = app.units;
    let mut v = units.from_mm(mm);
    let r = ui.add(
        crate::ui::field::NumField::new(&mut v)
            .range(units.from_mm(min_mm)..=units.from_mm(5000.0))
            .fixed_decimals(app.settings.precision.min(10) as usize)
            .suffix(format!(" {}", units.short()))
            .width(64.0),
    );
    r.changed().then(|| units.to_mm(v))
}

/// The Table tool's bar.
pub fn table_bar(app: &mut App, ui: &mut Ui) {
    crate::ui::toolbar::dim_labelled_as(ui, "toolbar.rows", "table.rows_desc", |ui| {
        ui.add(crate::ui::field::NumField::new(&mut app.table_rows).range(1..=100))
    });
    crate::ui::toolbar::dim_labelled_as(ui, "toolbar.columns", "table.columns_desc", |ui| {
        ui.add(crate::ui::field::NumField::new(&mut app.table_cols).range(1..=100))
    });
    vsep(ui);
    let target = app.table_target();
    let cells = app.table_edit.is_some() && target.is_some();
    if cells {
        cell_size(app, ui);
        vsep(ui);
    }
    background(app, ui, cells);
    vsep(ui);
    borders(app, ui, cells);
    if target.is_some() {
        vsep(ui);
        ui.menu_button(tr("table.options"), |ui| table_options(app, ui))
            .response
            .rich_tip(crate::ui::tips::keyed("table.options"));
    }
    if cells {
        vsep(ui);
        cell_text(app, ui);
    }
    vsep(ui);
    dim(ui, "toolbar.drag_to_draw_table");
}

/// Width and height of the selected cells.
fn cell_size(app: &mut App, ui: &mut Ui) {
    let Some((w, h)) = app.table_cell_size() else {
        return;
    };
    let tip = crate::ui::tips::pair("table.cell_width_tip", "table.cell_width_desc");
    if let Some(v) = crate::ui::tips::tip_over(ui, tip, |ui| {
        dim(ui, "table.cell_width");
        length(ui, app, w, 1.0)
    }) {
        app.table_set_cell_size(Some(v), None);
    }
    let tip = crate::ui::tips::pair("table.cell_height_tip", "table.cell_height_desc");
    if let Some(v) = crate::ui::tips::tip_over(ui, tip, |ui| {
        dim(ui, "table.cell_height");
        length(ui, app, h, 1.0)
    }) {
        app.table_set_cell_size(None, Some(v));
    }
}

/// The background: of the selected cells, else of the table's cells, else
/// of the next table.
fn background(app: &mut App, ui: &mut Ui, cells: bool) {
    let selected = app.selected_table();
    let fill = if cells {
        app.table_active_fill().unwrap_or(Fill::None)
    } else {
        selected
            .as_ref()
            .map(|(_, t)| t.cell_fill.clone())
            .unwrap_or_else(|| app.table_fill.clone())
    };
    dim(
        ui,
        if cells {
            "table.cell_background"
        } else {
            "toolbar.attr_fill"
        },
    );
    let mut new = None;
    let mut has_fill = !matches!(fill, Fill::None);
    if ui
        .checkbox(&mut has_fill, "")
        .rich_tip(crate::ui::tips::keyed("table.has_background"))
        .changed()
    {
        new = Some(if has_fill {
            Fill::Solid(Color::WHITE)
        } else {
            Fill::None
        });
    }
    if let Fill::Solid(c) = &fill {
        let mut c = *c;
        if crate::ui::color_picker::color_button(ui, &mut c) {
            new = Some(Fill::Solid(c));
        }
    }
    let Some(f) = new else {
        return;
    };
    if cells {
        app.table_set_cell_fill(Some(f));
        return;
    }
    match selected {
        Some((id, mut t)) => {
            t.cell_fill = f;
            app.run(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Table(t),
            });
        }
        None => app.table_fill = f,
    }
}

/// Which borders, their width, colour and style.
fn borders(app: &mut App, ui: &mut Ui, cells: bool) {
    let selected = app.selected_table();
    dim(ui, "toolbar.border");
    if cells || selected.is_some() {
        let current = app.table_ui.border_set;
        egui::ComboBox::from_id_salt("table_border_set")
            .width(130.0)
            .selected_text(tr(current.key()))
            .show_ui(ui, |ui| {
                for s in BorderSet::ALL {
                    if ui.selectable_label(s == current, tr(s.key())).clicked() {
                        app.table_ui.border_set = s;
                    }
                }
            })
            .response
            .rich_tip(tr("table.border_choice_tip"));
    }
    let shown = if cells || selected.is_some() {
        app.table_shown_border()
    } else {
        app.table_border.clone()
    };
    let mut border = shown.clone();
    let mut width = border.as_ref().map(|b| b.width).unwrap_or(0.0);
    let r = ui.add(
        crate::ui::field::NumField::new(&mut width)
            .range(0.0..=20.0)
            .speed(0.05)
            .fixed_decimals(3)
            .suffix(" mm"),
    );
    let r = r.rich_tip(crate::ui::tips::keyed("table.border_width"));
    if r.changed() {
        border = if width <= 0.0 {
            None
        } else {
            let mut b = border
                .clone()
                .unwrap_or_else(|| Stroke::hairline(Color::BLACK));
            b.width = width;
            Some(b)
        };
    }
    if let Some(b) = &mut border {
        crate::ui::color_picker::color_button(ui, &mut b.color);
        let idx = crate::table_tool::BORDER_DASHES
            .iter()
            .position(|(_, d)| *d == b.dash.as_slice())
            .unwrap_or(0);
        egui::ComboBox::from_id_salt("table_border_style")
            .width(90.0)
            .selected_text(tr(crate::table_tool::BORDER_DASHES[idx].0))
            .show_ui(ui, |ui| {
                for (i, (key, dash)) in crate::table_tool::BORDER_DASHES.iter().enumerate() {
                    if ui.selectable_label(i == idx, tr(key)).clicked() {
                        b.dash = dash.to_vec();
                    }
                }
            })
            .response
            .rich_tip(crate::ui::tips::keyed("table.border_style"));
    }
    if border == shown {
        return;
    }
    if cells || selected.is_some() {
        app.table_apply_borders(border);
    } else {
        app.table_border = border;
    }
}

/// Margins (one value for every side, or one per side) and separated
/// borders with their spacing.
fn table_options(app: &mut App, ui: &mut Ui) {
    let Some((_, _, t)) = app.table_target() else {
        return;
    };
    ui.label(egui::RichText::new(tr("table.margins")).strong());
    let mut m = t.cell_margins();
    let mut separate = app.table_ui.margins_separate;
    if ui
        .checkbox(&mut separate, tr("table.margins_per_side"))
        .changed()
    {
        app.table_ui.margins_separate = separate;
    }
    let mut changed = false;
    egui::Grid::new("table_margins")
        .num_columns(2)
        .show(ui, |ui| {
            if separate {
                for (key, v) in [
                    ("table.margin_top", &mut m.top),
                    ("table.margin_bottom", &mut m.bottom),
                    ("table.margin_left", &mut m.left),
                    ("table.margin_right", &mut m.right),
                ] {
                    ui.label(tr(key));
                    if let Some(x) = length(ui, app, *v, 0.0) {
                        *v = x;
                        changed = true;
                    }
                    ui.end_row();
                }
            } else {
                ui.label(tr("table.margin_all"));
                if let Some(x) = length(ui, app, m.top, 0.0) {
                    m = CellMargins::uniform(x);
                    changed = true;
                }
                ui.end_row();
            }
        });
    if changed {
        app.table_set_margins(m);
    }
    ui.separator();
    let mut on = t.spacing.is_some();
    if ui.checkbox(&mut on, tr("table.separated")).changed() {
        app.table_set_spacing(on.then_some([1.0, 1.0]));
    }
    if let Some([h, v]) = t.spacing {
        let mut sp = [h, v];
        let mut changed = false;
        egui::Grid::new("table_spacing")
            .num_columns(2)
            .show(ui, |ui| {
                for (key, i) in [("table.spacing_h", 0usize), ("table.spacing_v", 1)] {
                    ui.label(tr(key));
                    if let Some(x) = length(ui, app, sp[i], 0.0) {
                        sp[i] = x;
                        changed = true;
                    }
                    ui.end_row();
                }
            });
        if changed {
            app.table_set_spacing(Some(sp));
        }
    }
}

/// Font, size, bold, italic, underline and alignment of the selected
/// cells' text.
fn cell_text(app: &mut App, ui: &mut Ui) {
    let mut changed = crate::ui::font_list::text_font_list(app, ui, "table_font", 140.0);
    changed |= crate::ui::toolbar::text_size_list(app, ui);
    for (key, label) in [
        ("toolbar.bold", egui::RichText::new("B").strong()),
        ("toolbar.italic", egui::RichText::new("I").italics()),
        ("toolbar.underline", egui::RichText::new("U").underline()),
    ] {
        let on = match key {
            "toolbar.bold" => app.text_bold,
            "toolbar.italic" => app.text_italic,
            _ => app.text_underline,
        };
        if ui
            .add_sized([22.0, 22.0], egui::Button::selectable(on, label.size(14.0)))
            .rich_tip(tr(key))
            .clicked()
        {
            match key {
                "toolbar.bold" => app.text_bold = !on,
                "toolbar.italic" => app.text_italic = !on,
                _ => app.text_underline = !on,
            }
            changed = true;
        }
    }
    changed |= crate::ui::toolbar::text_align_list(app, ui);
    if changed {
        app.apply_text_style();
    }
}

/// What the count dialog does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableCountKind {
    InsertRows,
    InsertCols,
    SplitRows,
    SplitCols,
}

/// The count dialog: how many, and for inserts on which side.
#[derive(Debug, Clone, PartialEq)]
pub struct TableCountState {
    pub kind: TableCountKind,
    pub count: u32,
    /// Below (rows) or right (columns) of the selection.
    pub after: bool,
}

impl App {
    /// Open the count dialog; inserts start from the number of rows or
    /// columns the selection covers, splits from two.
    pub fn open_table_count(&mut self, kind: TableCountKind) {
        let (rows, cols) = self.table_selection_extent();
        let count = match kind {
            TableCountKind::InsertRows => rows,
            TableCountKind::InsertCols => cols,
            TableCountKind::SplitRows | TableCountKind::SplitCols => 2,
        };
        self.dialog = crate::ui::dialogs::Dialog::TableCount(TableCountState {
            kind,
            count,
            after: false,
        });
    }
}

pub fn count_dialog(app: &mut App, ctx: &Context, st: &mut TableCountState, close: &mut bool) {
    let (title, label, sides) = match st.kind {
        TableCountKind::InsertRows => (
            "table.insert_rows_title",
            "dialog.table_rows",
            Some(("table.above", "table.below")),
        ),
        TableCountKind::InsertCols => (
            "table.insert_cols_title",
            "dialog.table_columns",
            Some(("table.left_of", "table.right_of")),
        ),
        TableCountKind::SplitRows => ("table.split_rows_title", "dialog.table_rows", None),
        TableCountKind::SplitCols => ("table.split_cols_title", "dialog.table_columns", None),
    };
    let min = if sides.is_some() { 1 } else { 2 };
    window(ctx, tr(title)).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr(label));
            ui.add_sized(
                [90.0, 22.0],
                crate::ui::field::NumField::new(&mut st.count).range(min..=100),
            )
            .rich_tip(crate::ui::tips::pair(label, &format!("{title}_count_desc")));
        });
        if let Some((before, after)) = sides {
            ui.add_space(4.0);
            ui.radio_value(&mut st.after, false, tr(before))
                .rich_tip(crate::ui::tips::keyed(before));
            ui.radio_value(&mut st.after, true, tr(after))
                .rich_tip(crate::ui::tips::keyed(after));
        }
        ui.add_space(8.0);
        if ok_cancel(ui, close) {
            let n = st.count.clamp(min, 100);
            match st.kind {
                TableCountKind::InsertRows => app.table_insert_rows(n, st.after),
                TableCountKind::InsertCols => app.table_insert_cols(n, st.after),
                TableCountKind::SplitRows => app.table_split(true, n),
                TableCountKind::SplitCols => app.table_split(false, n),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;
    use tracedraw_core::{geometry::Rect, Table};

    fn frame(ctx: &Context, app: &mut App, enter: bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
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

    fn table(app: &App) -> Table {
        app.selected_table().unwrap().1
    }

    #[test]
    fn the_count_dialog_inserts_and_splits() {
        let mut app = App::headless();
        app.tool = crate::tools::Tool::Table;
        let t = Table::new(Rect::new(0.0, 0.0, 40.0, 20.0), 2, 2, None);
        let id = app.new_shape(ShapeKind::Table(t)).unwrap();
        app.select(vec![id]);
        app.table_click(id, tracedraw_core::Point::new(5.0, 15.0), false);
        let ctx = crate::theme::ui_context();
        app.open_table_count(TableCountKind::InsertRows);
        if let Dialog::TableCount(st) = &mut app.dialog {
            assert_eq!(st.count, 1);
            st.count = 3;
            st.after = true;
        }
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        assert!(matches!(app.dialog, Dialog::None));
        assert_eq!(table(&app).rows(), 5);
        app.open_table_count(TableCountKind::SplitCols);
        if let Dialog::TableCount(st) = &mut app.dialog {
            st.count = 4;
        }
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        assert_eq!(table(&app).cols(), 5);
    }

    #[test]
    fn the_bar_draws_with_and_without_cells() {
        let mut app = App::headless();
        app.tool = crate::tools::Tool::Table;
        let ctx = crate::theme::ui_context();
        let draw = |app: &mut App| {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.horizontal(|ui| table_bar(app, ui));
            });
            out.textures_delta.clear();
        };
        draw(&mut app);
        let t = Table::new(Rect::new(0.0, 0.0, 40.0, 20.0), 2, 2, None);
        let id = app.new_shape(ShapeKind::Table(t)).unwrap();
        app.select(vec![id]);
        draw(&mut app);
        app.table_click(id, tracedraw_core::Point::new(5.0, 15.0), false);
        draw(&mut app);
    }
}
