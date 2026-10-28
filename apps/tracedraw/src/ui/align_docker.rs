//! The Align and Distribute docker. Align: six picture buttons (left
//! edges, centres on a vertical line, right edges; top edges, centres on
//! a horizontal line, bottom edges), what they align to (the active
//! object, the page's edges, the page's centre, the grid) and Use Outline.
//! Distribute: eight picture buttons (left edges, centres, gaps and right
//! edges along x; top edges, centres, gaps and bottom edges along y) and
//! where they spread the objects (between the outermost, across the page).
//! See `docs/behavior/align-distribute.md`.

use crate::app::App;
use crate::i18n::tr;
use crate::ops::{Align, Distribute, DistributeTo};
use crate::ops2::AlignTo;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

/// The blue of the boxes in the pictures.
const BOX: Color32 = Color32::from_rgb(0x4A, 0x90, 0xD9);
const BOX_EDGE: Color32 = Color32::from_rgb(0x2F, 0x6D, 0xB5);

/// A picture on a docker button.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Icon {
    Align(Align),
    Distribute(Distribute),
    To(AlignTo),
    Extent(DistributeTo),
    Outline,
}

/// Draws on a 16 x 16 grid in `r`, with x and y swapped when `swap` (the
/// pictures along y are those along x turned over the diagonal).
struct Pen<'a> {
    p: &'a Painter,
    r: Rect,
    swap: bool,
    color: Color32,
}

impl Pen<'_> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        let (x, y) = if self.swap { (y, x) } else { (x, y) };
        self.r.min + Vec2::new(x, y) * (self.r.width() / 16.0)
    }

    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_two_pos(self.at(x0, y0), self.at(x1, y1))
    }

    /// A filled box with its edge.
    fn block(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let r = self.rect(x0, y0, x1, y1);
        let enabled = self.color == Tokens::icon();
        let (fill, edge) = if enabled {
            (BOX, BOX_EDGE)
        } else {
            (Tokens::icon_off(), Tokens::icon_off())
        };
        self.p.rect_filled(r, 0.0, fill);
        self.p
            .rect_stroke(r, 0.0, Stroke::new(0.8, edge), egui::StrokeKind::Inside);
    }

    fn line(&self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32) {
        self.p.line_segment(
            [self.at(x0, y0), self.at(x1, y1)],
            Stroke::new(width, self.color),
        );
    }

    fn frame(&self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32) {
        self.p.rect_stroke(
            self.rect(x0, y0, x1, y1),
            0.0,
            Stroke::new(width, self.color),
            egui::StrokeKind::Middle,
        );
    }

    /// A dotted frame.
    fn dotted(&self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let r = self.rect(x0, y0, x1, y1);
        let step = 2.5;
        let mut t = 0.0;
        let corners = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
        ];
        for i in 0..4 {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            let len = (b - a).length();
            while t < len {
                let p = a + (b - a) * (t / len.max(1e-3));
                self.p.circle_filled(p, 0.6, self.color);
                t += step;
            }
            t -= len;
        }
    }
}

/// Align pictures: `edge` 0 the low edge (left, or top on screen), 1 the
/// centre, 2 the high edge; a line and two bars against it.
fn align_picture(pen: &Pen, edge: u8) {
    match edge {
        0 => {
            pen.block(2.5, 3.0, 11.0, 6.5);
            pen.block(2.5, 9.5, 14.5, 13.0);
            pen.line(1.5, 1.0, 1.5, 15.0, 1.3);
        }
        1 => {
            pen.block(4.0, 3.0, 12.0, 6.5);
            pen.block(2.0, 9.5, 14.0, 13.0);
            pen.line(8.0, 1.0, 8.0, 15.0, 1.3);
        }
        _ => {
            pen.block(5.0, 3.0, 13.5, 6.5);
            pen.block(1.5, 9.5, 13.5, 13.0);
            pen.line(14.5, 1.0, 14.5, 15.0, 1.3);
        }
    }
}

/// Distribute pictures: three boxes whose low edges (0), centres (1) or
/// high edges (2) are evenly spaced, marked by lines, or with equal gaps
/// (3), marked under them.
fn distribute_picture(pen: &Pen, kind: u8) {
    let boxes: [(f32, f32, f32, f32); 3] = match kind {
        0 => [
            (1.5, 5.0, 4.5, 11.0),
            (6.5, 3.0, 9.0, 13.0),
            (11.5, 6.0, 14.5, 10.0),
        ],
        1 => [
            (1.5, 5.0, 4.5, 11.0),
            (6.5, 3.0, 9.5, 13.0),
            (11.0, 6.0, 15.0, 10.0),
        ],
        2 => [
            (1.5, 5.0, 4.5, 11.0),
            (7.0, 3.0, 9.5, 13.0),
            (11.0, 6.0, 14.5, 10.0),
        ],
        _ => [
            (1.5, 4.0, 4.0, 11.0),
            (6.0, 2.0, 9.0, 11.0),
            (11.0, 5.0, 14.5, 11.0),
        ],
    };
    for (x0, y0, x1, y1) in boxes {
        pen.block(x0, y0, x1, y1);
    }
    match kind {
        0 => {
            for x in [1.5, 6.5, 11.5] {
                pen.line(x, 1.0, x, 15.0, 1.0);
            }
        }
        1 => {
            for x in [3.0, 8.0, 13.0] {
                pen.line(x, 1.0, x, 15.0, 1.0);
            }
        }
        2 => {
            for x in [4.5, 9.5, 14.5] {
                pen.line(x, 1.0, x, 15.0, 1.0);
            }
        }
        _ => {
            for (a, b) in [(4.0, 6.0), (9.0, 11.0)] {
                pen.line(a, 13.5, b, 13.5, 1.0);
                pen.line(a, 12.5, a, 14.5, 1.0);
                pen.line(b, 12.5, b, 14.5, 1.0);
            }
        }
    }
}

fn paint(p: &Painter, r: Rect, icon: Icon, color: Color32) {
    let pen = |swap: bool| Pen { p, r, swap, color };
    match icon {
        Icon::Align(a) => match a {
            Align::Left => align_picture(&pen(false), 0),
            Align::CenterV => align_picture(&pen(false), 1),
            Align::Right => align_picture(&pen(false), 2),
            Align::Top => align_picture(&pen(true), 0),
            Align::CenterH => align_picture(&pen(true), 1),
            Align::Bottom | Align::CenterPage => align_picture(&pen(true), 2),
        },
        Icon::Distribute(d) => match d {
            Distribute::Left => distribute_picture(&pen(false), 0),
            Distribute::CentersH => distribute_picture(&pen(false), 1),
            Distribute::Right => distribute_picture(&pen(false), 2),
            Distribute::SpacingH => distribute_picture(&pen(false), 3),
            // Along y the top is the low edge on screen.
            Distribute::Top => distribute_picture(&pen(true), 0),
            Distribute::CentersV => distribute_picture(&pen(true), 1),
            Distribute::Bottom => distribute_picture(&pen(true), 2),
            Distribute::SpacingV => distribute_picture(&pen(true), 3),
        },
        Icon::To(to) => {
            let pen = pen(false);
            match to {
                AlignTo::ActiveObject => {
                    pen.frame(1.5, 6.0, 9.5, 14.0, 1.0);
                    pen.block(6.5, 2.0, 14.5, 10.0);
                }
                AlignTo::PageEdge => {
                    pen.frame(3.0, 1.5, 13.0, 14.5, 1.2);
                    pen.block(3.0, 6.0, 7.5, 10.0);
                }
                AlignTo::PageCenter => {
                    pen.frame(3.0, 1.5, 13.0, 14.5, 1.2);
                    pen.block(5.75, 6.0, 10.25, 10.0);
                }
                AlignTo::Grid => {
                    for i in 0..4 {
                        let t = 2.0 + 4.0 * i as f32;
                        pen.line(t, 2.0, t, 14.0, 0.9);
                        pen.line(2.0, t, 14.0, t, 0.9);
                    }
                }
            }
        }
        Icon::Extent(to) => {
            let pen = pen(false);
            match to {
                DistributeTo::Selection => {
                    pen.dotted(1.0, 3.0, 15.0, 13.0);
                    pen.block(2.5, 6.0, 5.0, 10.0);
                    pen.block(6.75, 6.0, 9.25, 10.0);
                    pen.block(11.0, 6.0, 13.5, 10.0);
                }
                DistributeTo::Page => {
                    pen.frame(1.5, 1.5, 14.5, 14.5, 1.2);
                    pen.block(1.5, 6.0, 4.0, 10.0);
                    pen.block(6.75, 6.0, 9.25, 10.0);
                    pen.block(12.0, 6.0, 14.5, 10.0);
                }
            }
        }
        Icon::Outline => {
            let pen = pen(false);
            pen.dotted(1.0, 1.0, 15.0, 15.0);
            pen.block(4.0, 4.0, 12.0, 12.0);
            pen.frame(4.0, 4.0, 12.0, 12.0, 2.2);
        }
    }
}

/// A 28 x 26 picture button, shaded while `pressed`.
fn button(ui: &mut Ui, icon: Icon, tip: &str, enabled: bool, pressed: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(28.0, 26.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let fill = if pressed {
        Some(Tokens::tool_active())
    } else if enabled && resp.hovered() {
        Some(Tokens::tool_hover())
    } else {
        None
    };
    if let Some(c) = fill {
        ui.painter().rect_filled(rect, 2.0, c);
    }
    paint(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
        icon,
        if enabled {
            Tokens::icon()
        } else {
            Tokens::icon_off()
        },
    );
    resp.rich_tip(tip)
}

/// A small gap between groups of buttons.
fn gap(ui: &mut Ui) {
    ui.add_space(8.0);
}

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

/// The key naming what Align aligns to.
fn align_to_key(to: AlignTo) -> &'static str {
    match to {
        AlignTo::ActiveObject => "docker.align_active",
        AlignTo::PageEdge => "docker.align_page_edge",
        AlignTo::PageCenter => "docker.align_page_center",
        AlignTo::Grid => "docker.align_grid",
    }
}

/// Draw the docker.
pub fn align_distribute(app: &mut App, ui: &mut Ui) {
    let has = !app.selection.is_empty();
    ui.label(egui::RichText::new(tr("docker.align")).font(crate::theme::bold(13.0)));
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, (key, a)) in [
            ("menu.object.align_left", Align::Left),
            ("menu.object.align_centers_v", Align::CenterV),
            ("menu.object.align_right", Align::Right),
            ("menu.object.align_top", Align::Top),
            ("menu.object.align_centers_h", Align::CenterH),
            ("menu.object.align_bottom", Align::Bottom),
        ]
        .into_iter()
        .enumerate()
        {
            if i == 3 {
                gap(ui);
            }
            if button(ui, Icon::Align(a), &tr(key), has, false).clicked() {
                let to = app.align_to;
                app.align_with(a, to);
            }
        }
    });
    ui.add_space(4.0);
    ui.label(tr("docker.align_to"));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for to in [
            AlignTo::ActiveObject,
            AlignTo::PageEdge,
            AlignTo::PageCenter,
            AlignTo::Grid,
        ] {
            if button(
                ui,
                Icon::To(to),
                &tr(align_to_key(to)),
                true,
                app.align_to == to,
            )
            .clicked()
            {
                app.align_to = to;
            }
        }
        gap(ui);
        if button(
            ui,
            Icon::Outline,
            &tr("docker.align_use_outline"),
            true,
            app.align_use_outline,
        )
        .clicked()
        {
            app.align_use_outline = !app.align_use_outline;
        }
    });
    dim(ui, tr(align_to_key(app.align_to)));
    ui.add_space(4.0);
    ui.separator();
    ui.label(egui::RichText::new(tr("docker.distribute")).font(crate::theme::bold(13.0)));
    ui.add_space(2.0);
    let can = app.can_distribute(app.distribute_to);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, (key, d)) in [
            ("menu.object.distribute_left", Distribute::Left),
            ("menu.object.distribute_centers_h", Distribute::CentersH),
            ("menu.object.distribute_spacing_h", Distribute::SpacingH),
            ("menu.object.distribute_right", Distribute::Right),
            ("menu.object.distribute_top", Distribute::Top),
            ("menu.object.distribute_centers_v", Distribute::CentersV),
            ("menu.object.distribute_spacing_v", Distribute::SpacingV),
            ("menu.object.distribute_bottom", Distribute::Bottom),
        ]
        .into_iter()
        .enumerate()
        {
            if i == 4 {
                gap(ui);
            }
            if button(ui, Icon::Distribute(d), &tr(key), can, false).clicked() {
                let to = app.distribute_to;
                app.distribute_with(d, to);
            }
        }
    });
    ui.add_space(4.0);
    ui.label(tr("docker.distribute_to"));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (to, key) in [
            (DistributeTo::Selection, "docker.distribute_selection"),
            (DistributeTo::Page, "docker.distribute_page"),
        ] {
            if button(
                ui,
                Icon::Extent(to),
                &tr(key),
                true,
                app.distribute_to == to,
            )
            .clicked()
            {
                app.distribute_to = to;
            }
        }
    });
    dim(
        ui,
        tr(match app.distribute_to {
            DistributeTo::Selection => "docker.distribute_selection",
            DistributeTo::Page => "docker.distribute_page",
        }),
    );
    if !can {
        ui.add_space(2.0);
        dim(
            ui,
            tr(match app.distribute_to {
                DistributeTo::Selection => "docker.distribute_hint",
                DistributeTo::Page => "docker.distribute_page_hint",
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{geometry::Rect as PageRect, ShapeId, ShapeKind};

    fn rect(app: &mut App, x0: f64, y0: f64, x1: f64, y1: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: tracedraw_core::Rect::new(x0, y0, x1, y1),
            radius: 0.0,
            corners: None,
        })
        .expect("shape")
    }

    fn bounds(app: &App, id: ShapeId) -> PageRect {
        app.doc().shape_on_page(id).expect("shape").bounds()
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn align_follows_the_target_the_docker_shows() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 20.0, 20.0);
        let b = rect(&mut app, 50.0, 40.0, 70.0, 60.0);
        app.select(vec![a, b]);
        // The active object is the last selected.
        app.align_with(Align::Left, AlignTo::ActiveObject);
        assert!(near(bounds(&app, a).x0, 50.0));
        // The page's edges.
        app.align_with(Align::Right, AlignTo::PageEdge);
        let page = app.page_rect();
        assert!(near(bounds(&app, a).x1, page.x1) && near(bounds(&app, b).x1, page.x1));
        // The page's centre: the top edges go to its middle.
        app.align_with(Align::Top, AlignTo::PageCenter);
        let c = page.center();
        assert!(near(bounds(&app, a).y1, c.y) && near(bounds(&app, b).y1, c.y));
        // The grid: each left edge to its nearest grid line.
        app.align_with(Align::Left, AlignTo::Grid);
        let g = app.doc().metadata.grid;
        let o = app.doc().metadata.rulers.origin_x;
        for id in [a, b] {
            let k = (bounds(&app, id).x0 - o) / g.spacing_x;
            assert!(near(k, k.round()), "{k}");
        }
        // Centres on a vertical line share x.
        app.align_with(Align::CenterV, AlignTo::ActiveObject);
        assert!(near(bounds(&app, a).center().x, bounds(&app, b).center().x));
    }

    #[test]
    fn use_outline_aligns_the_outer_edges_of_the_outlines() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 20.0, 20.0);
        let b = rect(&mut app, 50.0, 40.0, 70.0, 60.0);
        let thick = tracedraw_core::style::Stroke::new(tracedraw_core::Color::BLACK, 4.0);
        app.run(tracedraw_core::Command::SetStroke {
            shapes: vec![a],
            stroke: Some(thick),
        });
        app.run(tracedraw_core::Command::SetStroke {
            shapes: vec![b],
            stroke: None,
        });
        app.select(vec![a, b]);
        app.align_use_outline = true;
        app.align_with(Align::Left, AlignTo::ActiveObject);
        // The outline's outer edge (2 mm out) meets b's left edge.
        assert!(near(bounds(&app, a).x0 - 2.0, 50.0));
    }

    #[test]
    fn distribute_spreads_edges_centres_and_gaps() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        let b = rect(&mut app, 13.0, 0.0, 33.0, 10.0);
        let c = rect(&mut app, 90.0, 0.0, 100.0, 10.0);
        app.select(vec![a, b, c]);
        // Left edges between 0 and 90: b's goes to 45.
        app.distribute_with(Distribute::Left, DistributeTo::Selection);
        assert!(near(bounds(&app, b).x0, 45.0));
        // Right edges between 10 and 100: b's goes to 55.
        app.distribute_with(Distribute::Right, DistributeTo::Selection);
        assert!(near(bounds(&app, b).x1, 55.0));
        // Equal gaps: 100 - 40 of boxes = 60, two gaps of 30.
        app.distribute_with(Distribute::SpacingH, DistributeTo::Selection);
        assert!(near(bounds(&app, b).x0, 40.0));
        // Across the page: the first touches its left edge, the last its
        // right edge.
        app.distribute_with(Distribute::SpacingH, DistributeTo::Page);
        let page = app.page_rect();
        assert!(near(bounds(&app, a).x0, page.x0));
        assert!(near(bounds(&app, c).x1, page.x1));
        // Along y the tops of two objects go to the page's top and to
        // the height of the lower one above its bottom.
        app.select(vec![a, c]);
        assert!(app.can_distribute(DistributeTo::Page));
        assert!(!app.can_distribute(DistributeTo::Selection));
        app.distribute_with(Distribute::Top, DistributeTo::Page);
        let tops = [bounds(&app, a).y1, bounds(&app, c).y1];
        assert!(tops.iter().any(|t| near(*t, page.y1)));
        assert!(tops.iter().any(|t| near(*t, page.y0 + 10.0)));
    }

    #[test]
    fn the_docker_draws_with_and_without_a_selection() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for select in [false, true] {
            if select {
                let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
                app.select(vec![a]);
                app.distribute_to = DistributeTo::Page;
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 600.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| align_distribute(&mut app, ui));
            out.textures_delta.clear();
        }
    }
}
