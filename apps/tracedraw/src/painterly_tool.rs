//! The Painterly Brush tool: freehand strokes drawn with a painterly
//! brush (chalk, watercolour, bristles, fur, spray and more). A stroke is
//! an ordinary curve whose outline is painted with brush marks, so the
//! Shape tool reshapes it; choosing a brush, size or transparency with
//! curves selected paints them with it, and Clear Brushstroke gives back
//! the plain outline.

use crate::app::App;
use crate::i18n::tr;
use crate::ui::tips::RichTip;
use tracedraw_core::geometry::{BezPath, Point};
use tracedraw_core::painterly::{Painterly, PainterlyBrush};
use tracedraw_core::{Color, Command, Fill, Shape, ShapeId, ShapeKind, Stroke};

/// The tool's brush and size for new strokes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PainterlySettings {
    pub brush: Painterly,
    /// Brush size (the outline width), mm.
    pub size: f64,
}

impl Default for PainterlySettings {
    fn default() -> Self {
        PainterlySettings {
            brush: Painterly::new(PainterlyBrush::Chalk),
            size: 5.0,
        }
    }
}

/// i18n keys of the picker's groups.
pub const GROUPS: [&str; 3] = [
    "painterly.group_dry",
    "painterly.group_wet",
    "painterly.group_texture",
];

pub fn brush_name(b: PainterlyBrush) -> String {
    tr(&format!("painterly.{}", b.id()))
}

impl App {
    /// The outline a brushstroke gets: the default outline's colour
    /// (black without one), the tool's size and brush, a seed from the
    /// object.
    fn painterly_stroke(&self, base: Option<&Stroke>, id: ShapeId) -> Stroke {
        let color = base
            .map(|s| s.color)
            .or_else(|| self.default_stroke().map(|s| s.color))
            .unwrap_or(Color::BLACK);
        let mut stroke = base.cloned().unwrap_or_else(|| Stroke::new(color, 1.0));
        stroke.width = self.painterly.size.max(0.05);
        stroke.dash.clear();
        stroke.widths.clear();
        stroke.stretch = 1.0;
        let mut brush = self.painterly.brush;
        brush.seed = (id.0 as u32).wrapping_mul(2_654_435_761).max(1);
        stroke.painterly = Some(brush);
        stroke
    }

    /// A freehand drag ended: the points become a smoothed curve painted
    /// with the brush.
    pub fn add_painterly_stroke(&mut self, points: &[Point]) -> Option<ShapeId> {
        let tol = self.settings.curves.freehand_tolerance_px() / self.view.zoom.max(1e-6) as f64;
        let pts = tracedraw_core::geometry::simplify(points, tol.max(0.02));
        if pts.len() < 2 {
            return None;
        }
        let path: BezPath = tracedraw_core::geometry::smooth_path(&pts, false);
        let layer = self.active_layer()?;
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(
            id,
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        shape.fill = Fill::None;
        shape.stroke = Some(self.painterly_stroke(None, id));
        shape.name = Some(tr("painterly.object_name"));
        self.run(Command::AddShape { layer, shape });
        self.selection = vec![id];
        Some(id)
    }

    /// Paint the selected objects' outlines with the tool's brush and
    /// size (the brush picker or a setting changed with them selected).
    pub fn apply_painterly_to_selection(&mut self) {
        let shapes = self.selected_shapes();
        for s in shapes {
            if s.page_path().elements().is_empty() {
                continue;
            }
            // Keep each stroke's own seed so changing the size or
            // transparency does not reshuffle its marks.
            let mut stroke = self.painterly_stroke(s.stroke.as_ref(), s.id);
            if let (Some(old), Some(new)) = (
                s.stroke.as_ref().and_then(|st| st.painterly),
                stroke.painterly.as_mut(),
            ) {
                new.seed = old.seed;
            }
            self.run(Command::SetStroke {
                shapes: vec![s.id],
                stroke: Some(stroke),
            });
        }
    }

    /// Clear Brushstroke: the selected outlines become plain again.
    pub fn clear_brushstroke(&mut self) {
        for s in self.selected_shapes() {
            let Some(mut st) = s.stroke.clone().filter(|st| st.painterly.is_some()) else {
                continue;
            };
            st.painterly = None;
            self.run(Command::SetStroke {
                shapes: vec![s.id],
                stroke: Some(st),
            });
        }
    }

    /// Some selected object is painted with a brush.
    pub fn selection_has_brushstroke(&self) -> bool {
        self.selected_shapes()
            .iter()
            .any(|s| s.stroke.as_ref().is_some_and(|st| st.painterly.is_some()))
    }

    /// The brush of the first selected brushstroke, shown on the bar.
    pub fn sync_painterly_from_selection(&mut self) {
        if let Some(st) = self
            .selected_shapes()
            .iter()
            .filter_map(|s| s.stroke.clone())
            .find(|st| st.painterly.is_some())
        {
            if let Some(b) = st.painterly {
                self.painterly.brush.brush = b.brush;
                self.painterly.brush.transparency = b.transparency;
            }
            self.painterly.size = st.width;
        }
    }
}

/// The property bar: brush picker, size, transparency, Clear Brushstroke.
pub fn painterly_bar(app: &mut App, ui: &mut egui::Ui) {
    use crate::ui::field::NumField;
    app.sync_painterly_from_selection();
    let before = app.painterly;
    let current = app.painterly.brush.brush;
    egui::ComboBox::from_id_salt("painterly_brush")
        .selected_text(brush_name(current))
        .width(150.0)
        .height(420.0)
        .show_ui(ui, |ui| {
            for (g, key) in GROUPS.iter().enumerate() {
                ui.label(
                    egui::RichText::new(tr(key))
                        .color(crate::theme::Tokens::SELECTION)
                        .size(12.0),
                );
                for b in PainterlyBrush::ALL.into_iter().filter(|b| b.group() == g) {
                    ui.selectable_value(&mut app.painterly.brush.brush, b, brush_name(b));
                }
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("painterly.brush_tip"));
    ui.label("⬤").rich_tip(tr("painterly.size"));
    let mut size = app.units.world_from_mm(app.painterly.size);
    if ui
        .add(
            NumField::new(&mut size)
                .range(app.units.world_from_mm(0.1)..=app.units.world_from_mm(200.0))
                .fixed_decimals(app.settings.precision.min(10) as usize)
                .unit_mm(app.units.world_to_mm(1.0))
                .suffix(format!(" {}", app.units.short()))
                .width(76.0),
        )
        .rich_tip(tr("painterly.size"))
        .changed()
    {
        app.painterly.size = app.units.world_to_mm(size).max(0.1);
    }
    ui.label("▦").rich_tip(tr("painterly.transparency"));
    let mut pct = (app.painterly.brush.transparency * 100.0).round();
    if ui
        .add(
            NumField::new(&mut pct)
                .range(0.0..=100.0)
                .suffix(" %")
                .width(56.0),
        )
        .rich_tip(tr("painterly.transparency"))
        .changed()
    {
        app.painterly.brush.transparency = (pct / 100.0).clamp(0.0, 1.0);
    }
    crate::ui::propbar::sep(ui);
    let has = app.selection_has_brushstroke();
    if ui
        .add_enabled(has, egui::Button::new(tr("painterly.clear")))
        .clicked()
    {
        app.clear_brushstroke();
        return;
    }
    if app.painterly != before {
        // A new brush paints whatever is selected; size and transparency
        // change the brushstrokes already there.
        let brush_changed = app.painterly.brush.brush != before.brush.brush;
        if !app.selection.is_empty() && (brush_changed || has) {
            app.apply_painterly_to_selection();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_freehand_drag_becomes_a_brushstroke_that_can_be_restyled_and_cleared() {
        let mut app = App::headless();
        app.painterly.size = 8.0;
        app.painterly.brush.brush = PainterlyBrush::Watercolor;
        let pts: Vec<Point> = (0..40)
            .map(|i| Point::new(20.0 + i as f64 * 2.0, 100.0 + (i as f64 * 0.3).sin() * 10.0))
            .collect();
        let id = app.add_painterly_stroke(&pts).expect("a stroke");
        let stroke = |app: &App| {
            app.doc()
                .find_shape(id)
                .and_then(|s| s.stroke.clone())
                .expect("an outline")
        };
        let st = stroke(&app);
        assert_eq!(st.width, 8.0);
        let brush = st.painterly.expect("a brush");
        assert_eq!(brush.brush, PainterlyBrush::Watercolor);
        assert_eq!(app.selection, vec![id]);
        assert!(app.selection_has_brushstroke());
        // Another brush on the selection keeps the stroke's seed.
        app.painterly.brush.brush = PainterlyBrush::Fur;
        app.painterly.size = 3.0;
        app.apply_painterly_to_selection();
        let st = stroke(&app);
        assert_eq!(st.width, 3.0);
        let fur = st.painterly.expect("a brush");
        assert_eq!((fur.brush, fur.seed), (PainterlyBrush::Fur, brush.seed));
        // The bar shows the selection's brush.
        app.painterly = PainterlySettings::default();
        app.sync_painterly_from_selection();
        assert_eq!(app.painterly.brush.brush, PainterlyBrush::Fur);
        assert_eq!(app.painterly.size, 3.0);
        app.clear_brushstroke();
        assert!(stroke(&app).painterly.is_none());
        assert!(!app.selection_has_brushstroke());
        // Too short a drag draws nothing.
        assert!(app.add_painterly_stroke(&[Point::new(1.0, 1.0)]).is_none());
    }
}
