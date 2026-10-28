//! The form layout of the effect dockers: sections with a bold title and
//! a line above, rows with the label right-aligned in a column of its own
//! and the field after it, and a full-width button at the end.

use egui::{Align, Layout, RichText, Ui, Vec2};

/// Width of the label column.
pub const LABEL_W: f32 = 104.0;
/// Height of a form row.
pub const ROW_H: f32 = 24.0;

/// A line across the docker, then the section's bold title (when it has
/// one).
pub fn section(ui: &mut Ui, title: Option<&str>) {
    ui.add_space(4.0);
    ui.separator();
    if let Some(t) = title {
        ui.add_space(2.0);
        ui.label(RichText::new(t).strong());
    }
    ui.add_space(2.0);
}

/// A row: `label` right-aligned in the label column, then `add`.
pub fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(LABEL_W, ROW_H),
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.set_min_width(LABEL_W);
                ui.label(label);
            },
        );
        add(ui)
    })
    .inner
}

/// A row whose label column is empty (a check box under the fields).
pub fn indented<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_space(LABEL_W + ui.spacing().item_spacing.x);
        add(ui)
    })
    .inner
}

/// The docker's full-width button (Apply) with its tip; true when clicked.
pub fn wide_button(ui: &mut Ui, text: &str, enabled: bool, tip: &str) -> bool {
    use crate::ui::tips::RichTip;
    ui.add_space(6.0);
    let w = ui.available_width();
    ui.add_enabled(
        enabled,
        egui::Button::new(text).min_size(Vec2::new(w, 26.0)),
    )
    .rich_tip(tip)
    .rich_disabled_tip(tip)
    .clicked()
}
