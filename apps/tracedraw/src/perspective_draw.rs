//! Draw in Perspective (View > Draw in Perspective): a one-, two- or
//! three-point perspective grid over the page. While it is on, every
//! object a drawing tool makes is laid onto the active plane of the grid:
//! the box it was drawn in is taken onto the plane and the object gets a
//! Perspective effect that maps it there exactly (a plane in perspective
//! is a projective image of a flat rectangle), so it stays editable with
//! the Shape tool. The vanishing points and the grid's near corner are
//! dragged on the page.

use crate::app::App;
use crate::tools::Tool;
use crate::ui::tips::RichTip;
use egui::{Color32, Painter, Pos2, Stroke as EStroke};
use std::collections::HashSet;
use tracedraw_core::geometry::{Affine, Point, Rect};
use tracedraw_core::live::{homography, map_homography, Effect};
use tracedraw_core::{Command, ShapeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    One,
    Two,
    Three,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::One, Kind::Two, Kind::Three];
    pub fn key(self) -> &'static str {
        match self {
            Kind::One => "perspective.one_point",
            Kind::Two => "perspective.two_point",
            Kind::Three => "perspective.three_point",
        }
    }
    /// The planes the kind offers.
    pub fn planes(self) -> &'static [Plane] {
        match self {
            Kind::One => &[Plane::Ground, Plane::Left, Plane::Right, Plane::Top],
            Kind::Two | Kind::Three => &[Plane::Ground, Plane::Left, Plane::Right],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plane {
    Ground,
    Left,
    Right,
    Top,
}

impl Plane {
    pub fn key(self) -> &'static str {
        match self {
            Plane::Ground => "perspective.ground",
            Plane::Left => "perspective.left",
            Plane::Right => "perspective.right",
            Plane::Top => "perspective.top",
        }
    }
}

/// What can be dragged on the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// A vanishing point (0, 1, 2).
    Vanish(usize),
    /// The near bottom corner of the grid.
    Corner,
    /// The near top corner (sets the walls' height).
    Top,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub kind: Kind,
    pub plane: Plane,
    /// Vanishing points: [left or only one, right, vertical].
    pub vanish: [Point; 3],
    /// The near bottom corner (two- and three-point) or near bottom-left
    /// corner (one-point) of the grid.
    pub corner: Point,
    /// The near top corner's height above `corner`, mm.
    pub height: f64,
    /// One-point: the near face's width, mm.
    pub width: f64,
    /// How far towards the vanishing points the grid reaches (0 to 1).
    pub depth: f64,
    pub grid: bool,
    pub drag: Option<Handle>,
    /// Objects already on the page (not laid onto the grid).
    pub known: HashSet<ShapeId>,
}

impl Session {
    /// A grid sized to a page `w` x `h` mm.
    pub fn new(kind: Kind, w: f64, h: f64) -> Self {
        let horizon = h * 0.55;
        Session {
            kind,
            plane: Plane::Ground,
            vanish: [
                Point::new(
                    if kind == Kind::One {
                        w * 0.5
                    } else {
                        -w * 0.35
                    },
                    horizon,
                ),
                Point::new(w * 1.35, horizon),
                Point::new(w * 0.5, h * 2.2),
            ],
            corner: if kind == Kind::One {
                Point::new(w * 0.15, h * 0.08)
            } else {
                Point::new(w * 0.5, h * 0.08)
            },
            height: h * 0.75,
            width: w * 0.7,
            depth: if kind == Kind::One { 0.7 } else { 0.45 },
            grid: true,
            drag: None,
            known: HashSet::new(),
        }
    }

    fn top_corner(&self) -> Point {
        let c = self.corner;
        match self.kind {
            Kind::Three => {
                // The vertical edge leans to the vertical vanishing point.
                let v = self.vanish[2];
                let d = v - c;
                let t = if d.y.abs() > 1e-9 {
                    self.height / d.y
                } else {
                    0.0
                };
                c + d * t.clamp(-1.0, 0.95)
            }
            _ => Point::new(c.x, c.y + self.height),
        }
    }

    /// The quad (page mm) the unit square of `plane` maps to, corners in
    /// the order (0,0), (1,0), (1,1), (0,1).
    pub fn quad(&self, plane: Plane) -> Option<[Point; 4]> {
        let f = self.depth.clamp(0.05, 0.98);
        let toward = |p: Point, v: Point| p + (v - p) * f;
        let c = self.corner;
        let top = self.top_corner();
        match self.kind {
            Kind::One => {
                let v = self.vanish[0];
                let (l, r) = (c.x, c.x + self.width);
                let (b, t) = (c.y, top.y);
                let (lb, rb, lt, rt) = (
                    Point::new(l, b),
                    Point::new(r, b),
                    Point::new(l, t),
                    Point::new(r, t),
                );
                Some(match plane {
                    Plane::Ground => [lb, rb, toward(rb, v), toward(lb, v)],
                    Plane::Left => [lb, toward(lb, v), toward(lt, v), lt],
                    Plane::Right => [toward(rb, v), rb, rt, toward(rt, v)],
                    Plane::Top => [toward(lt, v), toward(rt, v), rt, lt],
                })
            }
            Kind::Two | Kind::Three => {
                let (v1, v2) = (self.vanish[0], self.vanish[1]);
                let left_b = toward(c, v1);
                let right_b = toward(c, v2);
                // Far top corners: towards the side point from the top
                // corner, up the edge that leans to the vertical point (or
                // stands upright).
                let up = |p: Point| -> Point {
                    if self.kind == Kind::Three {
                        self.vanish[2]
                    } else {
                        Point::new(p.x, p.y + 1.0)
                    }
                };
                let left_t = intersect(top, v1, left_b, up(left_b))?;
                let right_t = intersect(top, v2, right_b, up(right_b))?;
                Some(match plane {
                    Plane::Ground => {
                        let far = intersect(left_b, v2, right_b, v1)?;
                        [c, right_b, far, left_b]
                    }
                    Plane::Left => [left_b, c, top, left_t],
                    Plane::Right | Plane::Top => [c, right_b, right_t, top],
                })
            }
        }
    }

    /// The homography from the unit square to the plane, and back.
    pub fn maps(&self, plane: Plane) -> Option<([f64; 8], [f64; 8])> {
        let q = self.quad(plane)?;
        let unit = [
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
        ];
        Some((homography(&unit, &q)?, homography(&q, &unit)?))
    }

    /// The page positions of the handles.
    pub fn handles(&self) -> Vec<(Handle, Point)> {
        let mut h = vec![(Handle::Vanish(0), self.vanish[0])];
        if self.kind != Kind::One {
            h.push((Handle::Vanish(1), self.vanish[1]));
        }
        if self.kind == Kind::Three {
            h.push((Handle::Vanish(2), self.vanish[2]));
        }
        h.push((Handle::Corner, self.corner));
        let top = self.top_corner();
        h.push((
            Handle::Top,
            if self.kind == Kind::One {
                Point::new(self.corner.x + self.width, top.y)
            } else {
                top
            },
        ));
        h
    }

    /// Move a handle to `p`.
    pub fn drag_to(&mut self, handle: Handle, p: Point) {
        match handle {
            Handle::Vanish(i) => {
                if let Some(v) = self.vanish.get_mut(i) {
                    *v = p;
                }
                // The side points share the horizon.
                if self.kind != Kind::One && i < 2 {
                    let y = p.y;
                    self.vanish[0].y = y;
                    self.vanish[1].y = y;
                }
            }
            Handle::Corner => self.corner = p,
            Handle::Top => {
                self.height = (p.y - self.corner.y).max(1.0);
                if self.kind == Kind::One {
                    self.width = (p.x - self.corner.x).max(1.0);
                }
            }
        }
    }
}

/// Where the line a→b crosses the line c→d.
fn intersect(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let r = b - a;
    let s = d - c;
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-12 {
        return None;
    }
    let t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / den;
    Some(a + r * t)
}

/// The perspective corners (local coordinates) that lay an object with
/// local box `b` and page transform `t` onto a plane with maps
/// `(to_page, to_plane)`.
pub fn laid_corners(
    b: Rect,
    t: Affine,
    to_page: &[f64; 8],
    to_plane: &[f64; 8],
) -> Option<[Point; 4]> {
    let local = [
        Point::new(b.x0, b.y0),
        Point::new(b.x1, b.y0),
        Point::new(b.x1, b.y1),
        Point::new(b.x0, b.y1),
    ];
    let plane: Vec<Point> = local
        .iter()
        .map(|p| map_homography(to_plane, t * *p))
        .collect();
    if plane.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return None;
    }
    // The box's sides on the plane: each side's mean, so a wall whose
    // lines lean against the page's keeps the drawn orientation.
    let (bl, br, tr, tl) = (plane[0], plane[1], plane[2], plane[3]);
    let mut r = Rect::new(
        (bl.x + tl.x) / 2.0,
        (bl.y + br.y) / 2.0,
        (br.x + tr.x) / 2.0,
        (tl.y + tr.y) / 2.0,
    );
    // Turned over: the box lies past the vanishing line of this plane.
    if r.x1 - r.x0 < 1e-9 || r.y1 - r.y0 < 1e-9 {
        return None;
    }
    // Every corner must stay on the near side of the vanishing line (the
    // projective map's denominator keeps its sign); a box drawn past it is
    // pulled towards the grid until it fits.
    let den = |p: Point| to_page[6] * p.x + to_page[7] * p.y + 1.0;
    let sign = den(Point::new(0.5, 0.5)).signum();
    let fits = |r: &Rect| {
        [
            Point::new(r.x0, r.y0),
            Point::new(r.x1, r.y0),
            Point::new(r.x1, r.y1),
            Point::new(r.x0, r.y1),
        ]
        .iter()
        .all(|p| den(*p) * sign > 0.05)
    };
    let mut tries = 0;
    while !fits(&r) {
        tries += 1;
        if tries > 40 {
            return None;
        }
        let k = 0.85;
        r = Rect::new(
            0.5 + (r.x0 - 0.5) * k,
            0.5 + (r.y0 - 0.5) * k,
            0.5 + (r.x1 - 0.5) * k,
            0.5 + (r.y1 - 0.5) * k,
        );
    }
    let det = t.determinant();
    if !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    let inv = t.inverse();
    let snapped = [
        Point::new(r.x0, r.y0),
        Point::new(r.x1, r.y0),
        Point::new(r.x1, r.y1),
        Point::new(r.x0, r.y1),
    ];
    let mut out = [Point::ZERO; 4];
    for (o, p) in out.iter_mut().zip(snapped.iter()) {
        *o = inv * map_homography(to_page, *p);
    }
    Some(out)
}

/// Tools that draw new objects (the ones laid onto the grid).
fn draws(tool: Tool) -> bool {
    !matches!(
        tool,
        Tool::Pick
            | Tool::FreeformPick
            | Tool::FreeTransform
            | Tool::Shape
            | Tool::Zoom
            | Tool::Pan
            | Tool::Crop
            | Tool::Knife
            | Tool::Eraser
            | Tool::SegmentDelete
            | Tool::ColorEyedropper
            | Tool::AttributesEyedropper
            | Tool::InteractiveFill
            | Tool::MeshFill
            | Tool::Transparency
            | Tool::OutlinePen
            | Tool::OutlineColor
    )
}

impl App {
    /// The objects on the current page, at the top level.
    fn page_object_ids(&self) -> HashSet<ShapeId> {
        self.doc()
            .page(self.page)
            .map(|p| {
                p.layers
                    .iter()
                    .flat_map(|l| l.shapes.iter().map(|s| s.id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// View > Draw in Perspective on or off.
    pub fn toggle_perspective_drawing(&mut self) {
        if self.perspective.is_some() {
            self.perspective = None;
            return;
        }
        let size = self.page_size();
        let mut s = Session::new(Kind::One, size.width, size.height);
        s.known = self.page_object_ids();
        self.perspective = Some(s);
    }

    /// Change the grid's kind, keeping its plane when the kind has it.
    pub fn set_perspective_kind(&mut self, kind: Kind) {
        let size = self.page_size();
        if let Some(s) = self.perspective.as_mut() {
            let plane = s.plane;
            let known = std::mem::take(&mut s.known);
            *s = Session::new(kind, size.width, size.height);
            s.known = known;
            if kind.planes().contains(&plane) {
                s.plane = plane;
            }
        }
    }

    /// Lay the selected objects onto the active plane, one undo step each.
    pub fn lay_selection_onto_plane(&mut self) -> usize {
        let ids = self.selection.clone();
        ids.into_iter()
            .filter(|id| self.lay_onto_plane_with(*id, false))
            .count()
    }

    /// Lay `id` onto the active plane (one undo step with its drawing).
    pub fn lay_onto_plane(&mut self, id: ShapeId) -> bool {
        self.lay_onto_plane_with(id, true)
    }

    /// Lay `id` onto the active plane; with `amend`, as part of the step
    /// just recorded.
    fn lay_onto_plane_with(&mut self, id: ShapeId, amend: bool) -> bool {
        let Some(session) = self.perspective.as_ref() else {
            return false;
        };
        let Some((to_page, to_plane)) = session.maps(session.plane) else {
            return false;
        };
        let doc = self.doc();
        let Some(shape) = doc.find_shape(id).cloned() else {
            return false;
        };
        let parent = doc.parent_transform(id).unwrap_or(Affine::IDENTITY);
        use tracedraw_core::geometry::Shape as _;
        let b = shape.local_path().bounding_box();
        let Some(corners) = laid_corners(b, parent * shape.transform, &to_page, &to_plane) else {
            return false;
        };
        let mut effects = shape.effects.clone();
        effects.retain(|e| !matches!(e, Effect::Perspective { .. }));
        effects.push(Effect::Perspective { corners });
        let cmd = Command::SetEffects { shape: id, effects };
        if !amend || self.engine.amend(&cmd).is_err() {
            self.run(cmd);
        }
        true
    }

    /// After each frame's input: objects a drawing tool just made go onto
    /// the plane; anything else new (pasted, duplicated) is left as it is.
    pub fn perspective_tick(&mut self) {
        if self.perspective.is_none() || !matches!(self.drag, crate::app::Drag::None) {
            return;
        }
        let now = self.page_object_ids();
        let new: Vec<ShapeId> = match self.perspective.as_ref() {
            Some(s) => now
                .iter()
                .filter(|id| !s.known.contains(id))
                .copied()
                .collect(),
            None => return,
        };
        if new.is_empty() {
            return;
        }
        if draws(self.tool) && self.text_edit.is_none() {
            for id in &new {
                self.lay_onto_plane(*id);
            }
        }
        if let Some(s) = self.perspective.as_mut() {
            s.known.extend(new);
        }
    }

    /// A press on a handle drags it; true when the pointer was used here.
    pub fn perspective_pointer(&mut self, response: &egui::Response) -> bool {
        let Some(pos) = response.interact_pointer_pos().or(response.hover_pos()) else {
            return false;
        };
        let view = self.view;
        let Some(s) = self.perspective.as_mut() else {
            return false;
        };
        if response.drag_started_by(egui::PointerButton::Primary) {
            s.drag = s
                .handles()
                .into_iter()
                .find(|(_, p)| (view.to_screen(*p) - pos).length() <= 8.0)
                .map(|(h, _)| h);
        }
        let Some(h) = s.drag else {
            return false;
        };
        if response.dragged_by(egui::PointerButton::Primary) {
            s.drag_to(h, view.to_page(pos));
        }
        if response.drag_stopped() {
            s.drag = None;
        }
        true
    }
}

/// The grid: the active plane's lines strong, the others faint; the
/// horizon, the vanishing points and the corner handles.
pub fn draw_grid(app: &App, painter: &Painter, rect: egui::Rect) {
    let Some(s) = app.perspective.as_ref() else {
        return;
    };
    let view = &app.view;
    let blue = Color32::from_rgb(0x1F, 0x8F, 0xE0);
    // Horizon.
    let hy = view.to_screen(s.vanish[0]).y;
    painter.line_segment(
        [Pos2::new(rect.left(), hy), Pos2::new(rect.right(), hy)],
        EStroke::new(1.0, blue.gamma_multiply(0.7)),
    );
    if s.grid {
        for plane in s.kind.planes() {
            let Some((to_page, _)) = s.maps(*plane) else {
                continue;
            };
            let active = *plane == s.plane;
            let stroke = EStroke::new(
                if active { 1.0 } else { 0.7 },
                blue.gamma_multiply(if active { 0.75 } else { 0.25 }),
            );
            let n = 8;
            for i in 0..=n {
                let t = i as f64 / n as f64;
                for (a, b) in [
                    (Point::new(t, 0.0), Point::new(t, 1.0)),
                    (Point::new(0.0, t), Point::new(1.0, t)),
                ] {
                    let pa = view.to_screen(map_homography(&to_page, a));
                    let pb = view.to_screen(map_homography(&to_page, b));
                    painter.line_segment([pa, pb], stroke);
                }
            }
        }
    }
    for (h, p) in s.handles() {
        let at = view.to_screen(p);
        match h {
            Handle::Vanish(_) => {
                painter.circle_filled(at, 7.0, Color32::WHITE);
                painter.circle_stroke(at, 7.0, EStroke::new(2.0, blue));
                painter.circle_filled(at, 2.5, blue);
            }
            _ => {
                let r = egui::Rect::from_center_size(at, egui::vec2(9.0, 9.0));
                painter.rect_filled(r, 0.0, Color32::WHITE);
                painter.rect_stroke(r, 0.0, EStroke::new(1.5, blue), egui::StrokeKind::Middle);
            }
        }
    }
}

/// The bar at the top of the drawing window while drawing in
/// perspective: kind, plane, grid, Finish.
pub fn show_bar(app: &mut App, ctx: &egui::Context) {
    if app.perspective.is_none() {
        return;
    }
    use crate::i18n::tr;
    let at = app.canvas_rect.min + egui::vec2(14.0, 14.0);
    egui::Area::new(egui::Id::new("perspective_bar"))
        .fixed_pos(at)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(tr("perspective.finish")).clicked() {
                        app.perspective = None;
                        return;
                    }
                    ui.separator();
                    let Some(kind) = app.perspective.as_ref().map(|s| s.kind) else {
                        return;
                    };
                    for k in Kind::ALL {
                        if ui.selectable_label(kind == k, tr(k.key())).clicked() && kind != k {
                            app.set_perspective_kind(k);
                        }
                    }
                    ui.separator();
                    if let Some(s) = app.perspective.as_mut() {
                        for p in s.kind.planes() {
                            if ui.selectable_label(s.plane == *p, tr(p.key())).clicked() {
                                s.plane = *p;
                            }
                        }
                        ui.separator();
                        ui.checkbox(&mut s.grid, tr("perspective.grid"));
                        ui.separator();
                        ui.label(tr("perspective.depth"));
                        let mut d = (s.depth * 100.0).round();
                        if ui
                            .add(
                                crate::ui::field::NumField::new(&mut d)
                                    .range(5.0..=98.0)
                                    .suffix(" %")
                                    .width(50.0),
                            )
                            .changed()
                        {
                            s.depth = d / 100.0;
                        }
                    }
                    let some = !app.selection.is_empty();
                    if ui
                        .add_enabled(some, egui::Button::new(tr("perspective.put_on_plane")))
                        .rich_tip(tr("perspective.put_on_plane_tip"))
                        .clicked()
                    {
                        app.lay_selection_onto_plane();
                    }
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_plane_maps_the_unit_square_onto_its_quad_and_back() {
        for kind in Kind::ALL {
            let s = Session::new(kind, 210.0, 297.0);
            for plane in kind.planes() {
                let q = s.quad(*plane).expect("a quad");
                let (to_page, to_plane) = s.maps(*plane).expect("maps");
                let unit = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
                for ((u, w), corner) in unit.iter().zip(q.iter()) {
                    let p = map_homography(&to_page, Point::new(*u, *w));
                    assert!((p - *corner).hypot() < 1e-6, "{kind:?} {plane:?}");
                    let back = map_homography(&to_plane, p);
                    assert!((back - Point::new(*u, *w)).hypot() < 1e-6);
                }
            }
        }
    }

    #[test]
    fn grid_lines_meet_at_the_vanishing_points() {
        // One-point ground: the depth lines head for the vanishing point.
        let s = Session::new(Kind::One, 200.0, 200.0);
        let q = s.quad(Plane::Ground).expect("a quad");
        let meet = intersect(q[0], q[3], q[1], q[2]).expect("they meet");
        assert!((meet - s.vanish[0]).hypot() < 1e-6, "{meet:?}");
        // Two-point ground: each pair of sides meets at its own point.
        let s = Session::new(Kind::Two, 200.0, 200.0);
        let q = s.quad(Plane::Ground).expect("a quad");
        let a = intersect(q[0], q[3], q[1], q[2]).expect("meet");
        let b = intersect(q[0], q[1], q[3], q[2]).expect("meet");
        assert!((a - s.vanish[0]).hypot() < 1e-6 && (b - s.vanish[1]).hypot() < 1e-6);
        // Three-point walls: the vertical edges lean to the third point.
        let s = Session::new(Kind::Three, 200.0, 200.0);
        let q = s.quad(Plane::Left).expect("a quad");
        let v = intersect(q[0], q[3], q[1], q[2]).expect("meet");
        assert!((v - s.vanish[2]).hypot() < 1e-3, "{v:?}");
    }

    #[test]
    fn boxes_drawn_across_a_wall_keep_their_corners_apart_and_their_side_up() {
        let s = Session::new(Kind::One, 297.0, 210.0);
        for plane in Kind::One.planes() {
            let (tp, tl) = s.maps(*plane).expect("maps");
            // Partly off the wall, reaching towards the vanishing point.
            let b = Rect::new(101.0, 55.7, 129.0, 83.6);
            let laid = laid_corners(b, Affine::IDENTITY, &tp, &tl);
            // Past the right wall's and the ceiling's vanishing lines (the
            // box is left of the far end of the right wall, below the
            // horizon): left flat.
            if matches!(plane, Plane::Right | Plane::Top) {
                assert!(laid.is_none(), "{laid:?}");
                continue;
            }
            let c = laid.expect("laid");
            for i in 0..4 {
                for j in i + 1..4 {
                    assert!((c[i] - c[j]).hypot() > 0.5, "{plane:?} {c:?}");
                }
            }
            // The quad is not turned over: same winding as the drawn box.
            let area: f64 = (0..4)
                .map(|i| {
                    let (a, b) = (c[i], c[(i + 1) % 4]);
                    a.x * b.y - b.x * a.y
                })
                .sum();
            assert!(area > 0.0, "{plane:?} {c:?}");
        }
    }

    #[test]
    fn a_rectangle_drawn_on_the_grid_lands_on_the_plane_in_one_undo_step() {
        let mut app = App::headless();
        app.toggle_perspective_drawing();
        app.tool = Tool::Rectangle;
        let layer = app.active_layer().expect("a layer");
        let id = app.engine.new_shape_id();
        let shape = tracedraw_core::Shape::new(
            id,
            tracedraw_core::ShapeKind::Rect {
                rect: Rect::new(60.0, 30.0, 120.0, 60.0),
                radius: 0.0,
                corners: None,
            },
        );
        app.run(Command::AddShape { layer, shape });
        app.perspective_tick();
        let s = app.doc().find_shape(id).cloned().expect("the rectangle");
        assert!(s
            .effects
            .iter()
            .any(|e| matches!(e, Effect::Perspective { .. })));
        // Its corners lie on the ground plane: their plane coordinates form
        // a rectangle (two by two the same u and w).
        let session = app.perspective.as_ref().expect("a session");
        let (_, to_plane) = session.maps(Plane::Ground).expect("maps");
        let corners = match s.effects.iter().find_map(|e| match e {
            Effect::Perspective { corners } => Some(*corners),
            _ => None,
        }) {
            Some(c) => c,
            None => unreachable!(),
        };
        let p: Vec<Point> = corners
            .iter()
            .map(|c| map_homography(&to_plane, s.transform * *c))
            .collect();
        assert!((p[0].y - p[1].y).abs() < 1e-6 && (p[2].y - p[3].y).abs() < 1e-6);
        assert!((p[0].x - p[3].x).abs() < 1e-6 && (p[1].x - p[2].x).abs() < 1e-6);
        // Foreshortened: the far (upper) edge is shorter than the near one.
        let page: Vec<Point> = corners.iter().map(|c| s.transform * *c).collect();
        assert!((page[2] - page[3]).hypot() < (page[1] - page[0]).hypot());
        // Drawing and laying it down undo together.
        app.undo();
        assert!(app.doc().find_shape(id).is_none());
        // With the Pick tool, new objects (pasted, duplicated) stay flat.
        app.tool = Tool::Pick;
        let id2 = app.engine.new_shape_id();
        let shape = tracedraw_core::Shape::new(
            id2,
            tracedraw_core::ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            },
        );
        app.run(Command::AddShape { layer, shape });
        app.perspective_tick();
        assert!(app
            .doc()
            .find_shape(id2)
            .is_some_and(|s| s.effects.is_empty()));
        // Put on Plane lays an existing object down as a step of its own.
        app.select(vec![id2]);
        assert_eq!(app.lay_selection_onto_plane(), 1);
        assert!(app
            .doc()
            .find_shape(id2)
            .is_some_and(|s| !s.effects.is_empty()));
        app.undo();
        assert!(app
            .doc()
            .find_shape(id2)
            .is_some_and(|s| s.effects.is_empty()));
        app.toggle_perspective_drawing();
        assert!(app.perspective.is_none());
    }
}
