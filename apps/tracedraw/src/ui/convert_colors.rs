//! Tools > Convert Colors (#972): every colour of the selected objects or
//! of the whole drawing to CMYK or to RGB, through the colour management
//! profiles, in one undo step (`Command::ConvertColors`). Spot colours
//! stay unless asked. See docs/behavior/colour-management.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::ui::dialogs::window;
use egui::{Context, RichText};
use tracedraw_core::color_convert::{convert_document, ColorTarget};
use tracedraw_core::Command;

/// The dialog's choices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConvertColorsState {
    pub to: ColorTarget,
    /// The whole drawing rather than the selected objects.
    pub whole: bool,
    /// Spot colours become process colours too.
    pub spots: bool,
}

impl ConvertColorsState {
    /// To CMYK; the selection when there is one, else the whole drawing.
    pub fn new(app: &App) -> Self {
        ConvertColorsState {
            to: ColorTarget::Cmyk,
            whole: app.selection.is_empty(),
            spots: false,
        }
    }
}

impl App {
    /// Tools > Convert Colors: its dialog.
    pub fn convert_colors_dialog(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::ConvertColors(ConvertColorsState::new(self));
    }

    /// Convert the colours `st` asks for: one undo step, the status line
    /// says how many changed. False when nothing changed.
    pub fn convert_colors(&mut self, st: &ConvertColorsState) -> bool {
        let shapes = if st.whole {
            Vec::new()
        } else if self.selection.is_empty() {
            self.status = tr("convert_colors.nothing_selected");
            return false;
        } else {
            self.selection.clone()
        };
        // How many colours change, on a copy (the drawing's data is shared).
        let mut copy = self.doc().clone();
        let n = convert_document(&mut copy, &shapes, st.to, st.spots);
        let model = tr(match st.to {
            ColorTarget::Cmyk => "convert_colors.cmyk",
            ColorTarget::Rgb => "convert_colors.rgb",
        });
        if n == 0 {
            self.status = trf("convert_colors.none", &[("model", &model)]);
            return false;
        }
        self.run(Command::ConvertColors {
            shapes,
            to: st.to,
            spots: st.spots,
        });
        self.status = trf(
            "convert_colors.done",
            &[("n", &n.to_string()), ("model", &model)],
        );
        true
    }
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut ConvertColorsState, close: &mut bool) {
    let mut help = false;
    let mut ok = false;
    let has_selection = !app.selection.is_empty();
    if !has_selection {
        st.whole = true;
    }
    window(ctx, tr("convert_colors.title")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.strong(tr("convert_colors.to"));
        ui.radio_value(&mut st.to, ColorTarget::Cmyk, tr("convert_colors.cmyk"));
        ui.radio_value(&mut st.to, ColorTarget::Rgb, tr("convert_colors.rgb"));
        ui.add_space(6.0);
        ui.strong(tr("convert_colors.which"));
        ui.add_enabled_ui(has_selection, |ui| {
            ui.radio_value(&mut st.whole, false, tr("convert_colors.selection"));
        });
        ui.radio_value(&mut st.whole, true, tr("convert_colors.drawing"));
        ui.add_space(6.0);
        ui.checkbox(&mut st.spots, tr("convert_colors.spots"));
        ui.add_space(4.0);
        ui.label(RichText::new(tr("convert_colors.note")).weak());
        let (o, h) = crate::ui::dialogs::help_ok_cancel(ui, close);
        ok = o;
        help = h;
    });
    if ok {
        app.convert_colors(st);
    }
    if help {
        app.open_dialog_help();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;
    use tracedraw_core::{Color, Fill, ShapeKind};

    fn rect(app: &mut App, c: Color) -> tracedraw_core::ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(c),
        });
        app.run(Command::SetStroke {
            shapes: vec![id],
            stroke: None,
        });
        id
    }

    fn fill_of(app: &App, id: tracedraw_core::ShapeId) -> Color {
        match app.doc().find_shape(id).map(|s| s.fill.clone()) {
            Some(Fill::Solid(c)) => c,
            _ => panic!("solid"),
        }
    }

    /// The dialog converts the selection or the whole drawing, in one undo
    /// step each, and says how many colours changed; OK by Enter.
    #[test]
    fn colours_convert_for_the_selection_or_the_drawing() {
        let mut app = App::headless();
        app.show_welcome = false;
        let a = rect(&mut app, Color::rgb8(200, 30, 30));
        let b = rect(&mut app, Color::rgb8(30, 30, 200));
        app.select(vec![a]);
        app.convert_colors_dialog();
        let Dialog::ConvertColors(st) = &app.dialog else {
            panic!("open");
        };
        assert!(!st.whole && st.to == ColorTarget::Cmyk);
        // Drawn, then OK by Enter: the selected rectangle only.
        let ctx = crate::theme::ui_context();
        for events in [
            Vec::new(),
            Vec::new(),
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        ] {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(&mut app, ui.ctx()));
            out.textures_delta.clear();
        }
        assert!(matches!(app.dialog, Dialog::None));
        assert_eq!(
            fill_of(&app, a),
            Color::rgb8(200, 30, 30).convert_to("CMYK"),
            "through the colour engine"
        );
        assert_eq!(fill_of(&app, b).model_name(), "RGB");
        assert!(app.status.contains('1'), "{}", app.status);
        assert_eq!(app.engine.undo_label(), Some("Convert Colors"));
        // The whole drawing back to RGB: one step.
        let before = app.engine.history_labels().0.len();
        let st = ConvertColorsState {
            to: ColorTarget::Rgb,
            whole: true,
            spots: false,
        };
        assert!(app.convert_colors(&st));
        assert_eq!(app.engine.history_labels().0.len(), before + 1);
        assert_eq!(fill_of(&app, a).model_name(), "RGB");
        // Nothing left to change: no step.
        assert!(!app.convert_colors(&st));
        assert_eq!(app.engine.history_labels().0.len(), before + 1);
        // The selection asked for with nothing selected.
        app.select(Vec::new());
        assert!(!app.convert_colors(&ConvertColorsState { whole: false, ..st }));
        app.undo();
        assert_eq!(fill_of(&app, a).model_name(), "CMYK");
    }
}
