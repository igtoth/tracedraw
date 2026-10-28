//! The steps of each tool's Hints page. The text of step `n` (from 1) of a
//! tool is `hintstep.<tool id>.<n>`; most steps also have a small picture
//! of what the step does: the object, its handles and nodes, the pointer
//! and the drag. The pictures are drawn with the painter on a white page
//! in a 200 by 100 unit box, so they stay sharp at any panel width.

use crate::tools::Tool;
use egui::epaint::{Mesh, PathShape, PathStroke};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use std::f32::consts::{PI, TAU};

/// A step's picture.
pub type Pic = fn(&K);

/// The picture box, in units.
pub const W: f32 = 200.0;
pub const H: f32 = 100.0;

macro_rules! p {
    ($f:ident) => {
        Some($f as Pic)
    };
}

/// The steps of `tool`'s page: one entry per `hintstep.<id>.<n>` text,
/// with its picture if it has one.
pub fn steps(tool: Tool) -> &'static [Option<Pic>] {
    match tool {
        Tool::Pick => &[
            p!(pick_select),
            p!(pick_move),
            p!(pick_rotate),
            p!(pick_marquee),
            None,
        ],
        Tool::FreeformPick => &[p!(lasso_inside), p!(lasso_touch), None],
        Tool::FreeTransform => &[p!(ft_rotate), p!(ft_reflect), p!(ft_scale), None],
        Tool::Shape => &[
            p!(shape_move_node),
            p!(shape_handles),
            p!(shape_bend),
            p!(shape_add_node),
            p!(shape_corners),
        ],
        Tool::Smooth => &[p!(smooth_drag), None],
        Tool::Smear => &[p!(smear_out), p!(smear_in), None],
        Tool::Twirl => &[p!(twirl_hold), None],
        Tool::AttractRepel => &[p!(attract), p!(repel), None],
        Tool::Smudge => &[p!(smudge_drag), None],
        Tool::Roughen => &[p!(roughen_drag), None],
        Tool::VariableOutline => &[p!(var_one), p!(var_more), None, None],
        Tool::Crop => &[p!(crop_area), p!(crop_adjust), p!(crop_done)],
        Tool::Knife => &[p!(knife_cut), p!(knife_apart), None],
        Tool::SegmentDelete => &[p!(segdel_click), p!(segdel_marquee)],
        Tool::Eraser => &[p!(eraser_drag), None, None],
        Tool::Zoom => &[p!(zoom_click), p!(zoom_marquee), None],
        Tool::Pan => &[p!(pan_drag), None],
        Tool::Freehand => &[
            p!(freehand_drag),
            p!(freehand_clicks),
            p!(freehand_continue),
            None,
        ],
        Tool::TwoPointLine => &[p!(line_drag), p!(line_angles)],
        Tool::Bezier => &[p!(bezier_corners), p!(bezier_smooth), p!(bezier_close)],
        Tool::Pen => &[p!(pen_corners), p!(pen_preview), p!(bezier_close)],
        Tool::BSpline => &[p!(bspline_points), p!(bspline_loop)],
        Tool::Polyline => &[p!(polyline_clicks), None],
        Tool::ThreePointCurve => &[p!(curve3_line), p!(curve3_bend)],
        Tool::BrushStrokes => &[p!(brush_modes), p!(brush_drag), None],
        Tool::PainterlyBrush => &[p!(painterly_drag), p!(painterly_outline), None],
        Tool::ShapeRecognition => &[p!(recognize_shapes), p!(recognize_line), None],
        Tool::Sketch => &[p!(sketch_one), p!(sketch_join)],
        Tool::Rectangle => &[p!(rect_drag), p!(rect_square), p!(rect_corners), None],
        Tool::ThreePointRectangle => &[p!(rect3_base), p!(rect3_height)],
        Tool::Ellipse => &[p!(ellipse_drag), p!(ellipse_circle), p!(ellipse_pie_arc)],
        Tool::ThreePointEllipse => &[p!(ellipse3_base), p!(ellipse3_height)],
        Tool::Polygon => &[p!(polygon_drag), p!(polygon_center), p!(polygon_nodes)],
        Tool::Star => &[p!(star_drag), p!(star_complex), None],
        Tool::Spiral => &[p!(spiral_drag), p!(spiral_kinds), None],
        Tool::CommonShapes => &[p!(common_set), p!(common_drag)],
        Tool::ActionLines => &[p!(action_kinds), p!(action_drag)],
        Tool::GraphPaper => &[p!(graph_rows), p!(graph_drag), p!(graph_cells)],
        Tool::Text => &[p!(text_artistic), p!(text_paragraph), None],
        Tool::Table => &[p!(table_drag), p!(table_type), None],
        Tool::ParallelDimension => &[p!(pdim_points), p!(pdim_place), None],
        Tool::HorizontalVerticalDimension => &[p!(hvdim_points), p!(hvdim_place)],
        Tool::AngularDimension => &[p!(adim_vertex), p!(adim_sides), p!(adim_place)],
        Tool::SegmentDimension => &[p!(sdim_click), p!(sdim_place)],
        Tool::Callout => &[p!(callout_leader), p!(callout_text), None],
        Tool::Connector => &[p!(conn_drag), p!(conn_follow)],
        Tool::RightAngleConnector => &[p!(rconn_drag), p!(rconn_follow)],
        Tool::RoundedConnector => &[p!(rnd_conn_drag), p!(rnd_conn_radius)],
        Tool::AnchorEditing => &[p!(anchors_show), p!(anchors_add), None],
        Tool::DropShadow => &[
            p!(shadow_flat),
            p!(shadow_perspective),
            p!(shadow_opacity),
            None,
        ],
        Tool::Contour => &[p!(contour_out), p!(contour_in), None],
        Tool::Blend => &[p!(blend_drag), p!(blend_steps)],
        Tool::Distort => &[
            p!(distort_push),
            p!(distort_zipper),
            p!(distort_twister),
            p!(distort_center),
        ],
        Tool::Envelope => &[p!(envelope_frame), p!(envelope_bend), None],
        Tool::Extrude => &[p!(extrude_drag), p!(extrude_vp), None],
        Tool::BlockShadow => &[p!(block_shadow_drag), None],
        Tool::Transparency => &[p!(transp_uniform), p!(transp_gradient), p!(transp_slider)],
        Tool::ColorEyedropper => &[p!(eyedrop_sample), p!(eyedrop_apply), None],
        Tool::AttributesEyedropper => &[p!(attrs_sample), p!(attrs_apply), None],
        Tool::InteractiveFill => &[
            p!(fill_drag),
            p!(fill_slider),
            p!(fill_end),
            p!(fill_add_node),
        ],
        Tool::AreaFill => &[p!(area_click), p!(area_filled), None],
        Tool::MeshFill => &[p!(mesh_add), p!(mesh_color), p!(mesh_bend), None],
        Tool::OutlinePen => &[p!(pen_dialog), p!(pen_settings), None],
        Tool::OutlineColor => &[p!(outline_color_panel), p!(outline_color_palette)],
    }
}

/// Draw `pic` in `r`: a white page with a thin border, the picture on it.
pub fn draw(painter: &Painter, r: Rect, pic: Pic) {
    painter.rect_filled(r, 3.0, PAGE);
    painter.rect_stroke(
        r,
        3.0,
        Stroke::new(1.0, Color32::from_gray(0xCC)),
        egui::StrokeKind::Inside,
    );
    let s = (r.width() / W).min(r.height() / H);
    let o = r.center() - Vec2::new(W, H) * s / 2.0;
    let clip = painter.with_clip_rect(r.shrink(1.0));
    pic(&K { p: &clip, o, s, r });
}

// Colors of the pictures: they sit on a white page in every theme.
const PAGE: Color32 = Color32::WHITE;
const INK: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
const GHOST: Color32 = Color32::from_rgb(0x9A, 0x9A, 0x9A);
const ACCENT: Color32 = Color32::from_rgb(0x1E, 0x88, 0xE5);
const BLUE: Color32 = Color32::from_rgb(0x2F, 0x8F, 0xEA);
const RED: Color32 = Color32::from_rgb(0xE8, 0x3E, 0x52);
const YELLOW: Color32 = Color32::from_rgb(0xF6, 0xC3, 0x44);
const GREEN: Color32 = Color32::from_rgb(0x3F, 0xAE, 0x74);
const PURPLE: Color32 = Color32::from_rgb(0x8E, 0x6C, 0xD8);
const ORANGE: Color32 = Color32::from_rgb(0xF2, 0x8C, 0x28);
const LIGHT: Color32 = Color32::from_rgb(0xDD, 0xEB, 0xF7);

/// How a shape is filled.
#[derive(Clone, Copy)]
enum Paint {
    None,
    Solid(Color32),
    /// From color `c0` at `a` to `c1` at `b`.
    Linear((f32, f32), (f32, f32), Color32, Color32),
}

impl Paint {
    fn at(self, x: f32, y: f32) -> Color32 {
        match self {
            Paint::None => Color32::TRANSPARENT,
            Paint::Solid(c) => c,
            Paint::Linear(a, b, c0, c1) => {
                let d = (b.0 - a.0, b.1 - a.1);
                let len2 = (d.0 * d.0 + d.1 * d.1).max(1e-6);
                let t = ((x - a.0) * d.0 + (y - a.1) * d.1) / len2;
                lerp(c0, c1, t)
            }
        }
    }
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let (a, b) = (a.to_srgba_unmultiplied(), b.to_srgba_unmultiplied());
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(m(0), m(1), m(2), m(3))
}

fn alpha(c: Color32, a: u8) -> Color32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

type P = (f32, f32);

/// The picture kit: unit coordinates mapped onto the screen.
pub struct K<'a> {
    p: &'a Painter,
    o: Pos2,
    s: f32,
    /// The picture's page, to keep the pointer's tool icon on it.
    r: Rect,
}

impl K<'_> {
    fn pt(&self, (x, y): P) -> Pos2 {
        self.o + Vec2::new(x, y) * self.s
    }

    fn pts(&self, v: &[P]) -> Vec<Pos2> {
        v.iter().map(|&q| self.pt(q)).collect()
    }

    /// A size in pixels that grows a little with the picture.
    fn px(&self, v: f32) -> f32 {
        v * self.s.clamp(0.9, 1.6)
    }

    /// Fill the outline `v` (seen whole from its centroid, as every shape
    /// here is) with `paint` and stroke it in ink.
    fn shape(&self, v: &[P], paint: Paint) {
        self.fill(v, paint);
        self.outline(v, true, 1.0, INK);
    }

    fn fill(&self, v: &[P], paint: Paint) {
        if matches!(paint, Paint::None) || v.len() < 3 {
            return;
        }
        let v = subdivide(v, 4.0);
        let n = v.len();
        let c = v.iter().fold((0.0, 0.0), |a, q| (a.0 + q.0, a.1 + q.1));
        let c = (c.0 / n as f32, c.1 / n as f32);
        let rings = 6;
        let mut mesh = Mesh::default();
        mesh.colored_vertex(self.pt(c), paint.at(c.0, c.1));
        for j in 1..=rings {
            let t = j as f32 / rings as f32;
            for q in &v {
                let x = c.0 + (q.0 - c.0) * t;
                let y = c.1 + (q.1 - c.1) * t;
                mesh.colored_vertex(self.pt((x, y)), paint.at(x, y));
            }
        }
        let idx = |j: usize, i: usize| (1 + (j - 1) * n + i % n) as u32;
        for i in 0..n {
            mesh.add_triangle(0, idx(1, i), idx(1, i + 1));
            for j in 1..rings {
                mesh.add_triangle(idx(j, i), idx(j + 1, i), idx(j + 1, i + 1));
                mesh.add_triangle(idx(j, i), idx(j + 1, i + 1), idx(j, i + 1));
            }
        }
        self.p.add(Shape::mesh(mesh));
    }

    fn outline(&self, v: &[P], closed: bool, width: f32, color: Color32) {
        let pts = self.pts(v);
        let stroke = PathStroke::new(self.px(width), color);
        self.p.add(if closed {
            PathShape::closed_line(pts, stroke)
        } else {
            PathShape::line(pts, stroke)
        });
    }

    fn line(&self, v: &[P], width: f32, color: Color32) {
        self.outline(v, false, width, color);
    }

    fn dashed(&self, v: &[P], color: Color32) {
        let pts = self.pts(v);
        self.p.extend(Shape::dashed_line(
            &pts,
            Stroke::new(1.0, color),
            self.px(4.0),
            self.px(3.0),
        ));
    }

    fn dashed_closed(&self, v: &[P], color: Color32) {
        let mut v = v.to_vec();
        if let Some(&f) = v.first() {
            v.push(f);
        }
        self.dashed(&v, color);
    }

    /// The eight selection handles around a box, and the cross in its
    /// middle.
    fn handles(&self, (x0, y0): P, (x1, y1): P) {
        let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        for q in [
            (x0, y0),
            (mx, y0),
            (x1, y0),
            (x0, my),
            (x1, my),
            (x0, y1),
            (mx, y1),
            (x1, y1),
        ] {
            let r = Rect::from_center_size(self.pt(q), Vec2::splat(self.px(5.0)));
            self.p.rect_filled(r, 0.0, INK);
        }
        let c = self.pt((mx, my));
        let d = self.px(3.0);
        let s = Stroke::new(1.0, INK);
        self.p
            .line_segment([c - Vec2::splat(d), c + Vec2::splat(d)], s);
        self.p
            .line_segment([c + Vec2::new(-d, d), c + Vec2::new(d, -d)], s);
    }

    /// A node square; selected nodes are filled.
    fn node(&self, q: P, selected: bool) {
        let r = Rect::from_center_size(self.pt(q), Vec2::splat(self.px(5.0)));
        if selected {
            self.p.rect_filled(r, 0.0, ACCENT);
        } else {
            self.p.rect_filled(r, 0.0, PAGE);
        }
        self.p
            .rect_stroke(r, 0.0, Stroke::new(1.0, INK), egui::StrokeKind::Middle);
    }

    fn nodes(&self, v: &[P]) {
        for &q in v {
            self.node(q, false);
        }
    }

    /// A control handle from a node: a dashed line ending in a dot.
    fn ctrl(&self, from: P, to: P) {
        self.dashed(&[from, to], ACCENT);
        self.p.circle_filled(self.pt(to), self.px(2.6), ACCENT);
    }

    /// The pointer arrow with its tip at `q`.
    fn cursor(&self, q: P) {
        let k = self.px(1.0);
        // The whole arrow stays on the page.
        let o = self.pt(q);
        let o = Pos2::new(
            o.x.min(self.r.max.x - 13.0 * k),
            o.y.min(self.r.max.y - 21.0 * k),
        );
        let at = |x: f32, y: f32| o + Vec2::new(x, y) * k;
        let head = vec![at(0.0, 0.0), at(0.0, 16.0), at(11.5, 11.5)];
        let tail = vec![at(3.6, 12.6), at(6.6, 19.0), at(9.2, 17.9), at(6.3, 11.7)];
        self.p
            .add(PathShape::convex_polygon(head, INK, Stroke::NONE));
        self.p
            .add(PathShape::convex_polygon(tail, INK, Stroke::NONE));
        let edge = vec![
            at(0.0, 0.0),
            at(0.0, 16.0),
            at(3.6, 12.6),
            at(6.6, 19.0),
            at(9.2, 17.9),
            at(6.3, 11.7),
            at(11.5, 11.5),
        ];
        self.p
            .add(PathShape::closed_line(edge, Stroke::new(1.0, PAGE)));
    }

    /// The pointer with the tool's icon beside it.
    fn tool_cursor(&self, q: P, tool: Tool) {
        self.cursor(q);
        let size = self.px(14.0);
        let k = self.px(1.0);
        let tip = self.pt(q);
        let tip = Pos2::new(
            tip.x.min(self.r.max.x - 13.0 * k),
            tip.y.min(self.r.max.y - 21.0 * k),
        );
        let mut o = tip + Vec2::new(self.px(9.0), self.px(13.0));
        // Beside the arrow where the page ends below or to the right.
        if o.y + size > self.r.max.y - 2.0 {
            o.y = tip.y - size - self.px(2.0);
        }
        if o.x + size > self.r.max.x - 2.0 {
            o.x = tip.x - size - self.px(4.0);
        }
        let r = Rect::from_min_size(o, Vec2::splat(size));
        self.p.rect_filled(r.expand(1.0), 2.0, alpha(PAGE, 220));
        crate::ui::icons::draw(self.p, r, tool, INK);
    }

    /// A drag from `a` to `b`: a dashed accent line with an arrowhead.
    fn drag(&self, a: P, b: P) {
        self.drag_path(&[a, b]);
    }

    fn drag_path(&self, v: &[P]) {
        if v.len() < 2 {
            return;
        }
        self.dashed(v, ACCENT);
        let b = self.pt(v[v.len() - 1]);
        let a = self.pt(v[v.len() - 2]);
        self.arrowhead(a, b, ACCENT);
    }

    fn arrowhead(&self, a: Pos2, b: Pos2, color: Color32) {
        let d = (b - a).normalized();
        if !d.x.is_finite() {
            return;
        }
        let n = Vec2::new(-d.y, d.x);
        let l = self.px(7.0);
        let w = self.px(3.5);
        self.p.add(PathShape::convex_polygon(
            vec![b, b - d * l + n * w, b - d * l - n * w],
            color,
            Stroke::NONE,
        ));
    }

    /// Ripples where a click happens.
    fn click(&self, q: P) {
        let c = self.pt(q);
        self.p
            .circle_stroke(c, self.px(5.0), Stroke::new(1.2, ACCENT));
        self.p
            .circle_stroke(c, self.px(9.0), Stroke::new(1.0, alpha(ACCENT, 110)));
    }

    /// The round brush of the shaping tools.
    fn nib(&self, q: P, r: f32) {
        let c = self.pt(q);
        self.p.circle_filled(c, r * self.s, alpha(ACCENT, 28));
        self.p
            .circle_stroke(c, r * self.s, Stroke::new(1.0, ACCENT));
    }

    fn text(&self, q: P, s: &str, size: f32, color: Color32, align: Align2) {
        self.p.text(
            self.pt(q),
            align,
            s,
            FontId::proportional(size * self.s),
            color,
        );
    }

    /// The handles of a gradient: start and end squares in their colors,
    /// the line between them and the midpoint slider at `mid` (0 to 1).
    fn vector(&self, a: P, b: P, c0: Color32, c1: Color32, mid: f32) {
        let pa = self.pt(a);
        let pb = self.pt(b);
        self.p
            .line_segment([pa, pb], Stroke::new(self.px(2.5), PAGE));
        self.p.line_segment([pa, pb], Stroke::new(1.0, INK));
        self.arrowhead(pa, pb - (pb - pa).normalized() * self.px(6.0), INK);
        for (q, c) in [(pa, c0), (pb, c1)] {
            let r = Rect::from_center_size(q, Vec2::splat(self.px(10.0)));
            self.p.rect_filled(r, 0.0, c);
            self.p
                .rect_stroke(r, 0.0, Stroke::new(1.5, PAGE), egui::StrokeKind::Inside);
            self.p
                .rect_stroke(r, 0.0, Stroke::new(1.0, INK), egui::StrokeKind::Outside);
        }
        let m = pa + (pb - pa) * mid;
        let d = (pb - pa).normalized();
        let n = Vec2::new(-d.y, d.x);
        let bar = [m + n * self.px(7.0), m - n * self.px(7.0)];
        self.p.line_segment(bar, Stroke::new(self.px(4.0), INK));
        self.p.line_segment(bar, Stroke::new(self.px(2.0), PAGE));
    }

    /// A curved double arrow around `c` from angle `a0` to `a1`.
    fn curved_arrow(&self, c: P, r: f32, a0: f32, a1: f32) {
        let arc: Vec<P> = (0..=8)
            .map(|j| {
                let a = a0 + (a1 - a0) * j as f32 / 8.0;
                (c.0 + r * a.cos(), c.1 + r * a.sin())
            })
            .collect();
        self.line(&arc, 1.4, INK);
        self.arrowhead(self.pt(arc[1]), self.pt(arc[0]), INK);
        self.arrowhead(self.pt(arc[7]), self.pt(arc[8]), INK);
    }

    /// A diamond marker (anchors, effect centers).
    fn diamond(&self, q: P, fill: Color32) {
        let c = self.pt(q);
        let d = self.px(4.5);
        let v = vec![
            c + Vec2::new(0.0, -d),
            c + Vec2::new(d, 0.0),
            c + Vec2::new(0.0, d),
            c + Vec2::new(-d, 0.0),
        ];
        self.p
            .add(PathShape::convex_polygon(v, fill, Stroke::new(1.0, INK)));
    }
}

// ---- Geometry helpers ----

fn subdivide(v: &[P], step: f32) -> Vec<P> {
    let mut out = Vec::with_capacity(v.len() * 2);
    for i in 0..v.len() {
        let a = v[i];
        let b = v[(i + 1) % v.len()];
        let n = (((b.0 - a.0).hypot(b.1 - a.1)) / step).ceil().max(1.0) as usize;
        for k in 0..n {
            let t = k as f32 / n as f32;
            out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    }
    out
}

fn rect_pts(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<P> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn ellipse_pts(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<P> {
    (0..64)
        .map(|i| {
            let a = i as f32 / 64.0 * TAU;
            (cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

/// A star (or with `inner == outer`, a polygon of `n * 2` corners) with
/// its first point at the top.
fn star_pts(cx: f32, cy: f32, outer: f32, inner: f32, n: usize) -> Vec<P> {
    (0..n * 2)
        .map(|i| {
            let a = -PI / 2.0 + i as f32 * PI / n as f32;
            let r = if i % 2 == 0 { outer } else { inner };
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

fn polygon_pts(cx: f32, cy: f32, r: f32, n: usize) -> Vec<P> {
    (0..n)
        .map(|i| {
            let a = -PI / 2.0 + i as f32 * TAU / n as f32;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// A soft rounded blob, the "object" of the shaping pictures.
fn blob(cx: f32, cy: f32, r: f32) -> Vec<P> {
    (0..72)
        .map(|i| {
            let a = i as f32 / 72.0 * TAU;
            let k = 1.0 + 0.08 * (2.0 * a).sin() + 0.05 * (3.0 * a + 1.0).cos();
            (cx + r * 1.25 * k * a.cos(), cy + r * k * a.sin())
        })
        .collect()
}

fn rotate(v: &[P], c: P, deg: f32) -> Vec<P> {
    let (s, co) = deg.to_radians().sin_cos();
    v.iter()
        .map(|&(x, y)| {
            let (dx, dy) = (x - c.0, y - c.1);
            (c.0 + dx * co - dy * s, c.1 + dx * s + dy * co)
        })
        .collect()
}

fn translate(v: &[P], dx: f32, dy: f32) -> Vec<P> {
    v.iter().map(|&(x, y)| (x + dx, y + dy)).collect()
}

/// Points of a cubic Bezier segment.
fn cubic(a: P, b: P, c: P, d: P) -> Vec<P> {
    (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            (
                w[0] * a.0 + w[1] * b.0 + w[2] * c.0 + w[3] * d.0,
                w[0] * a.1 + w[1] * b.1 + w[2] * c.1 + w[3] * d.1,
            )
        })
        .collect()
}

/// Points of a quadratic curve through `a`, bent toward `c`, ending at `b`.
fn quad(a: P, c: P, b: P) -> Vec<P> {
    (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let u = 1.0 - t;
            (
                u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0,
                u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1,
            )
        })
        .collect()
}

/// A wave from `x0` to `x1` around `y`.
fn wave(x0: f32, x1: f32, y: f32, amp: f32, turns: f32) -> Vec<P> {
    (0..=48)
        .map(|i| {
            let t = i as f32 / 48.0;
            (x0 + (x1 - x0) * t, y + amp * (t * turns * TAU).sin())
        })
        .collect()
}

/// A closed uniform B-spline through the control points `c` (open when
/// `closed` is false, clamped at the ends).
fn bspline(c: &[P], closed: bool) -> Vec<P> {
    let n = c.len();
    let get = |i: isize| -> P {
        if closed {
            c[i.rem_euclid(n as isize) as usize]
        } else {
            c[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let mut out = Vec::new();
    let (from, to) = if closed {
        (0, n as isize)
    } else {
        (-2, n as isize - 1)
    };
    for i in from..to {
        let (p0, p1, p2, p3) = (get(i), get(i + 1), get(i + 2), get(i + 3));
        for k in 0..12 {
            let t = k as f32 / 12.0;
            let t2 = t * t;
            let t3 = t2 * t;
            let b0 = (1.0 - t).powi(3) / 6.0;
            let b1 = (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0;
            let b2 = (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0;
            let b3 = t3 / 6.0;
            out.push((
                b0 * p0.0 + b1 * p1.0 + b2 * p2.0 + b3 * p3.0,
                b0 * p0.1 + b1 * p1.1 + b2 * p2.1 + b3 * p3.1,
            ));
        }
    }
    if !closed {
        out.push(c[n - 1]);
    }
    out
}

/// Move the points of `v` near `at` by `d`, falling off over `r`.
fn push(v: &[P], at: P, d: (f32, f32), r: f32) -> Vec<P> {
    v.iter()
        .map(|&(x, y)| {
            let dist = (x - at.0).hypot(y - at.1);
            let w = (1.0 - dist / r).clamp(0.0, 1.0);
            let w = w * w * (3.0 - 2.0 * w);
            (x + d.0 * w, y + d.1 * w)
        })
        .collect()
}

fn bbox(v: &[P]) -> (P, P) {
    let mut a = (f32::MAX, f32::MAX);
    let mut b = (f32::MIN, f32::MIN);
    for &(x, y) in v {
        a = (a.0.min(x), a.1.min(y));
        b = (b.0.max(x), b.1.max(y));
    }
    (a, b)
}

fn grad(c0: Color32, c1: Color32, a: P, b: P) -> Paint {
    Paint::Linear(a, b, c0, c1)
}

// ---- Pick, Freehand Pick, Free Transform ----

fn pick_select(k: &K) {
    let r = rect_pts(70.0, 25.0, 130.0, 75.0);
    k.shape(&r, Paint::Solid(BLUE));
    k.handles((70.0, 25.0), (130.0, 75.0));
    k.click((112.0, 52.0));
    k.cursor((112.0, 52.0));
}

fn pick_move(k: &K) {
    k.dashed_closed(&rect_pts(30.0, 30.0, 80.0, 70.0), GHOST);
    let r = rect_pts(105.0, 22.0, 155.0, 62.0);
    k.shape(&r, Paint::Solid(BLUE));
    k.handles((105.0, 22.0), (155.0, 62.0));
    k.drag((58.0, 52.0), (128.0, 44.0));
    k.cursor((130.0, 44.0));
    // A corner handle pulled out.
    k.dashed_closed(&rect_pts(105.0, 22.0, 172.0, 82.0), ACCENT);
    k.drag((157.0, 64.0), (171.0, 81.0));
}

fn pick_rotate(k: &K) {
    let c = (100.0, 50.0);
    let box_ = rect_pts(72.0, 30.0, 128.0, 70.0);
    k.dashed_closed(&box_, GHOST);
    k.shape(&rotate(&box_, c, -15.0), Paint::Solid(BLUE));
    // Curved double arrows at the corners, straight ones on the sides.
    let corners = rotate(
        &[(64.0, 22.0), (136.0, 22.0), (136.0, 78.0), (64.0, 78.0)],
        c,
        -15.0,
    );
    for &q in &corners {
        let out = (q.1 - c.1).atan2(q.0 - c.0);
        k.curved_arrow(
            (q.0 - 5.0 * out.cos(), q.1 - 5.0 * out.sin()),
            7.0,
            out - 0.8,
            out + 0.8,
        );
    }
    for (a, b) in [
        ((92.0, 21.0), (108.0, 21.0)),
        ((92.0, 79.0), (108.0, 79.0)),
        ((63.0, 43.0), (63.0, 57.0)),
        ((137.0, 43.0), (137.0, 57.0)),
    ] {
        let v = rotate(&[a, b], c, -15.0);
        k.line(&v, 1.4, INK);
        k.arrowhead(k.pt(v[0]), k.pt(v[1]), INK);
        k.arrowhead(k.pt(v[1]), k.pt(v[0]), INK);
    }
    k.p.circle_stroke(k.pt(c), k.px(4.0), Stroke::new(1.0, INK));
    k.p.circle_filled(k.pt(c), k.px(1.2), INK);
    let arc: Vec<P> = (0..=10)
        .map(|j| {
            let a = -0.95 + 0.55 * j as f32 / 10.0;
            (c.0 + 55.0 * a.cos(), c.1 + 55.0 * a.sin())
        })
        .collect();
    k.drag_path(&arc);
    k.cursor(arc[10]);
}

fn pick_marquee(k: &K) {
    k.shape(&ellipse_pts(70.0, 50.0, 18.0, 18.0), Paint::Solid(RED));
    k.shape(&rect_pts(98.0, 36.0, 128.0, 64.0), Paint::Solid(YELLOW));
    k.shape(&star_pts(165.0, 50.0, 16.0, 7.0, 5), Paint::Solid(GREEN));
    k.dashed_closed(&rect_pts(44.0, 22.0, 138.0, 80.0), INK);
    k.handles((52.0, 32.0), (128.0, 68.0));
    k.drag((44.0, 22.0), (136.0, 78.0));
    k.cursor((138.0, 80.0));
}

fn lasso() -> Vec<P> {
    (0..48)
        .map(|i| {
            let a = i as f32 / 48.0 * TAU;
            let w = 1.0 + 0.06 * (5.0 * a).sin();
            (95.0 + 62.0 * w * a.cos(), 50.0 + 36.0 * w * a.sin())
        })
        .collect()
}

fn lasso_inside(k: &K) {
    k.shape(&ellipse_pts(72.0, 50.0, 15.0, 15.0), Paint::Solid(RED));
    k.shape(&star_pts(112.0, 50.0, 16.0, 7.0, 5), Paint::Solid(GREEN));
    k.shape(&rect_pts(166.0, 35.0, 192.0, 65.0), Paint::Solid(YELLOW));
    let l = lasso();
    k.dashed(&l[..44], INK);
    k.handles((57.0, 34.0), (128.0, 65.0));
    k.cursor(l[44]);
}

fn lasso_touch(k: &K) {
    k.shape(&ellipse_pts(72.0, 50.0, 15.0, 15.0), Paint::Solid(RED));
    k.shape(&rect_pts(140.0, 32.0, 172.0, 68.0), Paint::Solid(YELLOW));
    let l = lasso();
    k.dashed_closed(&l, INK);
    k.handles((57.0, 32.0), (172.0, 68.0));
    k.text((100.0, 92.0), "Alt", 10.0, ACCENT, Align2::CENTER_CENTER);
    k.cursor(l[40]);
}

fn ft_rotate(k: &K) {
    let r = rect_pts(70.0, 30.0, 130.0, 70.0);
    k.dashed_closed(&r, GHOST);
    let press = (70.0, 70.0);
    k.shape(&rotate(&r, press, -25.0), Paint::Solid(BLUE));
    k.p.circle_stroke(k.pt(press), k.px(4.0), Stroke::new(1.2, ACCENT));
    let arc: Vec<P> = (0..=10)
        .map(|j| {
            let a = -0.62 - 0.45 * j as f32 / 10.0;
            (press.0 + 72.0 * a.cos(), press.1 + 72.0 * a.sin())
        })
        .collect();
    k.drag_path(&arc);
    k.tool_cursor(arc[10], Tool::FreeTransform);
}

fn ft_reflect(k: &K) {
    let r = vec![(60.0, 30.0), (95.0, 40.0), (95.0, 70.0), (50.0, 70.0)];
    k.shape(&r, Paint::Solid(BLUE));
    let m: Vec<P> = r.iter().map(|&(x, y)| (200.0 - x, y)).collect();
    k.shape(&m, Paint::Solid(alpha(BLUE, 140)));
    k.dashed(&[(100.0, 10.0), (100.0, 92.0)], ACCENT);
    k.p.circle_stroke(k.pt((100.0, 88.0)), k.px(4.0), Stroke::new(1.2, ACCENT));
    k.tool_cursor((100.0, 14.0), Tool::FreeTransform);
}

fn ft_scale(k: &K) {
    let press = (50.0, 80.0);
    k.dashed_closed(&rect_pts(50.0, 50.0, 90.0, 80.0), GHOST);
    k.shape(&rect_pts(50.0, 20.0, 130.0, 80.0), Paint::Solid(BLUE));
    k.p.circle_stroke(k.pt(press), k.px(4.0), Stroke::new(1.2, ACCENT));
    k.drag((90.0, 50.0), (130.0, 20.0));
    k.tool_cursor((131.0, 20.0), Tool::FreeTransform);
    // Skew beside it.
    let sk = vec![(150.0, 40.0), (190.0, 40.0), (180.0, 80.0), (140.0, 80.0)];
    k.shape(&sk, Paint::Solid(PURPLE));
}

// ---- Shape tool ----

fn shape_curve() -> [P; 4] {
    [(40.0, 70.0), (85.0, 25.0), (130.0, 70.0), (170.0, 35.0)]
}

fn smooth_through(v: &[P]) -> Vec<P> {
    // Catmull-Rom through the points.
    let mut out = Vec::new();
    for i in 0..v.len() - 1 {
        let p0 = v[i.saturating_sub(1)];
        let p1 = v[i];
        let p2 = v[i + 1];
        let p3 = v[(i + 2).min(v.len() - 1)];
        let b = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
        let c = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
        let mut seg = cubic(p1, b, c, p2);
        if i > 0 {
            seg.remove(0);
        }
        out.extend(seg);
    }
    out
}

fn shape_move_node(k: &K) {
    let n = shape_curve();
    k.dashed(&smooth_through(&n), GHOST);
    let mut m = n;
    m[1] = (95.0, 12.0);
    k.line(&smooth_through(&m), 1.5, INK);
    k.nodes(&[m[0], m[2], m[3]]);
    k.node(m[1], true);
    k.drag((85.0, 25.0), (94.0, 14.0));
    k.tool_cursor((97.0, 14.0), Tool::Shape);
}

fn shape_handles(k: &K) {
    let a = (40.0, 75.0);
    let b = (100.0, 40.0);
    let c = (165.0, 70.0);
    let h0 = (70.0, 40.0);
    let h1 = (130.0, 40.0);
    let mut v = cubic(a, (45.0, 50.0), (70.0, 40.0), b);
    v.extend(cubic(b, (135.0, 40.0), (160.0, 50.0), c));
    k.line(&v, 1.5, INK);
    k.ctrl(b, h0);
    k.ctrl(b, h1);
    k.nodes(&[a, c]);
    k.node(b, true);
    k.drag(h1, (140.0, 22.0));
    k.tool_cursor((141.0, 21.0), Tool::Shape);
}

fn shape_bend(k: &K) {
    let a = (40.0, 70.0);
    let b = (160.0, 70.0);
    k.dashed(&[a, b], GHOST);
    k.line(&quad(a, (100.0, 0.0), b), 1.5, INK);
    k.nodes(&[a, b]);
    k.drag((100.0, 68.0), (100.0, 38.0));
    k.tool_cursor((100.0, 36.0), Tool::Shape);
}

fn shape_add_node(k: &K) {
    let n = shape_curve();
    let v = smooth_through(&n);
    k.line(&v, 1.5, INK);
    k.nodes(&n);
    let at = v[36];
    k.node(at, true);
    k.click(at);
    k.click(at);
    k.tool_cursor((at.0 + 2.0, at.1 + 2.0), Tool::Shape);
}

fn shape_corners(k: &K) {
    // A rectangle whose corners were rounded by dragging a corner node.
    let (x0, y0, x1, y1, r) = (25.0, 25.0, 95.0, 78.0, 12.0);
    let mut v = Vec::new();
    for (cx, cy, a0) in [
        (x1 - r, y0 + r, -PI / 2.0),
        (x1 - r, y1 - r, 0.0),
        (x0 + r, y1 - r, PI / 2.0),
        (x0 + r, y0 + r, PI),
    ] {
        for j in 0..=8 {
            let a = a0 + PI / 2.0 * j as f32 / 8.0;
            v.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    k.dashed_closed(&rect_pts(x0, y0, x1, y1), GHOST);
    k.shape(&v, Paint::Solid(YELLOW));
    k.node((x0 + r, y0), true);
    k.drag((x0, y0), (x0 + r, y0));
    // A pie made from an ellipse.
    let (cx, cy, rr) = (150.0, 52.0, 26.0);
    let mut pie = vec![(cx, cy)];
    for j in 0..=40 {
        let a = 0.0 + (TAU - 1.2) * j as f32 / 40.0;
        pie.push((cx + rr * a.cos(), cy - rr * a.sin()));
    }
    k.shape(&pie, Paint::Solid(PURPLE));
    k.node((cx + rr * (1.2f32).cos(), cy + rr * (1.2f32).sin()), true);
    k.tool_cursor((cx + 14.0, cy + 6.0), Tool::Shape);
}

// ---- Shaping brushes ----

fn smooth_drag(k: &K) {
    let rough: Vec<P> = (0..=60)
        .map(|i| {
            let t = i as f32 / 60.0;
            let x = 20.0 + 160.0 * t;
            let jag = if x < 100.0 {
                4.0 * (t * 60.0).sin() * (t * 23.0).cos()
            } else {
                0.0
            };
            (x, 55.0 - 18.0 * (t * PI).sin() + jag)
        })
        .collect();
    k.line(&rough, 1.5, INK);
    k.nib((100.0, 37.0), 18.0);
    k.drag((60.0, 30.0), (150.0, 30.0));
    k.tool_cursor((100.0, 37.0), Tool::Smooth);
}

fn smear_out(k: &K) {
    let b = blob(80.0, 52.0, 30.0);
    let v = push(&b, (118.0, 50.0), (45.0, -5.0), 22.0);
    k.shape(&v, Paint::Solid(GREEN));
    k.nib((160.0, 45.0), 12.0);
    k.drag((112.0, 50.0), (160.0, 45.0));
    k.tool_cursor((160.0, 45.0), Tool::Smear);
}

fn smear_in(k: &K) {
    let b = blob(90.0, 52.0, 32.0);
    let v = push(&b, (130.0, 50.0), (-28.0, 0.0), 24.0);
    k.shape(&v, Paint::Solid(GREEN));
    k.nib((104.0, 50.0), 12.0);
    k.drag((160.0, 50.0), (106.0, 50.0));
    k.tool_cursor((104.0, 50.0), Tool::Smear);
}

fn twirl_hold(k: &K) {
    let b = blob(80.0, 52.0, 30.0);
    let c = (118.0, 50.0);
    let v: Vec<P> = b
        .iter()
        .map(|&(x, y)| {
            let d = (x - c.0).hypot(y - c.1);
            let w = (1.0 - d / 26.0).clamp(0.0, 1.0);
            let a = w * w * 4.5;
            let (s, co) = a.sin_cos();
            let (dx, dy) = (x - c.0, y - c.1);
            (c.0 + dx * co - dy * s, c.1 + dx * s + dy * co)
        })
        .collect();
    k.shape(&v, Paint::Solid(PURPLE));
    k.nib(c, 26.0);
    let arc: Vec<P> = (0..=12)
        .map(|j| {
            let a = -0.5 + 2.4 * j as f32 / 12.0;
            (c.0 + 34.0 * a.cos(), c.1 + 34.0 * a.sin())
        })
        .collect();
    k.drag_path(&arc);
    k.tool_cursor(c, Tool::Twirl);
}

fn attract(k: &K) {
    let b = blob(80.0, 52.0, 30.0);
    let c = (150.0, 52.0);
    let v: Vec<P> = b
        .iter()
        .map(|&(x, y)| {
            let d = (x - c.0).hypot(y - c.1);
            let w = (1.0 - d / 40.0).clamp(0.0, 1.0);
            let w = w * w * (3.0 - 2.0 * w);
            (x + (c.0 - x) * w * 0.8, y + (c.1 - y) * w * 0.8)
        })
        .collect();
    k.shape(&v, Paint::Solid(BLUE));
    k.nib(c, 24.0);
    for a in [-0.8f32, 0.0, 0.8] {
        let (s, co) = a.sin_cos();
        k.drag(
            (c.0 - 34.0 * co, c.1 + 34.0 * s),
            (c.0 - 16.0 * co, c.1 + 16.0 * s),
        );
    }
    k.tool_cursor(c, Tool::AttractRepel);
}

fn repel(k: &K) {
    let b = blob(90.0, 52.0, 30.0);
    let c = (110.0, 50.0);
    let v: Vec<P> = b
        .iter()
        .map(|&(x, y)| {
            let d = (x - c.0).hypot(y - c.1).max(1e-3);
            let w = (1.0 - d / 26.0).clamp(0.0, 1.0);
            (x + (x - c.0) / d * w * 14.0, y + (y - c.1) / d * w * 14.0)
        })
        .collect();
    k.shape(&v, Paint::Solid(BLUE));
    k.nib(c, 26.0);
    for a in [-0.9f32, 0.0, 0.9] {
        let (s, co) = a.sin_cos();
        k.drag(
            (c.0 + 8.0 * co, c.1 + 8.0 * s),
            (c.0 + 24.0 * co, c.1 + 24.0 * s),
        );
    }
    k.text((100.0, 92.0), "Alt", 10.0, ACCENT, Align2::CENTER_CENTER);
    k.tool_cursor(c, Tool::AttractRepel);
}

fn smudge_drag(k: &K) {
    let b = blob(80.0, 52.0, 30.0);
    let v = push(&b, (116.0, 52.0), (40.0, 14.0), 30.0);
    k.shape(&v, Paint::Solid(ORANGE));
    k.nib((156.0, 66.0), 12.0);
    k.drag_path(&[(112.0, 52.0), (136.0, 62.0), (156.0, 66.0)]);
    k.tool_cursor((156.0, 66.0), Tool::Smudge);
}

fn roughen_drag(k: &K) {
    let b = ellipse_pts(100.0, 52.0, 60.0, 34.0);
    let v: Vec<P> = subdivide(&b, 2.0)
        .into_iter()
        .enumerate()
        .map(|(i, (x, y))| {
            if y < 40.0 && x > 70.0 && x < 140.0 {
                let d = if i % 2 == 0 { 5.0 } else { 0.0 };
                (x, y - d)
            } else {
                (x, y)
            }
        })
        .collect();
    k.shape(&v, Paint::Solid(YELLOW));
    k.nib((140.0, 24.0), 10.0);
    k.drag_path(&[(70.0, 24.0), (100.0, 16.0), (140.0, 24.0)]);
    k.tool_cursor((140.0, 24.0), Tool::Roughen);
}

fn var_band(k: &K, widths: &[(f32, f32)]) {
    // A path and its width along it from (t, width) pairs.
    let path: Vec<P> = (0..=60)
        .map(|i| {
            let t = i as f32 / 60.0;
            (20.0 + 160.0 * t, 55.0 - 22.0 * (t * PI * 1.2).sin())
        })
        .collect();
    let width_at = |t: f32| -> f32 {
        let mut w = 1.0;
        for &(tt, ww) in widths {
            let d = (t - tt).abs();
            w += ww * (1.0 - d / 0.3).clamp(0.0, 1.0);
        }
        w
    };
    let mut left = Vec::new();
    let mut right = Vec::new();
    for i in 0..path.len() {
        let a = path[i.saturating_sub(1)];
        let b = path[(i + 1).min(path.len() - 1)];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dy).max(1e-3);
        let n = (-dy / l, dx / l);
        let w = width_at(i as f32 / 60.0) / 2.0;
        left.push((path[i].0 + n.0 * w, path[i].1 + n.1 * w));
        right.push((path[i].0 - n.0 * w, path[i].1 - n.1 * w));
    }
    // Fill the band quad by quad.
    let mut mesh = Mesh::default();
    for i in 0..path.len() {
        mesh.colored_vertex(k.pt(left[i]), INK);
        mesh.colored_vertex(k.pt(right[i]), INK);
    }
    for i in 0..path.len() as u32 - 1 {
        mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
        mesh.add_triangle(2 * i + 1, 2 * i + 3, 2 * i + 2);
    }
    k.p.add(Shape::mesh(mesh));
    for &(t, _) in widths {
        let i = (t * 60.0).round() as usize;
        k.p.circle_filled(k.pt(path[i]), k.px(3.0), ACCENT);
        k.p.circle_stroke(k.pt(path[i]), k.px(3.0), Stroke::new(1.0, PAGE));
    }
}

fn var_one(k: &K) {
    var_band(k, &[(0.45, 14.0)]);
    k.drag((96.0, 32.0), (110.0, 14.0));
    k.tool_cursor((111.0, 13.0), Tool::VariableOutline);
}

fn var_more(k: &K) {
    var_band(k, &[(0.2, 9.0), (0.5, 3.0), (0.8, 15.0)]);
}

// ---- Crop, Knife, Segment Delete, Eraser ----

fn crop_scene(k: &K) {
    k.shape(&ellipse_pts(60.0, 50.0, 34.0, 30.0), Paint::Solid(BLUE));
    k.shape(&star_pts(130.0, 50.0, 32.0, 14.0, 5), Paint::Solid(YELLOW));
}

fn dim_outside(k: &K, x0: f32, y0: f32, x1: f32, y1: f32) {
    let c = Color32::from_rgba_unmultiplied(0, 0, 0, 70);
    for (a, b) in [
        ((0.0, 0.0), (W, y0)),
        ((0.0, y1), (W, H)),
        ((0.0, y0), (x0, y1)),
        ((x1, y0), (W, y1)),
    ] {
        k.p.rect_filled(Rect::from_two_pos(k.pt(a), k.pt(b)), 0.0, c);
    }
    k.dashed_closed(&rect_pts(x0, y0, x1, y1), INK);
    k.handles((x0, y0), (x1, y1));
}

fn crop_area(k: &K) {
    crop_scene(k);
    dim_outside(k, 45.0, 22.0, 140.0, 78.0);
    k.drag((45.0, 22.0), (138.0, 76.0));
    k.tool_cursor((140.0, 78.0), Tool::Crop);
}

fn crop_adjust(k: &K) {
    crop_scene(k);
    dim_outside(k, 60.0, 22.0, 155.0, 78.0);
    k.drag((95.0, 50.0), (110.0, 50.0));
    k.cursor((112.0, 50.0));
}

fn crop_done(k: &K) {
    let clip =
        k.p.with_clip_rect(Rect::from_two_pos(k.pt((60.0, 22.0)), k.pt((155.0, 78.0))));
    let kk = K {
        p: &clip,
        o: k.o,
        s: k.s,
        r: k.r,
    };
    crop_scene(&kk);
    k.dashed_closed(&rect_pts(60.0, 22.0, 155.0, 78.0), GHOST);
    k.click((110.0, 50.0));
    k.click((110.0, 50.0));
    k.cursor((110.0, 50.0));
}

fn knife_cut(k: &K) {
    k.shape(&ellipse_pts(100.0, 50.0, 50.0, 32.0), Paint::Solid(RED));
    k.line(&[(70.0, 8.0), (130.0, 92.0)], 1.0, PAGE);
    k.drag((70.0, 8.0), (120.0, 78.0));
    k.tool_cursor((122.0, 80.0), Tool::Knife);
}

fn knife_apart(k: &K) {
    let e = ellipse_pts(100.0, 50.0, 50.0, 32.0);
    // Points on the left of the cut line and on its right.
    let side = |q: &P| (q.0 - 70.0) * 84.0 - (q.1 - 8.0) * 60.0;
    let cut = |v: &Vec<P>, keep_left: bool| -> Vec<P> {
        let mut out = Vec::new();
        for i in 0..v.len() {
            let a = v[i];
            let b = v[(i + 1) % v.len()];
            let (sa, sb) = (side(&a), side(&b));
            if (sa < 0.0) == keep_left {
                out.push(a);
            }
            if (sa < 0.0) != (sb < 0.0) {
                let t = sa / (sa - sb);
                out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            }
        }
        out
    };
    k.shape(&translate(&cut(&e, true), -14.0, 0.0), Paint::Solid(RED));
    k.shape(&translate(&cut(&e, false), 14.0, 0.0), Paint::Solid(RED));
    k.handles((108.0, 18.0), (164.0, 82.0));
}

fn crossing_lines(k: &K, cut: bool) {
    let a = [(20.0, 30.0), (180.0, 30.0)];
    let b = [(20.0, 70.0), (180.0, 70.0)];
    let c = [(60.0, 10.0), (60.0, 90.0)];
    let d = [(140.0, 10.0), (140.0, 90.0)];
    for l in [b, c, d] {
        k.line(&l, 1.5, INK);
    }
    if cut {
        k.line(&[(20.0, 30.0), (60.0, 30.0)], 1.5, INK);
        k.line(&[(140.0, 30.0), (180.0, 30.0)], 1.5, INK);
        k.dashed(&[(60.0, 30.0), (140.0, 30.0)], RED);
    } else {
        k.line(&a, 1.5, INK);
    }
}

fn segdel_click(k: &K) {
    crossing_lines(k, true);
    k.click((100.0, 30.0));
    k.tool_cursor((100.0, 30.0), Tool::SegmentDelete);
}

fn segdel_marquee(k: &K) {
    k.line(&[(20.0, 30.0), (180.0, 30.0)], 1.5, INK);
    k.line(&[(60.0, 10.0), (60.0, 90.0)], 1.5, INK);
    k.line(&[(140.0, 10.0), (140.0, 30.0)], 1.5, INK);
    k.line(&[(20.0, 70.0), (60.0, 70.0)], 1.5, INK);
    k.dashed(&[(60.0, 70.0), (180.0, 70.0)], RED);
    k.dashed(&[(140.0, 30.0), (140.0, 90.0)], RED);
    k.dashed_closed(&rect_pts(110.0, 55.0, 175.0, 85.0), ACCENT);
    k.tool_cursor((175.0, 85.0), Tool::SegmentDelete);
}

fn eraser_drag(k: &K) {
    let r = rect_pts(35.0, 22.0, 165.0, 78.0);
    k.shape(&r, Paint::Solid(BLUE));
    let path = wave(20.0, 140.0, 50.0, 10.0, 1.0);
    k.line(&path, 14.0, PAGE);
    let clip =
        k.p.with_clip_rect(Rect::from_two_pos(k.pt((35.0, 22.0)), k.pt((165.0, 78.0))));
    for off in [-7.0, 7.0] {
        let edge: Vec<P> = path.iter().map(|&(x, y)| (x, y + off)).collect();
        clip.add(PathShape::line(k.pts(&edge), Stroke::new(1.0, INK)));
    }
    let end = path[path.len() - 1];
    k.p.circle_stroke(k.pt(end), 7.0 * k.s, Stroke::new(1.0, ACCENT));
    k.tool_cursor((end.0 + 4.0, end.1 + 4.0), Tool::Eraser);
}

// ---- Zoom, Pan ----

fn zoom_click(k: &K) {
    k.shape(&star_pts(55.0, 50.0, 14.0, 6.0, 5), Paint::Solid(YELLOW));
    k.drag((80.0, 50.0), (108.0, 50.0));
    k.shape(&star_pts(150.0, 50.0, 40.0, 17.0, 5), Paint::Solid(YELLOW));
    k.click((55.0, 50.0));
    k.tool_cursor((55.0, 50.0), Tool::Zoom);
}

fn zoom_marquee(k: &K) {
    k.shape(&ellipse_pts(60.0, 50.0, 22.0, 22.0), Paint::Solid(RED));
    k.shape(&rect_pts(110.0, 30.0, 160.0, 70.0), Paint::Solid(BLUE));
    k.dashed_closed(&rect_pts(30.0, 22.0, 92.0, 78.0), INK);
    k.drag((30.0, 22.0), (90.0, 76.0));
    k.tool_cursor((92.0, 78.0), Tool::Zoom);
}

fn pan_drag(k: &K) {
    k.dashed_closed(&ellipse_pts(70.0, 50.0, 20.0, 20.0), GHOST);
    k.dashed_closed(&rect_pts(100.0, 34.0, 130.0, 66.0), GHOST);
    k.shape(&ellipse_pts(100.0, 40.0, 20.0, 20.0), Paint::Solid(RED));
    k.shape(&rect_pts(130.0, 24.0, 160.0, 56.0), Paint::Solid(BLUE));
    k.drag((60.0, 80.0), (90.0, 70.0));
    k.tool_cursor((90.0, 70.0), Tool::Pan);
}

// ---- Curves ----

fn scribble() -> Vec<P> {
    (0..=60)
        .map(|i| {
            let t = i as f32 / 60.0;
            (
                20.0 + 150.0 * t,
                55.0 - 25.0 * (t * 5.0).sin() + 6.0 * (t * 17.0).cos(),
            )
        })
        .collect()
}

fn freehand_drag(k: &K) {
    let v = scribble();
    k.line(&v, 1.5, INK);
    k.tool_cursor(v[60], Tool::Freehand);
}

fn freehand_clicks(k: &K) {
    let a = (40.0, 75.0);
    let b = (160.0, 25.0);
    k.line(&[a, b], 1.5, INK);
    k.nodes(&[a, b]);
    k.click(a);
    k.click(b);
    k.tool_cursor(b, Tool::Freehand);
}

fn freehand_continue(k: &K) {
    let v: Vec<P> = scribble()[..36].to_vec();
    k.line(&v, 1.5, INK);
    k.node(v[0], false);
    let end = v[v.len() - 1];
    k.node(end, true);
    let more: Vec<P> = (0..=20)
        .map(|i| {
            let t = i as f32 / 20.0;
            (end.0 + 60.0 * t, end.1 + 20.0 * (t * 3.0).sin())
        })
        .collect();
    k.line(&more, 1.5, ACCENT);
    k.tool_cursor(more[20], Tool::Freehand);
}

fn line_drag(k: &K) {
    let a = (40.0, 72.0);
    let b = (160.0, 30.0);
    k.line(&[a, b], 1.5, INK);
    k.nodes(&[a, b]);
    k.drag((44.0, 82.0), (160.0, 40.0));
    k.tool_cursor(b, Tool::TwoPointLine);
}

fn line_angles(k: &K) {
    let o = (50.0, 80.0);
    for i in 0..=6 {
        let a = (i as f32 * 15.0).to_radians();
        let q = (o.0 + 110.0 * a.cos(), o.1 - 110.0 * a.sin());
        if i == 2 {
            k.line(&[o, q], 1.5, INK);
        } else {
            k.dashed(&[o, q], GHOST);
        }
    }
    k.text((100.0, 92.0), "Ctrl", 10.0, ACCENT, Align2::CENTER_CENTER);
    k.node(o, false);
    let a = 30f32.to_radians();
    k.tool_cursor(
        (o.0 + 110.0 * a.cos(), o.1 - 110.0 * a.sin()),
        Tool::TwoPointLine,
    );
}

fn bezier_corners(k: &K) {
    let v = [(30.0, 70.0), (75.0, 25.0), (120.0, 70.0), (165.0, 25.0)];
    k.line(&v, 1.5, INK);
    k.nodes(&v[..3]);
    k.node(v[3], true);
    k.click(v[3]);
    k.tool_cursor(v[3], Tool::Bezier);
}

fn bezier_smooth(k: &K) {
    let a = (30.0, 70.0);
    let b = (110.0, 55.0);
    let h_in = (75.0, 85.0);
    let h_out = (145.0, 25.0);
    k.line(&cubic(a, (40.0, 30.0), h_in, b), 1.5, INK);
    k.dashed(&cubic(b, h_out, (170.0, 70.0), (175.0, 70.0)), GHOST);
    k.ctrl(b, h_in);
    k.ctrl(b, h_out);
    k.node(a, false);
    k.node(b, true);
    k.drag(b, (143.0, 27.0));
    k.tool_cursor(h_out, Tool::Bezier);
}

fn bezier_close(k: &K) {
    let v = vec![
        (60.0, 75.0),
        (50.0, 30.0),
        (100.0, 15.0),
        (150.0, 30.0),
        (140.0, 75.0),
    ];
    let mut closed = v.clone();
    closed.push(v[0]);
    let curve = smooth_through(&closed);
    k.fill(&curve, Paint::Solid(LIGHT));
    k.line(&curve[..curve.len() - 20], 1.5, INK);
    k.dashed(&curve[curve.len() - 22..], ACCENT);
    k.nodes(&v);
    k.p.circle_stroke(k.pt(v[0]), k.px(7.0), Stroke::new(1.2, ACCENT));
    k.tool_cursor(v[0], Tool::Bezier);
}

fn pen_corners(k: &K) {
    let v = [(30.0, 70.0), (70.0, 30.0), (115.0, 65.0)];
    k.line(&v, 1.5, INK);
    k.nodes(&v);
    k.dashed(&[(115.0, 65.0), (165.0, 30.0)], ACCENT);
    k.click(v[2]);
    k.tool_cursor((165.0, 30.0), Tool::Pen);
}

fn pen_preview(k: &K) {
    let a = (30.0, 70.0);
    let b = (100.0, 40.0);
    k.line(&cubic(a, (40.0, 30.0), (70.0, 40.0), b), 1.5, INK);
    k.ctrl(b, (70.0, 40.0));
    k.ctrl(b, (130.0, 40.0));
    k.dashed(
        &cubic(b, (130.0, 40.0), (160.0, 75.0), (170.0, 75.0)),
        ACCENT,
    );
    k.node(a, false);
    k.node(b, true);
    k.tool_cursor((170.0, 75.0), Tool::Pen);
}

fn bspline_ctrl() -> Vec<P> {
    vec![
        (25.0, 75.0),
        (55.0, 15.0),
        (95.0, 80.0),
        (135.0, 15.0),
        (175.0, 70.0),
    ]
}

fn bspline_points(k: &K) {
    let c = bspline_ctrl();
    k.dashed(&c, ACCENT);
    k.line(&bspline(&c, false), 1.5, INK);
    for &q in &c {
        k.p.circle_filled(k.pt(q), k.px(3.0), ACCENT);
    }
    k.click(c[4]);
    k.tool_cursor(c[4], Tool::BSpline);
}

fn bspline_loop(k: &K) {
    let c = vec![
        (60.0, 80.0),
        (40.0, 25.0),
        (100.0, 10.0),
        (165.0, 25.0),
        (145.0, 80.0),
    ];
    k.dashed_closed(&c, ACCENT);
    let v = bspline(&c, true);
    k.fill(&v, Paint::Solid(LIGHT));
    k.outline(&v, true, 1.5, INK);
    for &q in &c {
        k.p.circle_filled(k.pt(q), k.px(3.0), ACCENT);
    }
    k.p.circle_stroke(k.pt(c[0]), k.px(7.0), Stroke::new(1.2, ACCENT));
    k.tool_cursor(c[0], Tool::BSpline);
}

fn polyline_clicks(k: &K) {
    let v = [
        (25.0, 70.0),
        (55.0, 25.0),
        (85.0, 70.0),
        (115.0, 25.0),
        (145.0, 70.0),
    ];
    k.line(&v, 1.5, INK);
    k.nodes(&v);
    k.dashed(&[(145.0, 70.0), (175.0, 30.0)], ACCENT);
    for &q in &v[1..] {
        k.click(q);
    }
    k.tool_cursor((175.0, 30.0), Tool::Polyline);
}

fn curve3_line(k: &K) {
    let a = (35.0, 70.0);
    let b = (165.0, 70.0);
    k.dashed(&[a, b], INK);
    k.nodes(&[a, b]);
    k.drag((35.0, 80.0), (163.0, 80.0));
    k.tool_cursor(b, Tool::ThreePointCurve);
}

fn curve3_bend(k: &K) {
    let a = (35.0, 70.0);
    let b = (165.0, 70.0);
    let top = (100.0, 25.0);
    k.line(&quad(a, (100.0, -20.0), b), 1.5, INK);
    k.dashed(&[a, b], GHOST);
    k.nodes(&[a, b]);
    k.click(top);
    k.tool_cursor(top, Tool::ThreePointCurve);
}

// ---- Artistic media ----

/// A filled stroke along `path` whose half width at `t` is `w(t)`.
fn stroke_band(k: &K, path: &[P], w: impl Fn(f32) -> f32, color: Color32) {
    let n = path.len();
    let mut mesh = Mesh::default();
    for i in 0..n {
        let a = path[i.saturating_sub(1)];
        let b = path[(i + 1).min(n - 1)];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dy).max(1e-3);
        let nn = (-dy / l, dx / l);
        let h = w(i as f32 / (n - 1) as f32);
        mesh.colored_vertex(k.pt((path[i].0 + nn.0 * h, path[i].1 + nn.1 * h)), color);
        mesh.colored_vertex(k.pt((path[i].0 - nn.0 * h, path[i].1 - nn.1 * h)), color);
    }
    for i in 0..n as u32 - 1 {
        mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
        mesh.add_triangle(2 * i + 1, 2 * i + 3, 2 * i + 2);
    }
    k.p.add(Shape::mesh(mesh));
}

fn brush_modes(k: &K) {
    let taper = |t: f32| 5.0 * (t * PI).sin();
    stroke_band(k, &wave(15.0, 85.0, 25.0, 6.0, 1.0), taper, INK);
    // Calligraphic: a flat nib at 45 degrees.
    let path = wave(110.0, 185.0, 25.0, 10.0, 1.0);
    let n = path.len();
    let mut mesh = Mesh::default();
    for q in &path {
        mesh.colored_vertex(k.pt((q.0 - 3.5, q.1 + 3.5)), INK);
        mesh.colored_vertex(k.pt((q.0 + 3.5, q.1 - 3.5)), INK);
    }
    for i in 0..n as u32 - 1 {
        mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
        mesh.add_triangle(2 * i + 1, 2 * i + 3, 2 * i + 2);
    }
    k.p.add(Shape::mesh(mesh));
    // Sprayer: shapes along the line.
    for (i, &q) in wave(20.0, 90.0, 72.0, 8.0, 1.0)
        .iter()
        .enumerate()
        .step_by(6)
    {
        let col = [RED, YELLOW, GREEN, BLUE, PURPLE][i / 6 % 5];
        k.shape(&star_pts(q.0, q.1, 6.0, 2.6, 5), Paint::Solid(col));
    }
    // Brush: a dry edge.
    let path = wave(110.0, 185.0, 72.0, 6.0, 1.0);
    stroke_band(k, &path, |t| 4.0 + 1.5 * (t * 90.0).sin(), INK);
}

fn brush_drag(k: &K) {
    let path = scribble();
    stroke_band(k, &path, |t| 6.0 * (t * PI).sin().powf(0.6), PURPLE);
    k.tool_cursor(path[60], Tool::BrushStrokes);
}

fn dabs(k: &K, path: &[P], size: f32, color: Color32) {
    for (i, &(x, y)) in path.iter().enumerate() {
        let t = i as f32 / path.len() as f32;
        let r = size * (0.4 + 0.6 * (t * PI).sin());
        for j in 0..3 {
            let a = (i * 7 + j * 3) as f32;
            let off = (r * 0.6 * a.sin(), r * 0.6 * (a * 1.3).cos());
            k.p.circle_filled(
                k.pt((x + off.0, y + off.1)),
                r * k.s * 0.55,
                alpha(color, 70),
            );
        }
    }
}

fn painterly_drag(k: &K) {
    let path = subdivide(&scribble(), 2.0);
    let path: Vec<P> = path[..path.len() / 2].to_vec();
    dabs(k, &path, 7.0, Color32::from_rgb(0x1F, 0x6F, 0xC4));
    k.tool_cursor(path[path.len() - 1], Tool::PainterlyBrush);
}

fn painterly_outline(k: &K) {
    let e = subdivide(&ellipse_pts(100.0, 50.0, 55.0, 32.0), 2.0);
    dabs(k, &e, 5.0, RED);
    k.click((100.0, 18.0));
    k.tool_cursor((100.0, 18.0), Tool::PainterlyBrush);
}

fn recognize_shapes(k: &K) {
    let wob: Vec<P> = subdivide(&rect_pts(22.0, 27.0, 78.0, 73.0), 2.0)
        .iter()
        .map(|&(x, y)| {
            let n = (x * 0.31).sin() + (y * 0.27).cos();
            (x + 1.6 * n, y - 1.6 * n)
        })
        .collect();
    k.outline(&wob, true, 1.2, GHOST);
    k.drag((88.0, 50.0), (112.0, 50.0));
    k.shape(&rect_pts(120.0, 25.0, 180.0, 75.0), Paint::None);
    k.handles((120.0, 25.0), (180.0, 75.0));
}

fn recognize_line(k: &K) {
    let wob: Vec<P> = (0..=40)
        .map(|i| {
            let t = i as f32 / 40.0;
            (20.0 + 65.0 * t, 70.0 - 40.0 * t + 2.5 * (t * 31.0).sin())
        })
        .collect();
    k.line(&wob, 1.2, GHOST);
    k.drag((88.0, 50.0), (112.0, 50.0));
    k.line(&[(120.0, 70.0), (185.0, 30.0)], 1.5, INK);
    k.nodes(&[(120.0, 70.0), (185.0, 30.0)]);
}

fn sketch_one(k: &K) {
    let v: Vec<P> = scribble()[..34].to_vec();
    for off in [-1.5f32, 1.2] {
        let jit: Vec<P> = v.iter().map(|&(x, y)| (x + off, y - off)).collect();
        k.line(&jit, 1.0, GHOST);
    }
    k.line(&v, 1.5, INK);
    let (a, b) = bbox(&v);
    k.handles(a, b);
    k.tool_cursor(v[33], Tool::Sketch);
}

fn sketch_join(k: &K) {
    let v = scribble();
    k.line(&v[..34], 1.5, INK);
    k.line(&v[33..], 1.5, ACCENT);
    k.p.circle_stroke(k.pt(v[33]), k.px(8.0), Stroke::new(1.0, ACCENT));
    k.tool_cursor(v[60], Tool::Sketch);
}

// ---- Rectangles, ellipses and other shapes ----

fn rect_drag(k: &K) {
    k.shape(&rect_pts(50.0, 22.0, 150.0, 78.0), Paint::Solid(LIGHT));
    k.drag((50.0, 22.0), (148.0, 76.0));
    k.tool_cursor((150.0, 78.0), Tool::Rectangle);
}

fn rect_square(k: &K) {
    k.shape(&rect_pts(25.0, 25.0, 75.0, 75.0), Paint::Solid(LIGHT));
    k.drag((25.0, 25.0), (73.0, 73.0));
    k.text((50.0, 92.0), "Ctrl", 10.0, ACCENT, Align2::CENTER_CENTER);
    k.tool_cursor((75.0, 75.0), Tool::Rectangle);
    k.shape(&rect_pts(110.0, 22.0, 180.0, 78.0), Paint::Solid(LIGHT));
    k.p.circle_filled(k.pt((145.0, 50.0)), k.px(2.5), ACCENT);
    k.drag((145.0, 50.0), (178.0, 76.0));
    k.text((145.0, 92.0), "Shift", 10.0, ACCENT, Align2::CENTER_CENTER);
}

fn corner_rect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32, kind: u8) -> Vec<P> {
    let c = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    let mut v = Vec::new();
    for i in 0..4 {
        let q = c[i];
        let prev = c[(i + 3) % 4];
        let next = c[(i + 1) % 4];
        let unit = |a: P| {
            let (dx, dy) = (a.0 - q.0, a.1 - q.1);
            let l = dx.hypot(dy).max(1e-3);
            (dx / l, dy / l)
        };
        let (u, w) = (unit(prev), unit(next));
        let s = (q.0 + u.0 * r, q.1 + u.1 * r);
        let e = (q.0 + w.0 * r, q.1 + w.1 * r);
        match kind {
            // Round: the corner bends outward.
            0 => v.extend(quad(s, q, e)),
            // Scalloped: a notch cut inward.
            1 => v.extend(quad(s, (q.0 + (u.0 + w.0) * r, q.1 + (u.1 + w.1) * r), e)),
            // Chamfered: a straight cut.
            _ => {
                v.push(s);
                v.push(e);
            }
        }
    }
    v
}

fn rect_corners(k: &K) {
    k.shape(
        &corner_rect(15.0, 25.0, 65.0, 75.0, 12.0, 0),
        Paint::Solid(YELLOW),
    );
    k.shape(
        &corner_rect(75.0, 25.0, 125.0, 75.0, 12.0, 1),
        Paint::Solid(GREEN),
    );
    k.shape(
        &corner_rect(135.0, 25.0, 185.0, 75.0, 12.0, 2),
        Paint::Solid(PURPLE),
    );
}

fn rect3_base(k: &K) {
    let a = (45.0, 80.0);
    let b = (140.0, 45.0);
    k.line(&[a, b], 1.5, INK);
    k.nodes(&[a, b]);
    k.drag((50.0, 88.0), (145.0, 53.0));
    k.tool_cursor(b, Tool::ThreePointRectangle);
}

fn rect3_height(k: &K) {
    let a = (45.0, 80.0);
    let b = (140.0, 45.0);
    let d = (-0.35 * 0.55 * 80.0, -0.94 * 0.55 * 80.0);
    let v = vec![a, b, (b.0 + d.0, b.1 + d.1), (a.0 + d.0, a.1 + d.1)];
    k.shape(&v, Paint::Solid(LIGHT));
    let top = (b.0 + d.0, b.1 + d.1);
    k.click(top);
    k.tool_cursor(top, Tool::ThreePointRectangle);
}

fn ellipse_drag(k: &K) {
    k.dashed_closed(&rect_pts(45.0, 20.0, 155.0, 80.0), GHOST);
    k.shape(&ellipse_pts(100.0, 50.0, 55.0, 30.0), Paint::Solid(LIGHT));
    k.drag((45.0, 20.0), (153.0, 78.0));
    k.tool_cursor((155.0, 80.0), Tool::Ellipse);
}

fn ellipse_circle(k: &K) {
    k.shape(&ellipse_pts(50.0, 50.0, 28.0, 28.0), Paint::Solid(LIGHT));
    k.text((50.0, 92.0), "Ctrl", 10.0, ACCENT, Align2::CENTER_CENTER);
    k.shape(&ellipse_pts(145.0, 48.0, 40.0, 28.0), Paint::Solid(LIGHT));
    k.p.circle_filled(k.pt((145.0, 48.0)), k.px(2.5), ACCENT);
    k.drag((145.0, 48.0), (178.0, 74.0));
    k.tool_cursor((180.0, 76.0), Tool::Ellipse);
    k.text((145.0, 92.0), "Shift", 10.0, ACCENT, Align2::CENTER_CENTER);
}

fn ellipse_pie_arc(k: &K) {
    let (cx, cy, r) = (55.0, 50.0, 32.0);
    let mut pie = vec![(cx, cy)];
    for j in 0..=40 {
        let a = 0.6 + (TAU - 1.2) * j as f32 / 40.0;
        pie.push((cx + r * a.cos(), cy - r * a.sin()));
    }
    k.shape(&pie, Paint::Solid(ORANGE));
    let arc: Vec<P> = (0..=40)
        .map(|j| {
            let a = 0.6 + (TAU - 1.2) * j as f32 / 40.0;
            (145.0 + r * a.cos(), 50.0 - r * a.sin())
        })
        .collect();
    k.line(&arc, 2.0, INK);
}

fn ellipse3_base(k: &K) {
    let a = (40.0, 75.0);
    let b = (160.0, 30.0);
    k.line(&[a, b], 1.5, INK);
    k.nodes(&[a, b]);
    k.drag((44.0, 84.0), (162.0, 40.0));
    k.tool_cursor(b, Tool::ThreePointEllipse);
}

fn ellipse3_height(k: &K) {
    let c = (100.0, 52.5);
    let e = rotate(&ellipse_pts(c.0, c.1, 64.0, 22.0), c, -20.6);
    k.shape(&e, Paint::Solid(LIGHT));
    k.dashed(&[(40.0, 75.0), (160.0, 30.0)], GHOST);
    let top = (c.0 + 22.0 * 0.35, c.1 - 22.0 * 0.94);
    k.click(top);
    k.tool_cursor(top, Tool::ThreePointEllipse);
}

fn polygon_drag(k: &K) {
    k.dashed_closed(&rect_pts(60.0, 14.0, 140.0, 86.0), GHOST);
    k.shape(&polygon_pts(100.0, 52.0, 38.0, 5), Paint::Solid(LIGHT));
    k.drag((60.0, 14.0), (138.0, 84.0));
    k.tool_cursor((140.0, 86.0), Tool::Polygon);
}

fn polygon_center(k: &K) {
    k.shape(&polygon_pts(100.0, 50.0, 36.0, 6), Paint::Solid(LIGHT));
    k.p.circle_filled(k.pt((100.0, 50.0)), k.px(2.5), ACCENT);
    k.drag((100.0, 50.0), (128.0, 76.0));
    k.tool_cursor((131.0, 78.0), Tool::Polygon);
}

fn polygon_nodes(k: &K) {
    // A pentagon whose side nodes were pulled in: every side follows.
    let v: Vec<P> = (0..10)
        .map(|i| {
            let a = -PI / 2.0 + i as f32 * PI / 5.0;
            let r = if i % 2 == 0 { 40.0 } else { 18.0 };
            (100.0 + r * a.cos(), 52.0 + r * a.sin())
        })
        .collect();
    let curve = smooth_through(&[v.clone(), vec![v[0]]].concat());
    k.dashed_closed(&polygon_pts(100.0, 52.0, 40.0, 5), GHOST);
    k.shape(&curve, Paint::Solid(PINK));
    k.nodes(&v);
    k.node(v[3], true);
    k.tool_cursor(v[3], Tool::Shape);
}

const PINK: Color32 = Color32::from_rgb(0xF4, 0x9A, 0xC1);

fn star_drag(k: &K) {
    k.shape(&star_pts(100.0, 54.0, 40.0, 17.0, 5), Paint::Solid(YELLOW));
    k.drag((60.0, 14.0), (138.0, 88.0));
    k.tool_cursor((140.0, 90.0), Tool::Star);
}

fn star_complex(k: &K) {
    let p = polygon_pts(100.0, 52.0, 40.0, 9);
    let mut v = Vec::new();
    for i in 0..9 {
        v.push(p[(i * 4) % 9]);
    }
    k.fill(&star_pts(100.0, 52.0, 40.0, 26.0, 9), Paint::Solid(YELLOW));
    k.p.circle_filled(k.pt((100.0, 52.0)), 22.0 * k.s, PAGE);
    k.outline(&v, true, 1.2, INK);
}

fn spiral(cx: f32, cy: f32, r: f32, turns: f32, log: bool) -> Vec<P> {
    (0..=200)
        .map(|i| {
            let t = i as f32 / 200.0;
            let a = t * turns * TAU;
            let rr = if log {
                r * (t * 3.0).exp_m1() / 3f32.exp_m1()
            } else {
                r * t
            };
            (cx + rr * a.cos(), cy + rr * a.sin())
        })
        .collect()
}

fn spiral_drag(k: &K) {
    k.line(&spiral(100.0, 50.0, 36.0, 4.0, false), 1.5, INK);
    k.drag((64.0, 14.0), (134.0, 84.0));
    k.tool_cursor((136.0, 86.0), Tool::Spiral);
}

fn spiral_kinds(k: &K) {
    k.line(&spiral(55.0, 50.0, 36.0, 4.0, false), 1.5, INK);
    k.line(&spiral(145.0, 50.0, 36.0, 4.0, true), 1.5, INK);
}

fn arrow_shape(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<P> {
    let my = (y0 + y1) / 2.0;
    let h = (y1 - y0) / 4.0;
    let head = x1 - (x1 - x0) * 0.35;
    vec![
        (x0, my - h),
        (head, my - h),
        (head, y0),
        (x1, my),
        (head, y1),
        (head, my + h),
        (x0, my + h),
    ]
}

fn heart(cx: f32, cy: f32, r: f32) -> Vec<P> {
    (0..64)
        .map(|i| {
            let t = i as f32 / 64.0 * TAU;
            let x = 16.0 * t.sin().powi(3);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            (cx + x * r / 16.0, cy - y * r / 16.0)
        })
        .collect()
}

fn common_set(k: &K) {
    let arrow = arrow_shape(10.0, 35.0, 50.0, 65.0);
    k.fill(&rect_pts(10.0, 42.5, 37.0, 57.5), Paint::Solid(BLUE));
    k.fill(
        &[(36.0, 35.0), (50.0, 50.0), (36.0, 65.0)],
        Paint::Solid(BLUE),
    );
    k.outline(&arrow, true, 1.0, INK);
    k.shape(&heart(80.0, 52.0, 18.0), Paint::Solid(RED));
    k.shape(
        &[(120.0, 30.0), (138.0, 50.0), (120.0, 70.0), (102.0, 50.0)],
        Paint::Solid(YELLOW),
    );
    // A speech bubble with a short tail.
    let mut bub = ellipse_pts(170.0, 45.0, 22.0, 16.0);
    bub.insert(24, (156.0, 70.0));
    k.shape(&bub, Paint::Solid(LIGHT));
}

fn common_drag(k: &K) {
    let v = arrow_shape(45.0, 22.0, 155.0, 78.0);
    k.fill(&rect_pts(45.0, 36.0, 117.0, 64.0), Paint::Solid(GREEN));
    k.fill(
        &[(116.0, 22.0), (155.0, 50.0), (116.0, 78.0)],
        Paint::Solid(GREEN),
    );
    k.outline(&v, true, 1.0, INK);
    k.drag((45.0, 22.0), (153.0, 76.0));
    k.tool_cursor((155.0, 78.0), Tool::CommonShapes);
}

fn action_lines_parallel(k: &K, x0: f32, y0: f32, x1: f32, y1: f32) {
    for i in 0..8 {
        let y = y0 + (y1 - y0) * (i as f32 + 0.5) / 8.0;
        let len = (x1 - x0) * (0.5 + 0.5 * ((i * 37 % 11) as f32 / 10.0));
        k.line(&[(x1 - len, y), (x1, y)], 1.6, INK);
    }
}

fn action_lines_radial(k: &K, c: P, r0: f32, r1: f32) {
    for i in 0..16 {
        let a = i as f32 / 16.0 * TAU;
        let inner = r0 * (1.0 + 0.4 * ((i * 7 % 5) as f32 / 4.0));
        k.line(
            &[
                (c.0 + inner * a.cos(), c.1 + inner * 0.7 * a.sin()),
                (c.0 + r1 * a.cos(), c.1 + r1 * 0.7 * a.sin()),
            ],
            1.4,
            INK,
        );
    }
}

fn action_kinds(k: &K) {
    action_lines_parallel(k, 15.0, 25.0, 85.0, 75.0);
    action_lines_radial(k, (145.0, 50.0), 16.0, 52.0);
}

fn action_drag(k: &K) {
    action_lines_radial(k, (100.0, 50.0), 18.0, 60.0);
    k.dashed_closed(&rect_pts(40.0, 8.0, 160.0, 92.0), GHOST);
    k.drag((40.0, 8.0), (158.0, 90.0));
    k.tool_cursor((160.0, 92.0), Tool::ActionLines);
}

fn grid(k: &K, x0: f32, y0: f32, x1: f32, y1: f32, cols: usize, rows: usize) {
    k.shape(&rect_pts(x0, y0, x1, y1), Paint::Solid(PAGE));
    for i in 1..cols {
        let x = x0 + (x1 - x0) * i as f32 / cols as f32;
        k.line(&[(x, y0), (x, y1)], 1.0, INK);
    }
    for j in 1..rows {
        let y = y0 + (y1 - y0) * j as f32 / rows as f32;
        k.line(&[(x0, y), (x1, y)], 1.0, INK);
    }
}

fn graph_rows(k: &K) {
    grid(k, 30.0, 20.0, 90.0, 80.0, 3, 3);
    grid(k, 110.0, 20.0, 190.0, 80.0, 4, 3);
}

fn graph_drag(k: &K) {
    grid(k, 50.0, 18.0, 150.0, 82.0, 4, 3);
    k.drag((50.0, 18.0), (148.0, 80.0));
    k.tool_cursor((150.0, 82.0), Tool::GraphPaper);
}

fn graph_cells(k: &K) {
    grid(k, 50.0, 18.0, 150.0, 82.0, 4, 3);
    k.shape(&rect_pts(75.0, 39.33, 100.0, 60.67), Paint::Solid(ORANGE));
    k.handles((75.0, 39.33), (100.0, 60.67));
}

// ---- Text and tables ----

fn text_artistic(k: &K) {
    k.text((40.0, 50.0), "Text", 34.0, INK, Align2::LEFT_CENTER);
    let x = 40.0 + 64.0;
    k.line(&[(x, 34.0), (x, 66.0)], 1.2, INK);
    k.click((40.0, 50.0));
    k.tool_cursor((150.0, 70.0), Tool::Text);
}

fn text_paragraph(k: &K) {
    k.dashed_closed(&rect_pts(40.0, 15.0, 160.0, 85.0), INK);
    k.handles((40.0, 15.0), (160.0, 85.0));
    let lens = [110.0, 104.0, 112.0, 96.0, 60.0];
    for (i, &l) in lens.iter().enumerate() {
        let y = 25.0 + i as f32 * 12.0;
        k.p.rect_filled(
            Rect::from_two_pos(k.pt((46.0, y)), k.pt((46.0 + l, y + 5.0))),
            1.0,
            GHOST,
        );
    }
    k.tool_cursor((160.0, 85.0), Tool::Text);
}

fn table_drag(k: &K) {
    grid(k, 40.0, 18.0, 160.0, 82.0, 4, 3);
    k.drag((40.0, 18.0), (158.0, 80.0));
    k.tool_cursor((160.0, 82.0), Tool::Table);
}

fn table_type(k: &K) {
    grid(k, 40.0, 18.0, 160.0, 82.0, 4, 3);
    let cell = Rect::from_two_pos(k.pt((70.0, 39.33)), k.pt((100.0, 60.67)));
    k.p.rect_stroke(
        cell,
        0.0,
        Stroke::new(2.0, ACCENT),
        egui::StrokeKind::Inside,
    );
    k.text((73.0, 50.0), "Abc", 11.0, INK, Align2::LEFT_CENTER);
    k.line(&[(93.0, 44.0), (93.0, 56.0)], 1.0, INK);
    k.tool_cursor((92.0, 56.0), Tool::Table);
}

// ---- Dimensions and callouts ----

fn dim_object(k: &K) -> (P, P) {
    let v = vec![(40.0, 80.0), (140.0, 40.0), (160.0, 80.0)];
    k.shape(&v, Paint::Solid(LIGHT));
    ((40.0, 80.0), (140.0, 40.0))
}

/// A dimension line from `a` to `b`, moved out by `off` across it.
fn dim_line(k: &K, a: P, b: P, off: f32, label: &str) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = dx.hypot(dy).max(1e-3);
    let n = (dy / l, -dx / l);
    let a2 = (a.0 + n.0 * off, a.1 + n.1 * off);
    let b2 = (b.0 + n.0 * off, b.1 + n.1 * off);
    k.line(&[a, (a2.0 + n.0 * 3.0, a2.1 + n.1 * 3.0)], 0.8, INK);
    k.line(&[b, (b2.0 + n.0 * 3.0, b2.1 + n.1 * 3.0)], 0.8, INK);
    k.line(&[a2, b2], 1.0, INK);
    k.arrowhead(k.pt(b2), k.pt(a2), INK);
    k.arrowhead(k.pt(a2), k.pt(b2), INK);
    let m = (
        (a2.0 + b2.0) / 2.0 + n.0 * 7.0,
        (a2.1 + b2.1) / 2.0 + n.1 * 7.0,
    );
    k.text(m, label, 10.0, INK, Align2::CENTER_CENTER);
}

fn pdim_points(k: &K) {
    let (a, b) = dim_object(k);
    k.node(a, true);
    k.node(b, true);
    k.drag(a, (138.0, 41.0));
    k.tool_cursor(b, Tool::ParallelDimension);
}

fn pdim_place(k: &K) {
    let (a, b) = dim_object(k);
    dim_line(k, a, b, 16.0, "107.7 mm");
    k.click((84.0, 47.0));
    k.tool_cursor((84.0, 47.0), Tool::ParallelDimension);
}

fn hvdim_points(k: &K) {
    let (a, b) = dim_object(k);
    k.node(a, true);
    k.node(b, true);
    k.drag(a, (138.0, 41.0));
    k.tool_cursor(b, Tool::HorizontalVerticalDimension);
}

fn hvdim_place(k: &K) {
    let (a, b) = dim_object(k);
    let y = 22.0;
    k.line(&[a, (a.0, y - 3.0)], 0.8, INK);
    k.line(&[b, (b.0, y - 3.0)], 0.8, INK);
    k.line(&[(a.0, y), (b.0, y)], 1.0, INK);
    k.arrowhead(k.pt((b.0, y)), k.pt((a.0, y)), INK);
    k.arrowhead(k.pt((a.0, y)), k.pt((b.0, y)), INK);
    k.text((90.0, y - 8.0), "100 mm", 10.0, INK, Align2::CENTER_CENTER);
    k.click((60.0, y));
    k.tool_cursor((60.0, y), Tool::HorizontalVerticalDimension);
}

fn angle_lines(k: &K) -> (P, P, P) {
    let v = (50.0, 80.0);
    let a = (170.0, 80.0);
    let b = (135.0, 20.0);
    k.line(&[a, v, b], 1.5, INK);
    (v, a, b)
}

fn adim_vertex(k: &K) {
    let (v, _, _) = angle_lines(k);
    k.click(v);
    k.tool_cursor(v, Tool::AngularDimension);
}

fn adim_sides(k: &K) {
    let (v, a, b) = angle_lines(k);
    k.node(v, true);
    k.click((150.0, 80.0));
    k.click(b);
    k.dashed(&[v, (150.0, 80.0)], ACCENT);
    k.dashed(&[v, b], ACCENT);
    k.tool_cursor(b, Tool::AngularDimension);
    let _ = a;
}

fn adim_place(k: &K) {
    let (v, _, b) = angle_lines(k);
    let a1 = (b.1 - v.1).atan2(b.0 - v.0);
    let r = 60.0;
    let arc: Vec<P> = (0..=20)
        .map(|j| {
            let a = a1 * j as f32 / 20.0;
            (v.0 + r * a.cos(), v.1 + r * a.sin())
        })
        .collect();
    k.line(&arc, 1.0, INK);
    k.arrowhead(k.pt(arc[1]), k.pt(arc[0]), INK);
    k.arrowhead(k.pt(arc[19]), k.pt(arc[20]), INK);
    let m = arc[10];
    k.text(
        (m.0 + 16.0, m.1 - 2.0),
        "35°",
        11.0,
        INK,
        Align2::LEFT_CENTER,
    );
    k.click((m.0 + 8.0, m.1));
    k.tool_cursor((m.0 + 8.0, m.1), Tool::AngularDimension);
}

fn sdim_click(k: &K) {
    let v = vec![(40.0, 80.0), (60.0, 30.0), (140.0, 30.0), (160.0, 80.0)];
    k.shape(&v, Paint::Solid(LIGHT));
    k.line(&[(60.0, 30.0), (140.0, 30.0)], 2.5, ACCENT);
    k.click((100.0, 30.0));
    k.tool_cursor((100.0, 30.0), Tool::SegmentDimension);
}

fn sdim_place(k: &K) {
    let v = vec![(40.0, 80.0), (60.0, 30.0), (140.0, 30.0), (160.0, 80.0)];
    k.shape(&v, Paint::Solid(LIGHT));
    dim_line(k, (60.0, 30.0), (140.0, 30.0), 12.0, "80 mm");
    k.tool_cursor((150.0, 18.0), Tool::SegmentDimension);
}

fn callout_leader(k: &K) {
    k.shape(&ellipse_pts(50.0, 65.0, 22.0, 18.0), Paint::Solid(GREEN));
    let tip = (55.0, 62.0);
    let elbow = (110.0, 28.0);
    k.line(&[tip, elbow], 1.0, INK);
    k.arrowhead(k.pt(elbow), k.pt(tip), INK);
    k.drag((60.0, 72.0), (115.0, 38.0));
    k.tool_cursor(elbow, Tool::Callout);
}

fn callout_text(k: &K) {
    k.shape(&ellipse_pts(50.0, 65.0, 22.0, 18.0), Paint::Solid(GREEN));
    let tip = (55.0, 62.0);
    let elbow = (110.0, 28.0);
    k.line(&[tip, elbow, (125.0, 28.0)], 1.0, INK);
    k.arrowhead(k.pt(elbow), k.pt(tip), INK);
    k.text((128.0, 28.0), "Note", 14.0, INK, Align2::LEFT_CENTER);
    k.line(&[(162.0, 20.0), (162.0, 36.0)], 1.0, INK);
    k.click((126.0, 28.0));
}

// ---- Connectors and anchors ----

fn two_boxes(k: &K, dx: f32, dy: f32) -> (P, P) {
    k.shape(&rect_pts(20.0, 25.0, 65.0, 60.0), Paint::Solid(LIGHT));
    let b = rect_pts(130.0 + dx, 45.0 + dy, 175.0 + dx, 80.0 + dy);
    k.shape(&b, Paint::Solid(YELLOW));
    ((65.0, 42.5), (130.0 + dx, 62.5 + dy))
}

fn anchor_marks(k: &K, a: P, b: P) {
    k.diamond(a, ACCENT);
    k.diamond(b, ACCENT);
}

fn conn_drag(k: &K) {
    let (a, b) = two_boxes(k, 0.0, 0.0);
    k.line(&[a, b], 1.5, INK);
    anchor_marks(k, a, b);
    k.drag((50.0, 50.0), (150.0, 66.0));
    k.tool_cursor((152.0, 66.0), Tool::Connector);
}

fn conn_follow(k: &K) {
    k.dashed_closed(&rect_pts(130.0, 45.0, 175.0, 80.0), GHOST);
    let (a, b) = two_boxes(k, 0.0, -35.0);
    k.line(&[a, b], 1.5, INK);
    anchor_marks(k, a, b);
    k.drag((152.0, 70.0), (152.0, 36.0));
    k.cursor((154.0, 34.0));
}

fn ortho(a: P, b: P) -> Vec<P> {
    let mx = (a.0 + b.0) / 2.0;
    vec![a, (mx, a.1), (mx, b.1), b]
}

fn rounded(v: &[P], r: f32) -> Vec<P> {
    let mut out = vec![v[0]];
    for i in 1..v.len() - 1 {
        let (a, c, b) = (v[i - 1], v[i], v[i + 1]);
        let la = (a.0 - c.0).hypot(a.1 - c.1).max(1e-3);
        let lb = (b.0 - c.0).hypot(b.1 - c.1).max(1e-3);
        let rr = r.min(la / 2.0).min(lb / 2.0);
        let s = (c.0 + (a.0 - c.0) / la * rr, c.1 + (a.1 - c.1) / la * rr);
        let e = (c.0 + (b.0 - c.0) / lb * rr, c.1 + (b.1 - c.1) / lb * rr);
        out.extend(quad(s, c, e));
    }
    out.push(v[v.len() - 1]);
    out
}

fn rconn_drag(k: &K) {
    let (a, b) = two_boxes(k, 0.0, 0.0);
    k.line(&ortho(a, b), 1.5, INK);
    anchor_marks(k, a, b);
    k.drag((50.0, 50.0), (150.0, 66.0));
    k.tool_cursor((152.0, 66.0), Tool::RightAngleConnector);
}

fn rconn_follow(k: &K) {
    k.dashed_closed(&rect_pts(130.0, 45.0, 175.0, 80.0), GHOST);
    let (a, b) = two_boxes(k, 0.0, -35.0);
    let (a, b) = (a, (b.0, b.1));
    k.line(&ortho(a, b), 1.5, INK);
    anchor_marks(k, a, b);
    k.drag((152.0, 70.0), (152.0, 36.0));
    k.cursor((154.0, 34.0));
}

fn rnd_conn_drag(k: &K) {
    let (a, b) = two_boxes(k, 0.0, 0.0);
    k.line(&rounded(&ortho(a, b), 6.0), 1.5, INK);
    anchor_marks(k, a, b);
    k.drag((50.0, 50.0), (150.0, 66.0));
    k.tool_cursor((152.0, 66.0), Tool::RoundedConnector);
}

fn rnd_conn_radius(k: &K) {
    let (a, b) = two_boxes(k, 0.0, 0.0);
    k.line(&rounded(&ortho(a, b), 12.0), 1.5, INK);
    anchor_marks(k, a, b);
}

fn anchors_show(k: &K) {
    let r = rect_pts(60.0, 22.0, 140.0, 78.0);
    k.shape(&r, Paint::Solid(LIGHT));
    for q in [(100.0, 22.0), (140.0, 50.0), (100.0, 78.0), (60.0, 50.0)] {
        k.diamond(q, PAGE);
    }
    k.click((120.0, 64.0));
    k.tool_cursor((120.0, 64.0), Tool::AnchorEditing);
}

fn anchors_add(k: &K) {
    let r = rect_pts(60.0, 22.0, 140.0, 78.0);
    k.shape(&r, Paint::Solid(LIGHT));
    for q in [(100.0, 22.0), (140.0, 50.0), (100.0, 78.0), (60.0, 50.0)] {
        k.diamond(q, PAGE);
    }
    k.diamond((120.0, 36.0), ACCENT);
    k.dashed(&[(84.0, 64.0), (118.0, 38.0)], ACCENT);
    k.diamond((84.0, 64.0), Color32::from_gray(0xE0));
    k.tool_cursor((120.0, 36.0), Tool::AnchorEditing);
}

// ---- Effects ----

fn soft_shadow(k: &K, v: &[P]) {
    // Copies spread around the shadow, faint, for a feathered edge.
    k.fill(
        v,
        Paint::Solid(Color32::from_rgba_unmultiplied(0, 0, 0, 40)),
    );
    for (r, a) in [(1.5, 22u8), (3.0, 14)] {
        for i in 0..8 {
            let t = i as f32 / 8.0 * TAU;
            let c = Color32::from_rgba_unmultiplied(0, 0, 0, a);
            k.fill(&translate(v, r * t.cos(), r * t.sin()), Paint::Solid(c));
        }
    }
}

fn shadow_flat(k: &K) {
    let r = star_pts(80.0, 45.0, 32.0, 14.0, 5);
    soft_shadow(k, &translate(&r, 16.0, 12.0));
    k.shape(&r, Paint::Solid(YELLOW));
    k.vector((80.0, 48.0), (96.0, 60.0), PAGE, INK, 0.5);
    k.drag((100.0, 64.0), (122.0, 76.0));
    k.tool_cursor((124.0, 78.0), Tool::DropShadow);
}

fn shadow_perspective(k: &K) {
    let r = rect_pts(70.0, 20.0, 110.0, 80.0);
    let sh: Vec<P> = vec![(70.0, 80.0), (110.0, 80.0), (165.0, 55.0), (125.0, 55.0)];
    soft_shadow(k, &sh);
    k.shape(&r, Paint::Solid(BLUE));
    k.vector((90.0, 80.0), (145.0, 55.0), PAGE, INK, 0.5);
    k.tool_cursor((147.0, 55.0), Tool::DropShadow);
}

fn shadow_opacity(k: &K) {
    let r = star_pts(80.0, 45.0, 32.0, 14.0, 5);
    let sh = translate(&r, 30.0, 18.0);
    k.fill(
        &sh,
        Paint::Solid(Color32::from_rgba_unmultiplied(0, 0, 0, 30)),
    );
    k.shape(&r, Paint::Solid(YELLOW));
    k.vector((80.0, 48.0), (110.0, 66.0), PAGE, INK, 0.3);
    k.drag((97.0, 60.0), (86.0, 53.0));
    k.cursor((88.0, 54.0));
}

fn contour_out(k: &K) {
    let base = star_pts(100.0, 50.0, 22.0, 10.0, 5);
    for i in (1..=3).rev() {
        let g = grow(&base, i as f32 * 6.0);
        k.outline(&g, true, 1.0, lerp(BLUE, INK, i as f32 / 3.0));
    }
    k.shape(&base, Paint::Solid(BLUE));
    k.drag((114.0, 56.0), (150.0, 74.0));
    k.tool_cursor((152.0, 76.0), Tool::Contour);
}

fn contour_in(k: &K) {
    let base = ellipse_pts(100.0, 50.0, 60.0, 38.0);
    k.shape(&base, Paint::Solid(ORANGE));
    for i in 1..=4 {
        let g = grow(&base, -(i as f32) * 7.0);
        k.fill(&g, Paint::Solid(lerp(ORANGE, PAGE, i as f32 / 4.0)));
        k.outline(&g, true, 1.0, INK);
    }
    k.drag((158.0, 50.0), (122.0, 50.0));
    k.tool_cursor((120.0, 50.0), Tool::Contour);
}

/// Move every point of a closed outline out along its normal by `d`.
fn grow(v: &[P], d: f32) -> Vec<P> {
    let n = v.len();
    (0..n)
        .map(|i| {
            let a = v[(i + n - 1) % n];
            let b = v[(i + 1) % n];
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let l = dx.hypot(dy).max(1e-3);
            // Points run clockwise on screen: the outward normal is (dy, -dx).
            let nn = (dy / l, -dx / l);
            (v[i].0 + nn.0 * d, v[i].1 + nn.1 * d)
        })
        .collect()
}

fn blend_shapes(k: &K, steps: usize) {
    let a = ellipse_pts(30.0, 60.0, 14.0, 14.0);
    let b: Vec<P> = star_pts(165.0, 42.0, 28.0, 12.0, 5);
    let b = subdivide(&b, 0.5);
    // Sample the star at 64 points evenly by angle from its center.
    let b: Vec<P> = (0..64)
        .map(|i| {
            let ang = i as f32 / 64.0 * TAU;
            let best = b
                .iter()
                .min_by(|p, q| {
                    let da = ((p.1 - 42.0).atan2(p.0 - 165.0) - ang).sin().abs()
                        + (((p.1 - 42.0).atan2(p.0 - 165.0) - ang).cos() - 1.0).abs();
                    let db = ((q.1 - 42.0).atan2(q.0 - 165.0) - ang).sin().abs()
                        + (((q.1 - 42.0).atan2(q.0 - 165.0) - ang).cos() - 1.0).abs();
                    da.total_cmp(&db)
                })
                .copied()
                .unwrap_or((165.0, 42.0));
            best
        })
        .collect();
    for s in 0..=steps + 1 {
        let t = s as f32 / (steps + 1) as f32;
        let v: Vec<P> = a
            .iter()
            .zip(&b)
            .map(|(p, q)| (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t))
            .collect();
        k.shape(&v, Paint::Solid(lerp(RED, YELLOW, t)));
    }
}

fn blend_drag(k: &K) {
    blend_shapes(k, 4);
    k.drag((32.0, 80.0), (160.0, 72.0));
    k.tool_cursor((162.0, 72.0), Tool::Blend);
}

fn blend_steps(k: &K) {
    blend_shapes(k, 12);
}

fn distort_push(k: &K) {
    let c = (65.0, 50.0);
    let v: Vec<P> = (0..120)
        .map(|i| {
            let a = i as f32 / 120.0 * TAU;
            let r = 26.0 + 12.0 * (a * 6.0).cos();
            (c.0 + r * a.cos(), c.1 + r * a.sin())
        })
        .collect();
    k.shape(&v, Paint::Solid(PURPLE));
    let c2 = (150.0, 50.0);
    let w: Vec<P> = (0..120)
        .map(|i| {
            let a = i as f32 / 120.0 * TAU;
            let r = 22.0 + 14.0 * (a * 6.0).cos().abs().powf(3.0);
            (c2.0 + r * a.cos(), c2.1 + r * a.sin())
        })
        .collect();
    k.shape(&w, Paint::Solid(PINK));
    k.diamond(c, PAGE);
    k.drag((65.0, 50.0), (95.0, 50.0));
    k.tool_cursor((97.0, 50.0), Tool::Distort);
}

fn distort_zipper(k: &K) {
    let c = (100.0, 50.0);
    let v: Vec<P> = (0..160)
        .map(|i| {
            let a = i as f32 / 160.0 * TAU;
            let r = if i % 4 < 2 { 36.0 } else { 28.0 };
            (c.0 + r * 1.3 * a.cos(), c.1 + r * a.sin())
        })
        .collect();
    k.shape(&v, Paint::Solid(GREEN));
    k.diamond(c, PAGE);
    k.drag(c, (150.0, 70.0));
    k.tool_cursor((152.0, 72.0), Tool::Distort);
}

fn distort_twister(k: &K) {
    let c = (100.0, 50.0);
    let base = subdivide(&star_pts(c.0, c.1, 40.0, 14.0, 5), 1.0);
    let v: Vec<P> = base
        .iter()
        .map(|&(x, y)| {
            let (dx, dy) = (x - c.0, y - c.1);
            let d = dx.hypot(dy);
            let a = d / 40.0 * 1.6;
            let (s, co) = a.sin_cos();
            (c.0 + dx * co - dy * s, c.1 + dx * s + dy * co)
        })
        .collect();
    k.shape(&v, Paint::Solid(ORANGE));
    k.diamond(c, PAGE);
    let arc: Vec<P> = (0..=14)
        .map(|j| {
            let a = -0.3 + 2.6 * j as f32 / 14.0;
            (c.0 + 46.0 * a.cos(), c.1 + 42.0 * a.sin())
        })
        .collect();
    k.drag_path(&arc);
    k.tool_cursor(arc[14], Tool::Distort);
}

fn distort_center(k: &K) {
    let petals = |cx: f32| -> Vec<P> {
        (0..120)
            .map(|i| {
                let a = i as f32 / 120.0 * TAU;
                let r = 26.0 + 12.0 * (a * 6.0).cos();
                (cx + r * a.cos(), 50.0 + r * a.sin())
            })
            .collect()
    };
    k.dashed_closed(&petals(75.0), GHOST);
    k.diamond((75.0, 50.0), PAGE);
    k.shape(&petals(125.0), Paint::Solid(PURPLE));
    k.diamond((125.0, 50.0), ACCENT);
    k.drag((80.0, 50.0), (119.0, 50.0));
    k.cursor((126.0, 51.0));
}

/// A striped panel bent by `f`, which maps the unit square onto the frame.
fn warped(k: &K, f: impl Fn(f32, f32) -> P) {
    let n = 24;
    for row in 0..4 {
        let mut mesh = Mesh::default();
        let color = [BLUE, YELLOW, BLUE, YELLOW][row];
        for i in 0..=n {
            let u = i as f32 / n as f32;
            mesh.colored_vertex(k.pt(f(u, row as f32 / 4.0)), color);
            mesh.colored_vertex(k.pt(f(u, (row + 1) as f32 / 4.0)), color);
        }
        for i in 0..n as u32 {
            mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
            mesh.add_triangle(2 * i + 1, 2 * i + 3, 2 * i + 2);
        }
        k.p.add(Shape::mesh(mesh));
    }
    let edge: Vec<P> = (0..=n)
        .map(|i| f(i as f32 / n as f32, 0.0))
        .chain((0..=n).map(|i| f(1.0, i as f32 / n as f32)))
        .chain((0..=n).map(|i| f(1.0 - i as f32 / n as f32, 1.0)))
        .chain((0..=n).map(|i| f(0.0, 1.0 - i as f32 / n as f32)))
        .collect();
    k.dashed_closed(&edge, INK);
    for (u, v) in [
        (0.0, 0.0),
        (0.5, 0.0),
        (1.0, 0.0),
        (1.0, 0.5),
        (1.0, 1.0),
        (0.5, 1.0),
        (0.0, 1.0),
        (0.0, 0.5),
    ] {
        k.node(f(u, v), false);
    }
}

fn envelope_frame(k: &K) {
    warped(k, |u, v| (50.0 + 100.0 * u, 20.0 + 60.0 * v));
    k.click((120.0, 45.0));
    k.tool_cursor((120.0, 45.0), Tool::Envelope);
}

fn envelope_bend(k: &K) {
    let f = |u: f32, v: f32| {
        let lift = 18.0 * (u * PI).sin();
        (50.0 + 100.0 * u, 26.0 + 54.0 * v - lift * (1.0 - v * 0.6))
    };
    warped(k, f);
    k.node(f(0.5, 0.0), true);
    k.drag((100.0, 26.0), (100.0, 10.0));
    k.tool_cursor((102.0, 8.0), Tool::Envelope);
}

fn extrusion(k: &K, vp: P) {
    let front = rect_pts(40.0, 35.0, 90.0, 78.0);
    let depth = 0.45;
    let back: Vec<P> = front
        .iter()
        .map(|&(x, y)| (x + (vp.0 - x) * depth, y + (vp.1 - y) * depth))
        .collect();
    let shades = [
        Color32::from_rgb(0x5A, 0xA8, 0xF2),
        Color32::from_rgb(0x1C, 0x5E, 0xA8),
        Color32::from_rgb(0x16, 0x4C, 0x8A),
        Color32::from_rgb(0x2A, 0x7B, 0xD4),
    ];
    // The sides seen from the front: those the vanishing point lies beyond.
    let mid = (65.0, 56.5);
    for i in 0..4 {
        let j = (i + 1) % 4;
        let side_mid = (
            (front[i].0 + front[j].0) / 2.0,
            (front[i].1 + front[j].1) / 2.0,
        );
        let out = (side_mid.0 - mid.0, side_mid.1 - mid.1);
        if out.0 * (vp.0 - side_mid.0) + out.1 * (vp.1 - side_mid.1) <= 0.0 {
            continue;
        }
        let face = vec![front[i], front[j], back[j], back[i]];
        k.fill(&face, Paint::Solid(shades[i]));
        // Edge by edge: a closed outline of a thin face miters into a spike.
        for e in 0..4 {
            k.line(&[face[e], face[(e + 1) % 4]], 1.0, INK);
        }
    }
    k.shape(&front, Paint::Solid(BLUE));
}

fn extrude_drag(k: &K) {
    let vp = (175.0, 20.0);
    extrusion(k, vp);
    k.dashed(&[(65.0, 52.0), vp], ACCENT);
    k.tool_cursor(vp, Tool::Extrude);
}

fn extrude_vp(k: &K) {
    let vp = (170.0, 75.0);
    extrusion(k, vp);
    k.dashed(&[(65.0, 52.0), vp], GHOST);
    let c = k.pt(vp);
    let r = Rect::from_center_size(c, Vec2::splat(k.px(9.0)));
    k.p.rect_filled(r, 0.0, PAGE);
    k.p.rect_stroke(r, 0.0, Stroke::new(1.0, INK), egui::StrokeKind::Middle);
    k.p.line_segment(
        [c - Vec2::X * k.px(3.0), c + Vec2::X * k.px(3.0)],
        Stroke::new(1.0, INK),
    );
    k.p.line_segment(
        [c - Vec2::Y * k.px(3.0), c + Vec2::Y * k.px(3.0)],
        Stroke::new(1.0, INK),
    );
    k.drag((175.0, 20.0), (170.0, 68.0));
    k.cursor((172.0, 78.0));
}

fn block_shadow_drag(k: &K) {
    let v = star_pts(70.0, 42.0, 30.0, 13.0, 5);
    let shade = Color32::from_gray(0x70);
    for i in (1..=30).rev() {
        let t = i as f32;
        k.fill(&translate(&v, t * 0.9, t * 0.6), Paint::Solid(shade));
    }
    k.shape(&v, Paint::Solid(RED));
    k.drag((80.0, 50.0), (110.0, 70.0));
    k.tool_cursor((112.0, 72.0), Tool::BlockShadow);
}

fn stripes(k: &K) {
    for i in 0..10 {
        let x = 20.0 + i as f32 * 16.0;
        let c = if i % 2 == 0 { GREEN } else { PAGE };
        k.p.rect_filled(
            Rect::from_two_pos(k.pt((x, 18.0)), k.pt((x + 16.0, 82.0))),
            0.0,
            c,
        );
    }
}

fn transp_uniform(k: &K) {
    stripes(k);
    k.fill(
        &ellipse_pts(100.0, 50.0, 55.0, 26.0),
        Paint::Solid(alpha(RED, 130)),
    );
    k.outline(&ellipse_pts(100.0, 50.0, 55.0, 26.0), true, 1.0, INK);
    k.click((120.0, 50.0));
    k.tool_cursor((120.0, 50.0), Tool::Transparency);
}

fn transp_gradient(k: &K) {
    stripes(k);
    let e = ellipse_pts(100.0, 50.0, 60.0, 28.0);
    k.fill(&e, grad(RED, alpha(RED, 0), (50.0, 50.0), (150.0, 50.0)));
    k.outline(&e, true, 1.0, INK);
    k.vector((50.0, 50.0), (150.0, 50.0), PAGE, INK, 0.5);
    k.tool_cursor((152.0, 52.0), Tool::Transparency);
}

fn transp_slider(k: &K) {
    stripes(k);
    let e = ellipse_pts(100.0, 50.0, 60.0, 28.0);
    k.fill(&e, grad(RED, alpha(RED, 0), (50.0, 60.0), (140.0, 30.0)));
    k.outline(&e, true, 1.0, INK);
    k.vector((50.0, 60.0), (140.0, 30.0), PAGE, INK, 0.7);
    k.drag((95.0, 54.0), (112.0, 49.0));
    k.cursor((114.0, 41.0));
}

fn eyedrop_sample(k: &K) {
    k.shape(&ellipse_pts(60.0, 50.0, 30.0, 30.0), Paint::Solid(RED));
    k.shape(&rect_pts(120.0, 25.0, 175.0, 75.0), Paint::Solid(PAGE));
    k.click((60.0, 50.0));
    k.tool_cursor((60.0, 50.0), Tool::ColorEyedropper);
}

fn eyedrop_apply(k: &K) {
    k.shape(&ellipse_pts(60.0, 50.0, 30.0, 30.0), Paint::Solid(RED));
    k.shape(&rect_pts(120.0, 25.0, 175.0, 75.0), Paint::Solid(RED));
    k.click((147.0, 50.0));
    k.tool_cursor((147.0, 50.0), Tool::InteractiveFill);
}

fn attrs_shape(k: &K, v: &[P]) {
    k.fill(v, grad(YELLOW, ORANGE, (0.0, 20.0), (0.0, 80.0)));
    k.outline(v, true, 3.0, PURPLE);
}

fn attrs_sample(k: &K) {
    attrs_shape(k, &star_pts(60.0, 52.0, 32.0, 14.0, 5));
    k.shape(&ellipse_pts(145.0, 50.0, 30.0, 25.0), Paint::Solid(LIGHT));
    k.click((60.0, 52.0));
    k.tool_cursor((60.0, 52.0), Tool::AttributesEyedropper);
}

fn attrs_apply(k: &K) {
    attrs_shape(k, &star_pts(60.0, 52.0, 32.0, 14.0, 5));
    attrs_shape(k, &ellipse_pts(145.0, 50.0, 30.0, 25.0));
    k.click((145.0, 50.0));
    k.tool_cursor((145.0, 50.0), Tool::AttributesEyedropper);
}

fn fill_rect(k: &K, a: P, b: P) {
    let r = rect_pts(30.0, 18.0, 170.0, 82.0);
    k.fill(&r, grad(BLUE, RED, a, b));
    k.outline(&r, true, 1.0, INK);
}

fn fill_drag(k: &K) {
    let (a, b) = ((45.0, 50.0), (155.0, 50.0));
    fill_rect(k, a, b);
    k.vector(a, b, BLUE, RED, 0.5);
    k.tool_cursor((157.0, 52.0), Tool::InteractiveFill);
}

fn fill_slider(k: &K) {
    let (a, b) = ((45.0, 50.0), (155.0, 50.0));
    let r = rect_pts(30.0, 18.0, 170.0, 82.0);
    // The midpoint moved right: blue reaches further.
    k.fill(
        &r,
        Paint::Linear(a, (b.0 + 60.0, b.1), BLUE, lerp(BLUE, RED, 1.6)),
    );
    let half: Vec<P> = rect_pts(127.0, 18.0, 170.0, 82.0);
    k.fill(
        &half,
        grad(lerp(BLUE, RED, 0.5), RED, (127.0, 0.0), (155.0, 0.0)),
    );
    k.outline(&r, true, 1.0, INK);
    k.vector(a, b, BLUE, RED, 0.75);
    k.drag((100.0, 66.0), (126.0, 66.0));
    k.cursor((128.0, 58.0));
}

fn fill_end(k: &K) {
    let (a, b) = ((45.0, 75.0), (145.0, 25.0));
    fill_rect(k, a, b);
    k.vector(a, b, BLUE, RED, 0.5);
    k.drag((155.0, 50.0), (147.0, 30.0));
    k.cursor((147.0, 27.0));
}

fn fill_add_node(k: &K) {
    let (a, b) = ((45.0, 50.0), (155.0, 50.0));
    let r = rect_pts(30.0, 18.0, 170.0, 82.0);
    k.fill(
        &rect_pts(30.0, 18.0, 100.0, 82.0),
        grad(BLUE, YELLOW, a, (100.0, 50.0)),
    );
    k.fill(
        &rect_pts(100.0, 18.0, 170.0, 82.0),
        grad(YELLOW, RED, (100.0, 50.0), b),
    );
    k.outline(&r, true, 1.0, INK);
    k.vector(a, b, BLUE, RED, 0.25);
    let m = k.pt((100.0, 50.0));
    let sq = Rect::from_center_size(m, Vec2::splat(k.px(9.0)));
    k.p.rect_filled(sq, 0.0, YELLOW);
    k.p.rect_stroke(sq, 0.0, Stroke::new(1.0, INK), egui::StrokeKind::Outside);
    k.click((100.0, 50.0));
    k.cursor((102.0, 52.0));
}

fn overlap_circles(k: &K) {
    k.outline(&ellipse_pts(80.0, 50.0, 36.0, 36.0), true, 1.5, INK);
    k.outline(&ellipse_pts(120.0, 50.0, 36.0, 36.0), true, 1.5, INK);
}

fn lens_pts() -> Vec<P> {
    // The overlap of the two circles.
    let mut v = Vec::new();
    let h = (36.0f32 * 36.0 - 20.0 * 20.0).sqrt();
    let a = h.atan2(20.0);
    for j in 0..=20 {
        let t = -a + 2.0 * a * j as f32 / 20.0;
        v.push((80.0 + 36.0 * t.cos(), 50.0 + 36.0 * t.sin()));
    }
    for j in 0..=20 {
        let t = PI - a + 2.0 * a * j as f32 / 20.0;
        v.push((120.0 + 36.0 * t.cos(), 50.0 + 36.0 * t.sin()));
    }
    v
}

fn area_click(k: &K) {
    overlap_circles(k);
    k.click((100.0, 50.0));
    k.tool_cursor((100.0, 50.0), Tool::AreaFill);
}

fn area_filled(k: &K) {
    let l = lens_pts();
    k.fill(&l, Paint::Solid(GREEN));
    overlap_circles(k);
    let (a, b) = bbox(&l);
    k.handles(a, b);
}

/// A mesh over a rectangle: node positions moved by `bend`, colors at the
/// nodes from `color`.
fn mesh(k: &K, bend: impl Fn(usize, usize) -> P, color: impl Fn(usize, usize) -> Color32) {
    let (cols, rows) = (3usize, 3usize);
    let at = |i: usize, j: usize| {
        let base = (
            40.0 + 120.0 * i as f32 / cols as f32,
            15.0 + 70.0 * j as f32 / rows as f32,
        );
        let d = bend(i, j);
        (base.0 + d.0, base.1 + d.1)
    };
    let mut m = Mesh::default();
    for j in 0..=rows {
        for i in 0..=cols {
            m.colored_vertex(k.pt(at(i, j)), color(i, j));
        }
    }
    let idx = |i: usize, j: usize| (j * (cols + 1) + i) as u32;
    for j in 0..rows {
        for i in 0..cols {
            m.add_triangle(idx(i, j), idx(i + 1, j), idx(i + 1, j + 1));
            m.add_triangle(idx(i, j), idx(i + 1, j + 1), idx(i, j + 1));
        }
    }
    k.p.add(Shape::mesh(m));
    for j in 0..=rows {
        let v: Vec<P> = (0..=cols).map(|i| at(i, j)).collect();
        k.dashed(&v, INK);
    }
    for i in 0..=cols {
        let v: Vec<P> = (0..=rows).map(|j| at(i, j)).collect();
        k.dashed(&v, INK);
    }
    for j in 0..=rows {
        for i in 0..=cols {
            k.node(at(i, j), false);
        }
    }
}

fn mesh_add(k: &K) {
    mesh(k, |_, _| (0.0, 0.0), |_, _| LIGHT);
    k.click((100.0, 50.0));
    k.tool_cursor((100.0, 50.0), Tool::MeshFill);
}

fn mesh_color(k: &K) {
    mesh(
        k,
        |_, _| (0.0, 0.0),
        |i, j| if (i, j) == (2, 1) { RED } else { LIGHT },
    );
    k.node((120.0, 38.33), true);
    for (i, c) in [RED, YELLOW, GREEN, BLUE].iter().enumerate() {
        let x = 168.0 + (i % 2) as f32 * 14.0;
        let y = 60.0 + (i / 2) as f32 * 14.0;
        k.p.rect_filled(
            Rect::from_two_pos(k.pt((x, y)), k.pt((x + 12.0, y + 12.0))),
            1.0,
            *c,
        );
    }
    k.click((174.0, 66.0));
    k.cursor((174.0, 66.0));
}

fn mesh_bend(k: &K) {
    mesh(
        k,
        |i, j| {
            if (i, j) == (2, 1) {
                (14.0, -12.0)
            } else {
                (0.0, 0.0)
            }
        },
        |i, j| match (i, j) {
            (2, 1) => RED,
            (1, 2) => YELLOW,
            _ => LIGHT,
        },
    );
    k.node((134.0, 26.33), true);
    k.drag((120.0, 38.33), (132.0, 28.0));
    k.cursor((136.0, 28.0));
}

// ---- Outline ----

fn pen_dialog(k: &K) {
    k.shape(&ellipse_pts(50.0, 50.0, 28.0, 22.0), Paint::Solid(LIGHT));
    k.handles((22.0, 28.0), (78.0, 72.0));
    // A small dialog box.
    let d = Rect::from_two_pos(k.pt((100.0, 12.0)), k.pt((188.0, 88.0)));
    k.p.rect_filled(d, 3.0, Color32::from_gray(0xF3));
    k.p.rect_stroke(d, 3.0, Stroke::new(1.0, GHOST), egui::StrokeKind::Inside);
    k.p.rect_filled(
        Rect::from_two_pos(k.pt((100.0, 12.0)), k.pt((188.0, 22.0))),
        0.0,
        Color32::from_gray(0xE2),
    );
    for (i, w) in [1.0, 2.5, 4.0].iter().enumerate() {
        let y = 34.0 + i as f32 * 14.0;
        k.line(&[(108.0, y), (160.0, y)], *w, INK);
    }
    k.dashed(&[(108.0, 76.0), (160.0, 76.0)], INK);
    k.p.rect_filled(
        Rect::from_two_pos(k.pt((166.0, 72.0)), k.pt((184.0, 82.0))),
        2.0,
        ACCENT,
    );
}

fn pen_settings(k: &K) {
    let wave1 = wave(20.0, 170.0, 25.0, 6.0, 1.0);
    k.line(&wave1, 4.0, INK);
    let a = wave1[wave1.len() - 2];
    let b = wave1[wave1.len() - 1];
    k.arrowhead(k.pt(a), k.pt((b.0 + 8.0, b.1)), INK);
    let v = wave(20.0, 180.0, 55.0, 6.0, 1.0);
    let pts = k.pts(&v);
    k.p.extend(Shape::dashed_line(
        &pts,
        Stroke::new(k.px(3.0), PURPLE),
        k.px(10.0),
        k.px(5.0),
    ));
    let corners = [(30.0, 92.0), (60.0, 72.0), (90.0, 92.0)];
    k.p.add(PathShape::line(
        k.pts(&corners),
        PathStroke::new(k.px(6.0), RED),
    ));
    k.line(&[(120.0, 82.0), (180.0, 82.0)], 6.0, GREEN);
}

fn outline_color_panel(k: &K) {
    k.fill(&ellipse_pts(55.0, 50.0, 30.0, 26.0), Paint::Solid(LIGHT));
    k.outline(&ellipse_pts(55.0, 50.0, 30.0, 26.0), true, 3.0, RED);
    k.handles((25.0, 24.0), (85.0, 76.0));
    // A color wheel.
    let c = (145.0, 50.0);
    let mut m = Mesh::default();
    m.colored_vertex(k.pt(c), PAGE);
    for i in 0..=36 {
        let a = i as f32 / 36.0 * TAU;
        let hue = egui::ecolor::Hsva::new(i as f32 / 36.0, 0.9, 0.95, 1.0);
        m.colored_vertex(
            k.pt((c.0 + 32.0 * a.cos(), c.1 + 32.0 * a.sin())),
            hue.into(),
        );
    }
    for i in 1..=36u32 {
        m.add_triangle(0, i, i + 1);
    }
    k.p.add(Shape::mesh(m));
    k.p.circle_stroke(k.pt((177.0, 50.0)), k.px(4.0), Stroke::new(1.5, INK));
}

fn outline_color_palette(k: &K) {
    k.fill(&ellipse_pts(70.0, 42.0, 34.0, 26.0), Paint::Solid(LIGHT));
    k.outline(&ellipse_pts(70.0, 42.0, 34.0, 26.0), true, 3.0, GREEN);
    let colors = [INK, RED, ORANGE, YELLOW, GREEN, BLUE, PURPLE];
    for (i, c) in colors.iter().enumerate() {
        let x = 120.0 + i as f32 * 10.0;
        k.p.rect_filled(
            Rect::from_two_pos(k.pt((x, 72.0)), k.pt((x + 9.0, 84.0))),
            0.0,
            *c,
        );
    }
    k.click((164.0, 78.0));
    k.cursor((164.0, 78.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_steps_with_text_and_pictures() {
        crate::i18n::set_language("en");
        for t in Tool::ALL {
            let steps = steps(t);
            assert!(steps.len() >= 2, "{t:?} has fewer than two steps");
            assert!(steps.iter().any(Option::is_some), "{t:?} has no picture");
            for n in 1..=steps.len() {
                let key = format!("hintstep.{}.{n}", t.id());
                assert!(crate::i18n::has_english(&key), "missing {key}");
            }
            let extra = format!("hintstep.{}.{}", t.id(), steps.len() + 1);
            assert!(!crate::i18n::has_english(&extra), "{extra} has no step");
        }
    }

    #[test]
    fn every_picture_draws_on_a_page() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |_| {});
        out.textures_delta.clear();
        let painter = Painter::new(
            ctx,
            egui::LayerId::background(),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0)),
        );
        for t in Tool::ALL {
            for pic in steps(t).iter().flatten() {
                draw(
                    &painter,
                    Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(280.0, 140.0)),
                    *pic,
                );
            }
        }
    }
}
