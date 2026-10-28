//! The connector tools' property bar (also shown when connectors are
//! selected with the Pick tool): how the connector runs (Straight, Right
//! angle, Rounded), the rounded corners' radius, then the outline (width,
//! style, arrowheads) of the selected connectors.

use crate::app::App;
use crate::connectors::ConnectorKind;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::field::NumField;
use crate::ui::tips::RichTip;
use egui::Ui;

fn tool_of(kind: ConnectorKind) -> Tool {
    match kind {
        ConnectorKind::Straight => Tool::Connector,
        ConnectorKind::RightAngle => Tool::RightAngleConnector,
        ConnectorKind::Rounded => Tool::RoundedConnector,
    }
}

fn kind_key(kind: ConnectorKind) -> &'static str {
    match kind {
        ConnectorKind::Straight => "connector.straight",
        ConnectorKind::RightAngle => "connector.right_angle",
        ConnectorKind::Rounded => "connector.rounded",
    }
}

/// The bar.
pub fn connector_bar(app: &mut App, ui: &mut Ui) {
    let selected = app.selected_connectors();
    // The kind shown: the first selected connector's, else the tool's.
    let shown = selected
        .first()
        .map(|(_, c)| c.kind)
        .unwrap_or(match app.tool {
            Tool::RightAngleConnector => ConnectorKind::RightAngle,
            Tool::RoundedConnector => ConnectorKind::Rounded,
            _ => ConnectorKind::Straight,
        });
    for kind in [
        ConnectorKind::Straight,
        ConnectorKind::RightAngle,
        ConnectorKind::Rounded,
    ] {
        if ui
            .selectable_label(shown == kind, tr(kind_key(kind)))
            .rich_tip(crate::ui::tips::keyed(kind_key(kind)))
            .clicked()
        {
            if selected.is_empty() {
                app.set_tool(tool_of(kind));
            } else {
                app.set_connectors(Some(kind), None);
            }
        }
    }
    if shown == ConnectorKind::Rounded {
        let mut r = selected
            .first()
            .map(|(_, c)| c.radius)
            .unwrap_or(app.connector_radius);
        let resp = crate::ui::toolbar::dim_labelled(ui, "connector.radius", |ui| {
            ui.add(
                NumField::new(&mut r)
                    .speed(0.1)
                    .range(0.0..=1000.0)
                    .suffix(" mm"),
            )
        });
        if resp.changed() {
            if selected.is_empty() {
                app.connector_radius = r.clamp(0.0, 1000.0);
            } else {
                app.set_connectors(None, Some(r));
            }
        }
    }
    if !selected.is_empty() {
        crate::ui::propbar::sep(ui);
        crate::ui::propbar::outline_part(app, ui);
    }
    crate::ui::propbar::sep(ui);
    ui.label(
        egui::RichText::new(tr("toolbar.drag_between_objects"))
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_draws_for_tools_and_selections() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for t in [
            Tool::Connector,
            Tool::RightAngleConnector,
            Tool::RoundedConnector,
        ] {
            app.tool = t;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 80.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| connector_bar(&mut app, ui));
            });
            out.textures_delta.clear();
        }
        assert_eq!(tool_of(ConnectorKind::Rounded), Tool::RoundedConnector);
    }
}
