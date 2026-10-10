//! DXF (ASCII) import and export, the drawing-exchange format of CAD
//! programs.
//!
//! Import reads the HEADER (units, extents), the LAYER table (colours),
//! BLOCKS (for INSERT references) and ENTITIES: LINE, LWPOLYLINE,
//! POLYLINE/VERTEX, CIRCLE, ARC, ELLIPSE, SPLINE, SOLID, 3DFACE, HATCH,
//! TEXT, MTEXT, POINT and INSERT. Everything becomes paths, text and
//! groups in millimetres with the drawing's lower-left extent at the
//! origin. Export writes one LWPOLYLINE (flattened curves) per outline,
//! a solid HATCH per filled object, TEXT for text and a LAYER per layer.
//! Unknown entities and malformed pairs are skipped, never fatal.

use std::collections::HashMap;
use std::fmt::Write as _;
use tracedraw_core::{
    document::{Layer as DocLayer, ParagraphStyle, Shape, ShapeKind, TextSpan},
    geometry::{Affine, BezPath, PathEl, Point, Rect, Size, Vec2},
    id::IdSource,
    Color, Document, Fill, ShapeId, Stroke, TextAlign,
};

/// Result of a DXF import.
#[derive(Debug, Clone)]
pub struct Imported {
    pub shapes: Vec<Shape>,
    pub size: Size,
    /// Layer name per top-level shape (same length as `shapes`).
    pub layers: Vec<String>,
    pub warnings: Vec<String>,
}

/// One group-code / value pair.
#[derive(Debug, Clone)]
struct Pair {
    code: i32,
    value: String,
}

fn pairs(text: &str) -> Vec<Pair> {
    let mut out = Vec::new();
    let mut lines = text.lines();
    while let (Some(c), Some(v)) = (lines.next(), lines.next()) {
        if let Ok(code) = c.trim().parse::<i32>() {
            out.push(Pair {
                code,
                value: v.trim_end_matches('\r').to_string(),
            });
        }
    }
    out
}

/// A raw entity: its type and the pairs after the `0` line.
#[derive(Debug, Clone, Default)]
struct Entity {
    kind: String,
    pairs: Vec<Pair>,
}

impl Entity {
    fn f(&self, code: i32) -> Option<f64> {
        self.pairs
            .iter()
            .find(|p| p.code == code)
            .and_then(|p| p.value.trim().parse().ok())
    }
    fn f_or(&self, code: i32, d: f64) -> f64 {
        self.f(code).unwrap_or(d)
    }
    fn i(&self, code: i32) -> Option<i64> {
        self.pairs
            .iter()
            .find(|p| p.code == code)
            .and_then(|p| p.value.trim().parse().ok())
    }
    fn s(&self, code: i32) -> Option<&str> {
        self.pairs
            .iter()
            .find(|p| p.code == code)
            .map(|p| p.value.as_str())
    }
    fn all_f(&self, code: i32) -> Vec<f64> {
        self.pairs
            .iter()
            .filter(|p| p.code == code)
            .filter_map(|p| p.value.trim().parse().ok())
            .collect()
    }
}

/// Split a flat pair list into entities (each starts with code 0).
fn entities(p: &[Pair]) -> Vec<Entity> {
    let mut out: Vec<Entity> = Vec::new();
    for pair in p {
        if pair.code == 0 {
            out.push(Entity {
                kind: pair.value.trim().to_ascii_uppercase(),
                pairs: Vec::new(),
            });
        } else if let Some(e) = out.last_mut() {
            e.pairs.push(pair.clone());
        }
    }
    out
}

/// AutoCAD colour index to RGB (standard palette approximation).
pub fn aci_to_rgb(index: i64) -> [u8; 3] {
    match index {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        7 => [0, 0, 0],
        8 => [128, 128, 128],
        9 => [192, 192, 192],
        10..=249 => {
            // Hue steps of 15 degrees every ten indices; within a block,
            // pairs of (full, half-saturated) at decreasing brightness.
            let i = index - 10;
            let hue = (i / 10) as f32 * 15.0;
            let shade = (i % 10) as usize;
            let v = [1.0, 1.0, 0.65, 0.65, 0.5, 0.5, 0.35, 0.35, 0.3, 0.3][shade];
            let s = if shade.is_multiple_of(2) { 1.0 } else { 0.5 };
            let (r, g, b) = hsv(hue, s, v);
            [r, g, b]
        }
        250..=255 => {
            let v = [51u8, 91, 132, 173, 214, 255][(index - 250) as usize];
            [v, v, v]
        }
        _ => [0, 0, 0],
    }
}

fn hsv(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let hh = (h / 60.0) % 6.0;
    let x = c * (1.0 - ((hh % 2.0) - 1.0).abs());
    let (r, g, b) = match hh as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let q = |f: f32| ((f + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    (q(r), q(g), q(b))
}

/// Nearest colour index for export (standard seven plus greys).
pub fn rgb_to_aci(rgb: [u8; 3]) -> i64 {
    let mut best = (7, i64::MAX);
    for i in [1, 2, 3, 4, 5, 6, 7, 8, 9, 250, 251, 252, 253, 254, 255] {
        let c = aci_to_rgb(i);
        let d = (0..3).map(|k| (c[k] as i64 - rgb[k] as i64).pow(2)).sum();
        if d < best.1 {
            best = (i, d);
        }
    }
    best.0
}

struct Ctx<'a> {
    ids: &'a mut IdSource,
    unit: f64,
    layer_colors: HashMap<String, i64>,
    blocks: HashMap<String, (Point, Vec<Entity>)>,
    warnings: Vec<String>,
}

impl Ctx<'_> {
    fn warn(&mut self, m: impl Into<String>) {
        let m = m.into();
        if self.warnings.len() < 50 && !self.warnings.contains(&m) {
            self.warnings.push(m);
        }
    }

    fn pt(&self, e: &Entity, xc: i32) -> Point {
        Point::new(
            e.f_or(xc, 0.0) * self.unit,
            e.f_or(xc + 10, 0.0) * self.unit,
        )
    }

    fn color_of(&self, e: &Entity, layer_color_override: Option<i64>) -> Color {
        if let Some(tc) = e.i(420) {
            let v = tc as u32;
            return Color::rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8);
        }
        let aci = e.i(62).unwrap_or(256);
        let aci = if aci == 256 || aci == 0 {
            layer_color_override
                .or_else(|| e.s(8).and_then(|l| self.layer_colors.get(l).copied()))
                .unwrap_or(7)
        } else {
            aci
        };
        let [r, g, b] = aci_to_rgb(aci.abs());
        Color::rgb8(r, g, b)
    }

    fn stroke_of(&self, e: &Entity, color: Color) -> Option<Stroke> {
        // Lineweight 370 is in 1/100 mm; -1/-2/-3 mean by layer/block/default.
        // A polyline's constant width (43, drawing units) wins when set.
        let lw = e.i(370).unwrap_or(-1);
        let mut width = if lw > 0 { lw as f64 / 100.0 } else { 0.0 };
        if e.kind == "LWPOLYLINE" {
            if let Some(w) = e.f(43).filter(|w| *w > 0.0) {
                width = w * self.unit;
            }
        }
        let mut s = Stroke::new(color, width.max(Stroke::HAIRLINE));
        if width <= 0.0 {
            s = Stroke::hairline(color);
        }
        Some(s)
    }

    fn shape(&mut self, kind: ShapeKind, e: &Entity, filled: bool) -> Shape {
        let color = self.color_of(e, None);
        let mut s = Shape::new(ShapeId(self.ids.shape().0), kind);
        if filled {
            s.fill = Fill::Solid(color);
            s.stroke = None;
        } else {
            s.fill = Fill::None;
            s.stroke = self.stroke_of(e, color);
        }
        if let Some(l) = e.s(8) {
            s.data.push(("dxf.layer".into(), l.to_string()));
        }
        s
    }

    fn convert(&mut self, e: &Entity, depth: usize) -> Option<Shape> {
        match e.kind.as_str() {
            "LINE" => {
                let a = self.pt(e, 10);
                let b = self.pt(e, 11);
                let mut p = BezPath::new();
                p.move_to(a);
                p.line_to(b);
                Some(self.shape(
                    ShapeKind::Path {
                        path: p,
                        closed: false,
                    },
                    e,
                    false,
                ))
            }
            "LWPOLYLINE" => {
                let closed = e.i(70).unwrap_or(0) & 1 != 0;
                // Vertices are 10/20 pairs in order; bulges (42) follow their vertex.
                let mut verts: Vec<(Point, f64)> = Vec::new();
                let mut cur: Option<(f64, f64, f64)> = None;
                for p in &e.pairs {
                    match p.code {
                        10 => {
                            if let Some((x, y, b)) = cur.take() {
                                verts.push((Point::new(x * self.unit, y * self.unit), b));
                            }
                            cur = Some((p.value.trim().parse().unwrap_or(0.0), 0.0, 0.0));
                        }
                        20 => {
                            if let Some(c) = cur.as_mut() {
                                c.1 = p.value.trim().parse().unwrap_or(0.0);
                            }
                        }
                        42 => {
                            if let Some(c) = cur.as_mut() {
                                c.2 = p.value.trim().parse().unwrap_or(0.0);
                            }
                        }
                        _ => {}
                    }
                }
                if let Some((x, y, b)) = cur {
                    verts.push((Point::new(x * self.unit, y * self.unit), b));
                }
                let path = polyline_path(&verts, closed);
                (!path.elements().is_empty())
                    .then(|| self.shape(ShapeKind::Path { path, closed }, e, false))
            }
            "CIRCLE" => {
                let c = self.pt(e, 10);
                let r = e.f_or(40, 0.0) * self.unit;
                (r > 0.0).then(|| {
                    self.shape(
                        ShapeKind::Ellipse {
                            rect: Rect::new(c.x - r, c.y - r, c.x + r, c.y + r),
                            arc: None,
                        },
                        e,
                        false,
                    )
                })
            }
            "ARC" => {
                let c = self.pt(e, 10);
                let r = e.f_or(40, 0.0) * self.unit;
                let (a0, a1) = (e.f_or(50, 0.0), e.f_or(51, 360.0));
                (r > 0.0).then(|| {
                    let path = tracedraw_core::geometry::ellipse_arc_path(
                        Rect::new(c.x - r, c.y - r, c.x + r, c.y + r),
                        a0,
                        a1,
                        false,
                    );
                    self.shape(
                        ShapeKind::Path {
                            path,
                            closed: false,
                        },
                        e,
                        false,
                    )
                })
            }
            "ELLIPSE" => {
                let c = self.pt(e, 10);
                let major = Vec2::new(e.f_or(11, 1.0), e.f_or(21, 0.0)) * self.unit;
                let ratio = e.f_or(40, 1.0);
                let (t0, t1) = (e.f_or(41, 0.0), e.f_or(42, std::f64::consts::TAU));
                let a = major.hypot();
                let b = a * ratio;
                let rot = major.y.atan2(major.x);
                let full = (t1 - t0).abs() >= std::f64::consts::TAU - 1e-6;
                let rect = Rect::new(-a, -b, a, b);
                let local = if full {
                    tracedraw_core::geometry::ellipse_path(rect)
                } else {
                    tracedraw_core::geometry::ellipse_arc_path(
                        rect,
                        t0.to_degrees(),
                        t1.to_degrees(),
                        false,
                    )
                };
                let path = Affine::translate(c.to_vec2()) * Affine::rotate(rot) * local;
                Some(self.shape(ShapeKind::Path { path, closed: full }, e, false))
            }
            "SPLINE" => {
                let degree = e.i(71).unwrap_or(3).clamp(1, 3) as usize;
                let closed = e.i(70).unwrap_or(0) & 1 != 0;
                let knots = e.all_f(40);
                let xs = e.all_f(10);
                let ys = e.all_f(20);
                let ctrl: Vec<Point> = xs
                    .iter()
                    .zip(ys.iter())
                    .map(|(x, y)| Point::new(x * self.unit, y * self.unit))
                    .collect();
                let fit: Vec<Point> = e
                    .all_f(11)
                    .iter()
                    .zip(e.all_f(21).iter())
                    .map(|(x, y)| Point::new(x * self.unit, y * self.unit))
                    .collect();
                let path = if ctrl.len() > degree {
                    nurbs_path(&ctrl, &knots, degree)
                } else if fit.len() >= 2 {
                    polyline_path(&fit.iter().map(|p| (*p, 0.0)).collect::<Vec<_>>(), closed)
                } else {
                    BezPath::new()
                };
                (!path.elements().is_empty())
                    .then(|| self.shape(ShapeKind::Path { path, closed }, e, false))
            }
            "SOLID" | "3DFACE" | "TRACE" => {
                let p0 = self.pt(e, 10);
                let p1 = self.pt(e, 11);
                let p2 = self.pt(e, 12);
                let p3 = if e.f(13).is_some() {
                    self.pt(e, 13)
                } else {
                    p2
                };
                let mut path = BezPath::new();
                path.move_to(p0);
                path.line_to(p1);
                // SOLID stores its corners in a bow-tie order.
                path.line_to(p3);
                path.line_to(p2);
                path.close_path();
                Some(self.shape(ShapeKind::Path { path, closed: true }, e, e.kind == "SOLID"))
            }
            "HATCH" => {
                let solid = e.i(70).unwrap_or(0) == 1;
                let path = self.hatch_path(e);
                if path.elements().is_empty() {
                    return None;
                }
                let mut s = self.shape(ShapeKind::Path { path, closed: true }, e, solid);
                if !solid {
                    // Pattern hatches: outline only, 50 % tint of the colour.
                    let c = self.color_of(e, None);
                    s.fill = Fill::Solid(c);
                    s.opacity = 0.3;
                    s.stroke = self.stroke_of(e, c);
                }
                Some(s)
            }
            "TEXT" | "MTEXT" => self.text(e),
            "POINT" => {
                let c = self.pt(e, 10);
                let r = 0.25;
                Some(self.shape(
                    ShapeKind::Ellipse {
                        rect: Rect::new(c.x - r, c.y - r, c.x + r, c.y + r),
                        arc: None,
                    },
                    e,
                    true,
                ))
            }
            "INSERT" => {
                if depth > 16 {
                    self.warn("block references nested too deeply");
                    return None;
                }
                let name = e.s(2).unwrap_or("").to_string();
                let Some((base, ents)) = self.blocks.get(&name).cloned() else {
                    self.warn(format!("block {name} not found"));
                    return None;
                };
                let pos = self.pt(e, 10);
                let sx = e.f_or(41, 1.0);
                let sy = e.f_or(42, 1.0);
                let rot = e.f_or(50, 0.0).to_radians();
                let m = Affine::translate(pos.to_vec2())
                    * Affine::rotate(rot)
                    * Affine::scale_non_uniform(sx, sy)
                    * Affine::translate(-base.to_vec2());
                let layer_color = e.s(8).and_then(|l| self.layer_colors.get(l).copied());
                let mut children = Vec::new();
                for be in &ents {
                    if let Some(mut c) = self.convert(be, depth + 1) {
                        // Colour "by block" (0) follows the insert's layer.
                        if be.i(62) == Some(0) {
                            let col = self.color_of(e, layer_color);
                            if let Some(s) = c.stroke.as_mut() {
                                s.color = col;
                            }
                            if matches!(c.fill, Fill::Solid(_)) {
                                c.fill = Fill::Solid(col);
                            }
                        }
                        children.push(c);
                    }
                }
                if children.is_empty() {
                    return None;
                }
                let mut g = Shape::new(ShapeId(self.ids.shape().0), ShapeKind::Group { children });
                g.transform = m;
                g.fill = Fill::None;
                g.stroke = None;
                g.name = Some(name);
                Some(g)
            }
            "POLYLINE" | "VERTEX" | "SEQEND" | "ATTRIB" | "ATTDEF" | "DIMENSION" | "VIEWPORT"
            | "IMAGE" | "LEADER" | "MLINE" | "REGION" | "BODY" | "3DSOLID" | "XLINE" | "RAY" => {
                if matches!(e.kind.as_str(), "DIMENSION" | "IMAGE" | "LEADER" | "MLINE") {
                    self.warn(format!("{} entities are not imported", e.kind));
                }
                None
            }
            other => {
                self.warn(format!("{other} entities are not imported"));
                None
            }
        }
    }

    fn hatch_path(&mut self, e: &Entity) -> BezPath {
        // Boundary paths: code 92 starts a path (flags), then edges (72 type)
        // or polyline vertices when flag 2 is set.
        let mut path = BezPath::new();
        let mut i = 0;
        let p = &e.pairs;
        let unit = self.unit;
        let num = |k: usize| -> f64 {
            p.get(k)
                .and_then(|x| x.value.trim().parse().ok())
                .unwrap_or(0.0)
        };
        while i < p.len() {
            if p[i].code != 92 {
                i += 1;
                continue;
            }
            let flags = p[i].value.trim().parse::<i64>().unwrap_or(0);
            i += 1;
            if flags & 2 != 0 {
                // Polyline boundary: 72 has-bulge, 73 closed, 93 count, then 10/20(/42).
                let mut verts: Vec<(Point, f64)> = Vec::new();
                let mut closed = true;
                while i < p.len() && p[i].code != 92 && p[i].code != 75 && p[i].code != 97 {
                    match p[i].code {
                        73 => closed = num(i) != 0.0,
                        10 => {
                            let x = num(i);
                            let y = if p.get(i + 1).map(|q| q.code) == Some(20) {
                                num(i + 1)
                            } else {
                                0.0
                            };
                            let mut b = 0.0;
                            if p.get(i + 2).map(|q| q.code) == Some(42) {
                                b = num(i + 2);
                            }
                            verts.push((Point::new(x * unit, y * unit), b));
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let _ = closed;
                path.extend(polyline_path(&verts, true));
            } else {
                // Edge list: 93 count, then per edge 72 type.
                let mut started = false;
                while i < p.len() && p[i].code != 92 && p[i].code != 75 && p[i].code != 97 {
                    if p[i].code == 72 {
                        let ty = num(i) as i64;
                        match ty {
                            1 => {
                                // Line: 10 20 11 21
                                let a = Point::new(num(i + 1) * unit, num(i + 2) * unit);
                                let b = Point::new(num(i + 3) * unit, num(i + 4) * unit);
                                if !started {
                                    path.move_to(a);
                                    started = true;
                                }
                                path.line_to(b);
                                i += 5;
                                continue;
                            }
                            2 => {
                                // Arc: 10 20 centre, 40 radius, 50 51 angles, 73 ccw
                                let c = Point::new(num(i + 1) * unit, num(i + 2) * unit);
                                let r = num(i + 3) * unit;
                                let (a0, a1) = (num(i + 4), num(i + 5));
                                let ccw = num(i + 6) != 0.0;
                                let (s0, s1) = if ccw { (a0, a1) } else { (-a1, -a0) };
                                let arc = tracedraw_core::geometry::ellipse_arc_path(
                                    Rect::new(c.x - r, c.y - r, c.x + r, c.y + r),
                                    s0,
                                    s1,
                                    false,
                                );
                                append_continuing(&mut path, &arc, &mut started);
                                i += 7;
                                continue;
                            }
                            3 => {
                                // Ellipse arc: centre, major end, ratio, angles, ccw
                                let c = Point::new(num(i + 1) * unit, num(i + 2) * unit);
                                let major = Vec2::new(num(i + 3), num(i + 4)) * unit;
                                let ratio = num(i + 5);
                                let (a0, a1) = (num(i + 6), num(i + 7));
                                let a = major.hypot();
                                let b = a * ratio;
                                let local = tracedraw_core::geometry::ellipse_arc_path(
                                    Rect::new(-a, -b, a, b),
                                    a0,
                                    a1,
                                    false,
                                );
                                let arc = Affine::translate(c.to_vec2())
                                    * Affine::rotate(major.y.atan2(major.x))
                                    * local;
                                append_continuing(&mut path, &arc, &mut started);
                                i += 9;
                                continue;
                            }
                            4 => {
                                // Spline edge: 94 degree, 73, 74, 95 knots, 96 ctrl, 40 knots, 10/20 ctrl
                                let mut k = i + 1;
                                let mut degree = 3usize;
                                let mut knots = Vec::new();
                                let mut ctrl = Vec::new();
                                while k < p.len()
                                    && p[k].code != 72
                                    && p[k].code != 92
                                    && p[k].code != 97
                                    && p[k].code != 75
                                {
                                    match p[k].code {
                                        94 => degree = (num(k) as usize).clamp(1, 3),
                                        40 => knots.push(num(k)),
                                        10 => {
                                            let y = if p.get(k + 1).map(|q| q.code) == Some(20) {
                                                num(k + 1)
                                            } else {
                                                0.0
                                            };
                                            ctrl.push(Point::new(num(k) * unit, y * unit));
                                        }
                                        _ => {}
                                    }
                                    k += 1;
                                }
                                if ctrl.len() > degree {
                                    let sp = nurbs_path(&ctrl, &knots, degree);
                                    append_continuing(&mut path, &sp, &mut started);
                                }
                                i = k;
                                continue;
                            }
                            _ => {}
                        }
                    }
                    i += 1;
                }
                if started {
                    path.close_path();
                }
            }
        }
        path
    }

    fn text(&mut self, e: &Entity) -> Option<Shape> {
        let mut text = e.s(1).unwrap_or("").to_string();
        // MTEXT continues long strings in code 3 chunks before the final 1.
        let extra: String = e
            .pairs
            .iter()
            .filter(|p| p.code == 3)
            .map(|p| p.value.as_str())
            .collect();
        if !extra.is_empty() {
            text = format!("{extra}{text}");
        }
        let text = strip_mtext_codes(&text);
        if text.trim().is_empty() {
            return None;
        }
        let height = e.f_or(40, 2.5) * self.unit;
        let size_pt = (height / 25.4 * 72.0).max(0.5);
        let origin = self.pt(e, 10);
        let rotation = if e.kind == "MTEXT" {
            // Direction vector 11/21 wins over 50.
            match (e.f(11), e.f(21)) {
                (Some(x), Some(y)) if x != 0.0 || y != 0.0 => y.atan2(x).to_degrees(),
                _ => e.f_or(50, 0.0),
            }
        } else {
            e.f_or(50, 0.0)
        };
        let family = e
            .s(7)
            .filter(|s| !s.is_empty() && *s != "Standard" && *s != "STANDARD")
            .unwrap_or("Arial")
            .to_string();
        let align = if e.kind == "MTEXT" {
            match e.i(71).unwrap_or(1) % 3 {
                2 => TextAlign::Center,
                0 => TextAlign::Right,
                _ => TextAlign::Left,
            }
        } else {
            match e.i(72).unwrap_or(0) {
                1 | 4 => TextAlign::Center,
                2 => TextAlign::Right,
                _ => TextAlign::Left,
            }
        };
        // Aligned TEXT uses the second alignment point as its anchor.
        let origin = if e.kind == "TEXT" && e.i(72).unwrap_or(0) != 0 && e.f(11).is_some() {
            self.pt(e, 11)
        } else {
            origin
        };
        // MTEXT anchors at the top of the first line; drop by one line height.
        let origin = if e.kind == "MTEXT" {
            let attach = e.i(71).unwrap_or(1);
            let lines = text.lines().count().max(1) as f64;
            let dy = match attach {
                1..=3 => height,
                4..=6 => height - lines * height * 0.5,
                _ => -(lines - 1.0) * height,
            };
            Point::new(origin.x, origin.y - dy)
        } else {
            origin
        };
        let frame = if e.kind == "MTEXT" {
            e.f(41).filter(|w| *w > 0.0).map(|w| {
                Size::new(
                    w * self.unit,
                    height * 1.4 * text.lines().count().max(1) as f64,
                )
            })
        } else {
            None
        };
        let span = TextSpan::new(text, family, size_pt);
        let mut s = self.shape(
            ShapeKind::Text {
                spans: vec![span],
                origin: Point::ZERO,
                frame,
                align,
                para: ParagraphStyle::default(),
                on_path: None,
            },
            e,
            true,
        );
        s.transform = Affine::translate(origin.to_vec2()) * Affine::rotate(rotation.to_radians());
        Some(s)
    }
}

/// Continue `path` with `piece`, joining when its start meets the end.
fn append_continuing(path: &mut BezPath, piece: &BezPath, started: &mut bool) {
    for el in piece.elements() {
        match el {
            PathEl::MoveTo(p) => {
                if !*started {
                    path.move_to(*p);
                    *started = true;
                } else {
                    path.line_to(*p);
                }
            }
            PathEl::ClosePath => {}
            other => path.push(*other),
        }
    }
}

/// Strip MTEXT formatting codes (\P paragraph, \fArial|b1; font, {} groups).
pub fn strip_mtext_codes(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if i + 1 < chars.len() => {
                let n = chars[i + 1];
                match n {
                    'P' => {
                        out.push('\n');
                        i += 2;
                    }
                    '~' => {
                        out.push('\u{a0}');
                        i += 2;
                    }
                    '\\' | '{' | '}' => {
                        out.push(n);
                        i += 2;
                    }
                    'f' | 'F' | 'H' | 'W' | 'Q' | 'A' | 'C' | 'T' | 'p' | 'S' => {
                        // Codes with an argument up to ';'.
                        let mut k = i + 2;
                        while k < chars.len() && chars[k] != ';' {
                            k += 1;
                        }
                        if n == 'S' {
                            // Stacked fraction: keep "a/b".
                            let inner: String = chars[i + 2..k.min(chars.len())].iter().collect();
                            out.push_str(&inner.replace(['^', '#'], "/"));
                        }
                        i = k + 1;
                    }
                    _ => i += 2, // \L \O \K toggles and unknown codes
                }
            }
            '{' | '}' => i += 1,
            '%' if i + 2 < chars.len() && chars[i + 1] == '%' => {
                // %%d degree, %%p plus-minus, %%c diameter, %%u underline toggle
                match chars[i + 2].to_ascii_lowercase() {
                    'd' => out.push('\u{b0}'),
                    'p' => out.push('\u{b1}'),
                    'c' => out.push('\u{2205}'),
                    'u' | 'o' => {}
                    other => {
                        out.push('%');
                        out.push('%');
                        out.push(other);
                    }
                }
                i += 3;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// Polyline with bulges (tangent of a quarter of the arc's angle) to a path.
fn polyline_path(verts: &[(Point, f64)], closed: bool) -> BezPath {
    let mut path = BezPath::new();
    if verts.is_empty() {
        return path;
    }
    path.move_to(verts[0].0);
    let n = verts.len();
    let last = if closed { n } else { n - 1 };
    for i in 0..last {
        let (a, bulge) = verts[i];
        let b = verts[(i + 1) % n].0;
        if bulge.abs() < 1e-9 || (b - a).hypot() < 1e-9 {
            path.line_to(b);
        } else {
            bulge_arc(&mut path, a, b, bulge);
        }
    }
    if closed {
        path.close_path();
    }
    path
}

/// Arc from `a` to `b` with the given bulge, as cubic Beziers.
fn bulge_arc(path: &mut BezPath, a: Point, b: Point, bulge: f64) {
    let theta = 4.0 * bulge.atan(); // included angle, signed
    let chord = b - a;
    let d = chord.hypot();
    let r = d / (2.0 * (theta / 2.0).sin().abs());
    let mid = a.midpoint(b);
    // Centre is offset from the chord midpoint along its normal.
    let h = (r * r - (d / 2.0).powi(2)).max(0.0).sqrt();
    let n = Vec2::new(-chord.y, chord.x) / d;
    let side = if theta.abs() <= std::f64::consts::PI {
        1.0
    } else {
        -1.0
    };
    let center = mid + n * h * side * theta.signum();
    let a0 = (a - center).atan2();
    let segs = ((theta.abs() / (std::f64::consts::FRAC_PI_2)).ceil() as usize).max(1);
    let step = theta / segs as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let mut ang = a0;
    for _ in 0..segs {
        let p0 = center + Vec2::from_angle(ang) * r;
        let p3 = center + Vec2::from_angle(ang + step) * r;
        let t0 = Vec2::new(-ang.sin(), ang.cos()) * r * k;
        let t1 = Vec2::new(-(ang + step).sin(), (ang + step).cos()) * r * k;
        path.curve_to(p0 + t0, p3 - t1, p3);
        ang += step;
    }
}

/// Evaluate a B-spline (uniform weights) into a polyline-based path.
fn nurbs_path(ctrl: &[Point], knots: &[f64], degree: usize) -> BezPath {
    let n = ctrl.len();
    let knots: Vec<f64> = if knots.len() == n + degree + 1 {
        knots.to_vec()
    } else {
        // Clamped uniform knot vector when the file's is unusable.
        let inner = n - degree;
        let mut k = vec![0.0; degree + 1];
        for i in 1..inner {
            k.push(i as f64 / inner as f64);
        }
        k.extend(std::iter::repeat_n(1.0, degree + 1));
        k
    };
    let (t0, t1) = (knots[degree], knots[n]);
    let steps = (n * 8).clamp(16, 2000);
    let mut path = BezPath::new();
    for s in 0..=steps {
        let t = t0 + (t1 - t0) * s as f64 / steps as f64;
        let p = de_boor(ctrl, &knots, degree, t.min(t1 - 1e-12));
        if s == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path
}

fn de_boor(ctrl: &[Point], knots: &[f64], p: usize, t: f64) -> Point {
    let n = ctrl.len();
    let mut k = p;
    while k + 1 < n && t >= knots[k + 1] {
        k += 1;
    }
    let mut d: Vec<Point> = (0..=p)
        .map(|j| ctrl[(k + j).saturating_sub(p).min(n - 1)])
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = k + j - p;
            let (ka, kb) = (knots[i], knots[i + 1 + p - r]);
            let alpha = if (kb - ka).abs() < 1e-12 {
                0.0
            } else {
                (t - ka) / (kb - ka)
            };
            d[j] = Point::new(
                (1.0 - alpha) * d[j - 1].x + alpha * d[j].x,
                (1.0 - alpha) * d[j - 1].y + alpha * d[j].y,
            );
        }
    }
    d[p]
}

/// Parse DXF text.
pub fn parse(text: &str, ids: &mut IdSource) -> Result<Imported, String> {
    let all = pairs(text);
    if all.is_empty()
        || !all
            .iter()
            .any(|p| p.code == 0 && p.value.trim() == "SECTION")
    {
        return Err("not a DXF file".into());
    }
    // Sections.
    let mut sections: Vec<(String, Vec<Pair>)> = Vec::new();
    let mut i = 0;
    while i < all.len() {
        if all[i].code == 0 && all[i].value.trim() == "SECTION" {
            let name = all
                .get(i + 1)
                .filter(|p| p.code == 2)
                .map(|p| p.value.trim().to_ascii_uppercase())
                .unwrap_or_default();
            let start = (i + 2).min(all.len());
            let mut end = start;
            while end < all.len() && !(all[end].code == 0 && all[end].value.trim() == "ENDSEC") {
                end += 1;
            }
            sections.push((name, all[start..end].to_vec()));
            i = end + 1;
        } else {
            i += 1;
        }
    }
    let section = |name: &str| {
        sections
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, p)| p.as_slice())
    };

    // Header: units and extents.
    let mut unit = 1.0;
    let mut ext: Option<(Point, Point)> = None;
    if let Some(h) = section("HEADER") {
        let mut var = String::new();
        let (mut min, mut max) = (Point::ZERO, Point::ZERO);
        let (mut have_min, mut have_max) = (false, false);
        for p in h {
            if p.code == 9 {
                var = p.value.trim().to_string();
            } else if var == "$INSUNITS" && p.code == 70 {
                unit = match p.value.trim().parse::<i64>().unwrap_or(0) {
                    1 => 25.4,
                    2 => 304.8,
                    3 => 1_609_344.0,
                    4 => 1.0,
                    5 => 10.0,
                    6 => 1000.0,
                    8 => 0.0254,
                    9 => 0.0254 * 1000.0,
                    10 => 914.4,
                    13 => 0.001,
                    14 => 0.1,
                    _ => 1.0,
                };
            } else if var == "$EXTMIN" {
                if p.code == 10 {
                    min.x = p.value.trim().parse().unwrap_or(0.0);
                    have_min = true;
                } else if p.code == 20 {
                    min.y = p.value.trim().parse().unwrap_or(0.0);
                }
            } else if var == "$EXTMAX" {
                if p.code == 10 {
                    max.x = p.value.trim().parse().unwrap_or(0.0);
                    have_max = true;
                } else if p.code == 20 {
                    max.y = p.value.trim().parse().unwrap_or(0.0);
                }
            }
        }
        if have_min && have_max && max.x > min.x && max.y > min.y && max.x - min.x < 1e7 {
            ext = Some((min, max));
        }
    }
    let mut ctx = Ctx {
        ids,
        unit,
        layer_colors: HashMap::new(),
        blocks: HashMap::new(),
        warnings: Vec::new(),
    };
    // Layer colours.
    if let Some(t) = section("TABLES") {
        for e in entities(t) {
            if e.kind == "LAYER" {
                if let (Some(name), Some(c)) = (e.s(2), e.i(62)) {
                    ctx.layer_colors.insert(name.to_string(), c.abs());
                }
            }
        }
    }
    // Blocks.
    if let Some(b) = section("BLOCKS") {
        let ents = entities(b);
        let mut cur: Option<(String, Point, Vec<Entity>)> = None;
        for e in ents {
            match e.kind.as_str() {
                "BLOCK" => {
                    let name = e.s(2).unwrap_or("").to_string();
                    let base = Point::new(e.f_or(10, 0.0) * unit, e.f_or(20, 0.0) * unit);
                    cur = Some((name, base, Vec::new()));
                }
                "ENDBLK" => {
                    if let Some((n, b, es)) = cur.take() {
                        ctx.blocks.insert(n, (b, es));
                    }
                }
                _ => {
                    if let Some(c) = cur.as_mut() {
                        c.2.push(e);
                    }
                }
            }
        }
    }
    // Entities, with old-style POLYLINE/VERTEX sequences folded.
    let Some(ents_pairs) = section("ENTITIES") else {
        return Err("DXF has no ENTITIES section".into());
    };
    let ents = fold_polylines(entities(ents_pairs));
    let mut shapes = Vec::new();
    let mut layers = Vec::new();
    for e in &ents {
        if let Some(s) = ctx.convert(e, 0) {
            layers.push(e.s(8).unwrap_or("0").to_string());
            shapes.push(s);
        }
    }
    // Page: the extents, else the content bounds, with the origin moved to
    // the lower-left corner.
    let bounds = shapes
        .iter()
        .map(|s| s.bounds())
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::new(0.0, 0.0, 210.0, 297.0));
    let (min, max) = match ext {
        Some((a, b)) => (
            Point::new(a.x * unit, a.y * unit),
            Point::new(b.x * unit, b.y * unit),
        ),
        None => (
            Point::new(bounds.x0, bounds.y0),
            Point::new(bounds.x1, bounds.y1),
        ),
    };
    let margin = 5.0;
    let shift = Affine::translate((-min.x + margin, -min.y + margin));
    for s in shapes.iter_mut() {
        s.transform = shift * s.transform;
    }
    let size = Size::new(
        (max.x - min.x + 2.0 * margin).max(10.0),
        (max.y - min.y + 2.0 * margin).max(10.0),
    );
    Ok(Imported {
        shapes,
        size,
        layers,
        warnings: ctx.warnings,
    })
}

/// Turn POLYLINE ... VERTEX ... SEQEND runs into LWPOLYLINE-like entities.
fn fold_polylines(ents: Vec<Entity>) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut cur: Option<Entity> = None;
    for e in ents {
        match e.kind.as_str() {
            "POLYLINE" => {
                let mut lw = Entity {
                    kind: "LWPOLYLINE".into(),
                    pairs: e
                        .pairs
                        .iter()
                        .filter(|p| p.code != 10 && p.code != 20)
                        .cloned()
                        .collect(),
                };
                // Closed flag is bit 1 of code 70 for both.
                lw.pairs.retain(|p| p.code != 70);
                lw.pairs.push(Pair {
                    code: 70,
                    value: e.i(70).unwrap_or(0).to_string(),
                });
                cur = Some(lw);
            }
            "VERTEX" => {
                if let Some(c) = cur.as_mut() {
                    for code in [10, 20, 42] {
                        if let Some(p) = e.pairs.iter().find(|p| p.code == code) {
                            c.pairs.push(p.clone());
                        }
                    }
                }
            }
            "SEQEND" => {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
            }
            _ => {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
                out.push(e);
            }
        }
    }
    if let Some(c) = cur {
        out.push(c);
    }
    out
}

/// Build a document from an import (one layer per DXF layer, in order).
pub fn to_document(imp: Imported, title: &str) -> Document {
    let mut doc = Document::new(title, imp.size);
    let page = doc.pages[0].id;
    let mut ids = doc.ids().clone();
    let mut by_layer: Vec<(String, Vec<Shape>)> = Vec::new();
    for (s, l) in imp.shapes.into_iter().zip(imp.layers) {
        match by_layer.iter_mut().find(|(n, _)| *n == l) {
            Some((_, v)) => v.push(s),
            None => by_layer.push((l, vec![s])),
        }
    }
    if let Ok(p) = doc.page_mut(page) {
        p.layers.clear();
        for (name, shapes) in by_layer {
            let mut layer = DocLayer::new(ids.layer(), name);
            layer.shapes = shapes;
            p.layers.push(layer);
        }
        if p.layers.is_empty() {
            p.layers.push(DocLayer::new(ids.layer(), "0"));
        }
    }
    doc.set_ids(ids);
    doc
}

// ----- export ---------------------------------------------------------------

fn fmt(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

struct Writer<'a> {
    out: String,
    symbols: &'a [tracedraw_core::Symbol],
    handle: u32,
    layer: String,
}

impl Writer<'_> {
    fn pair(&mut self, code: i32, value: &str) {
        let _ = writeln!(self.out, "{code:>3}\n{value}");
    }

    fn next_handle(&mut self) -> String {
        self.handle += 1;
        format!("{:X}", self.handle)
    }

    fn entity_header(&mut self, kind: &str, color: Color, subclass: &str) {
        self.entity_header_lw(kind, color, subclass, 0.0);
    }

    fn entity_header_lw(&mut self, kind: &str, color: Color, subclass: &str, width_mm: f64) {
        self.pair(0, kind);
        let h = self.next_handle();
        self.pair(5, &h);
        self.pair(100, "AcDbEntity");
        let layer = self.layer.clone();
        self.pair(8, &layer);
        let rgb = color.to_rgb8();
        self.pair(62, &rgb_to_aci(rgb).to_string());
        let tc = ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32;
        self.pair(420, &tc.to_string());
        if width_mm > 0.0 {
            // Lineweight in 1/100 mm (nearest standard value is the
            // reader's business).
            self.pair(
                370,
                &((width_mm * 100.0).round() as i64)
                    .clamp(0, 211)
                    .to_string(),
            );
        }
        self.pair(100, subclass);
    }

    /// Flatten a path into polylines (one per subpath).
    fn polylines(path: &BezPath) -> Vec<(Vec<Point>, bool)> {
        let mut out: Vec<(Vec<Point>, bool)> = Vec::new();
        let mut cur: Vec<Point> = Vec::new();
        let mut closed = false;
        let flush = |cur: &mut Vec<Point>, closed: bool, out: &mut Vec<(Vec<Point>, bool)>| {
            if cur.len() >= 2 {
                out.push((std::mem::take(cur), closed));
            } else {
                cur.clear();
            }
        };
        tracedraw_core::geometry::flatten(path, 0.05, &mut |el| match el {
            PathEl::MoveTo(p) => {
                flush(&mut cur, closed, &mut out);
                closed = false;
                cur.push(p);
            }
            PathEl::LineTo(p) => cur.push(p),
            PathEl::ClosePath => {
                closed = true;
                flush(&mut cur, true, &mut out);
                closed = false;
            }
            _ => {}
        });
        flush(&mut cur, closed, &mut out);
        out
    }

    fn lwpolyline(&mut self, pts: &[Point], closed: bool, color: Color, width_mm: f64) {
        self.entity_header_lw("LWPOLYLINE", color, "AcDbPolyline", width_mm);
        self.pair(90, &pts.len().to_string());
        self.pair(70, if closed { "1" } else { "0" });
        if width_mm > 0.0 {
            self.pair(43, &fmt(width_mm));
        }
        for p in pts {
            self.pair(10, &fmt(p.x));
            self.pair(20, &fmt(p.y));
        }
    }

    fn hatch(&mut self, polys: &[(Vec<Point>, bool)], color: Color) {
        if polys.is_empty() {
            return;
        }
        self.entity_header("HATCH", color, "AcDbHatch");
        self.pair(10, "0");
        self.pair(20, "0");
        self.pair(30, "0");
        self.pair(210, "0");
        self.pair(220, "0");
        self.pair(230, "1");
        self.pair(2, "SOLID");
        self.pair(70, "1"); // solid fill
        self.pair(71, "0"); // not associative
        self.pair(91, &polys.len().to_string());
        for (pts, _) in polys {
            self.pair(92, "3"); // external polyline
            self.pair(72, "0"); // no bulges
            self.pair(73, "1"); // closed
            self.pair(93, &pts.len().to_string());
            for p in pts {
                self.pair(10, &fmt(p.x));
                self.pair(20, &fmt(p.y));
            }
            self.pair(97, "0");
        }
        self.pair(75, "0");
        self.pair(76, "1");
        self.pair(98, "0");
    }

    fn shape(&mut self, shape: &Shape, parent: Affine) {
        if !shape.visible {
            return;
        }
        if !shape.effects.is_empty() {
            let ev = tracedraw_core::live::evaluate(shape);
            for s in ev
                .below
                .iter()
                .chain(std::iter::once(&ev.main))
                .chain(ev.above.iter())
            {
                let mut s = s.clone();
                s.effects.clear();
                self.shape(&s, parent);
            }
            return;
        }
        let transform = parent * shape.transform;
        match &shape.kind {
            ShapeKind::Group { children } => {
                for c in children {
                    self.shape(c, transform);
                }
                return;
            }
            ShapeKind::Table(_) | ShapeKind::SymbolInstance { .. } => {
                for c in shape.expand(self.symbols) {
                    self.shape(&c, transform);
                }
                return;
            }
            ShapeKind::ClipFrame { frame, contents } => {
                // No clipping in DXF: the frame, then the contents.
                self.shape(frame, transform);
                for c in contents {
                    self.shape(c, transform);
                }
                return;
            }
            ShapeKind::Text {
                spans,
                origin,
                align,
                ..
            } => {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                let size_pt = spans.first().map(|s| s.size_pt).unwrap_or(12.0);
                let color = match &shape.fill {
                    Fill::Solid(c) => *c,
                    _ => Color::BLACK,
                };
                let o = transform * *origin;
                let c = transform.as_coeffs();
                let rot = c[1].atan2(c[0]).to_degrees();
                let scale = (c[0] * c[3] - c[1] * c[2]).abs().sqrt();
                for (i, line) in text.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let h = size_pt / 72.0 * 25.4 * scale;
                    let dy = -(i as f64) * h * 1.2;
                    let p = o + Vec2::new(-rot.to_radians().sin(), rot.to_radians().cos()) * dy;
                    self.entity_header("TEXT", color, "AcDbText");
                    self.pair(10, &fmt(p.x));
                    self.pair(20, &fmt(p.y));
                    self.pair(30, "0");
                    self.pair(40, &fmt(h));
                    self.pair(1, &escape_text(line));
                    self.pair(50, &fmt(rot));
                    let hj = match align {
                        TextAlign::Center => 1,
                        TextAlign::Right => 2,
                        _ => 0,
                    };
                    if hj != 0 {
                        self.pair(72, &hj.to_string());
                        self.pair(11, &fmt(p.x));
                        self.pair(21, &fmt(p.y));
                        self.pair(31, "0");
                    }
                    self.pair(100, "AcDbText");
                }
                return;
            }
            ShapeKind::Bitmap { .. } => {
                // Bitmaps have no vector form; their frame is written.
            }
            _ => {}
        }
        let path = transform * shape.local_path();
        if path.elements().is_empty() {
            return;
        }
        let polys = Self::polylines(&path);
        if let Fill::Solid(c) = &shape.fill {
            let closed: Vec<(Vec<Point>, bool)> = polys
                .iter()
                .filter(|(p, _)| p.len() >= 3)
                .cloned()
                .collect();
            self.hatch(&closed, *c);
        } else if !matches!(shape.fill, Fill::None) {
            // Fountain, pattern, texture and mesh fills: their first colour.
            let c = match &shape.fill {
                Fill::Fountain(f) => f.stops.first().map(|s| s.color),
                Fill::Pattern(tracedraw_core::Pattern::TwoColor { front, .. }) => Some(*front),
                Fill::Texture(t) => Some(t.color_a),
                Fill::Mesh(m) => m.nodes.first().map(|n| n.color),
                _ => None,
            }
            .unwrap_or(Color::Gray { v: 0.5 });
            let closed: Vec<(Vec<Point>, bool)> = polys
                .iter()
                .filter(|(p, _)| p.len() >= 3)
                .cloned()
                .collect();
            self.hatch(&closed, c);
        }
        if let Some(s) = &shape.stroke {
            let scale = {
                let c = transform.as_coeffs();
                (c[0] * c[3] - c[1] * c[2]).abs().sqrt()
            };
            let w = if s.scale_with_object {
                s.width * scale
            } else {
                s.width
            };
            let w = if w <= Stroke::HAIRLINE + 1e-9 { 0.0 } else { w };
            for (pts, closed) in &polys {
                self.lwpolyline(pts, *closed, s.color, w);
            }
        }
    }
}

fn escape_text(s: &str) -> String {
    s.replace('\\', "\\\\")
}

/// Write one page as an ASCII DXF (AC1015 / AutoCAD 2000 dialect).
pub fn page_to_dxf(doc: &Document, page_index: usize) -> String {
    let mut w = Writer {
        out: String::new(),
        symbols: &doc.symbols,
        handle: 0x100,
        layer: "0".into(),
    };
    let Some(page) = doc.pages.get(page_index) else {
        return String::new();
    };
    let layers = doc.layers_for_page(page.id).unwrap_or_default();
    // HEADER
    w.pair(0, "SECTION");
    w.pair(2, "HEADER");
    w.pair(9, "$ACADVER");
    w.pair(1, "AC1015");
    w.pair(9, "$INSUNITS");
    w.pair(70, "4");
    w.pair(9, "$EXTMIN");
    w.pair(10, "0");
    w.pair(20, "0");
    w.pair(30, "0");
    w.pair(9, "$EXTMAX");
    w.pair(10, &fmt(page.size.width));
    w.pair(20, &fmt(page.size.height));
    w.pair(30, "0");
    w.pair(9, "$LIMMIN");
    w.pair(10, "0");
    w.pair(20, "0");
    w.pair(9, "$LIMMAX");
    w.pair(10, &fmt(page.size.width));
    w.pair(20, &fmt(page.size.height));
    w.pair(0, "ENDSEC");
    // TABLES: line types and layers.
    w.pair(0, "SECTION");
    w.pair(2, "TABLES");
    w.pair(0, "TABLE");
    w.pair(2, "LTYPE");
    w.pair(5, "5");
    w.pair(100, "AcDbSymbolTable");
    w.pair(70, "1");
    w.pair(0, "LTYPE");
    w.pair(5, "14");
    w.pair(100, "AcDbSymbolTableRecord");
    w.pair(100, "AcDbLinetypeTableRecord");
    w.pair(2, "CONTINUOUS");
    w.pair(70, "0");
    w.pair(3, "Solid line");
    w.pair(72, "65");
    w.pair(73, "0");
    w.pair(40, "0");
    w.pair(0, "ENDTAB");
    w.pair(0, "TABLE");
    w.pair(2, "LAYER");
    w.pair(5, "2");
    w.pair(100, "AcDbSymbolTable");
    w.pair(70, &layers.len().max(1).to_string());
    let layer_names: Vec<String> = layers.iter().map(|l| safe_name(&l.name)).collect();
    for (i, l) in layers.iter().enumerate() {
        w.pair(0, "LAYER");
        let h = w.next_handle();
        w.pair(5, &h);
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbLayerTableRecord");
        w.pair(2, &layer_names[i]);
        w.pair(70, if l.locked { "4" } else { "0" });
        w.pair(62, if l.visible { "7" } else { "-7" });
        w.pair(6, "CONTINUOUS");
        w.pair(290, if l.printable { "1" } else { "0" });
    }
    if layers.is_empty() {
        w.pair(0, "LAYER");
        w.pair(5, "10");
        w.pair(100, "AcDbSymbolTableRecord");
        w.pair(100, "AcDbLayerTableRecord");
        w.pair(2, "0");
        w.pair(70, "0");
        w.pair(62, "7");
        w.pair(6, "CONTINUOUS");
    }
    w.pair(0, "ENDTAB");
    w.pair(0, "ENDSEC");
    // BLOCKS (empty) and ENTITIES.
    w.pair(0, "SECTION");
    w.pair(2, "BLOCKS");
    w.pair(0, "ENDSEC");
    w.pair(0, "SECTION");
    w.pair(2, "ENTITIES");
    for (i, l) in layers.iter().enumerate() {
        if !l.visible {
            continue;
        }
        w.layer = layer_names[i].clone();
        for s in &l.shapes {
            w.shape(s, Affine::IDENTITY);
        }
    }
    w.pair(0, "ENDSEC");
    w.pair(0, "SECTION");
    w.pair(2, "OBJECTS");
    w.pair(0, "DICTIONARY");
    w.pair(5, "C");
    w.pair(100, "AcDbDictionary");
    w.pair(0, "ENDSEC");
    w.pair(0, "EOF");
    w.out
}

/// Layer names: no spaces or special characters per the DXF rules.
fn safe_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() {
        "0".into()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;

    fn dxf(entities: &str) -> String {
        format!(
            "  0\nSECTION\n  2\nHEADER\n  9\n$INSUNITS\n 70\n4\n  0\nENDSEC\n  0\nSECTION\n  2\nTABLES\n  0\nTABLE\n  2\nLAYER\n  0\nLAYER\n  2\nWalls\n 62\n1\n  0\nENDTAB\n  0\nENDSEC\n  0\nSECTION\n  2\nENTITIES\n{entities}  0\nENDSEC\n  0\nEOF\n"
        )
    }

    #[test]
    fn lines_polylines_circles_and_layer_colours() {
        let text = dxf(
            "  0\nLINE\n  8\nWalls\n 10\n0\n 20\n0\n 11\n100\n 21\n0\n  0\nLWPOLYLINE\n  8\n0\n 62\n5\n 90\n3\n 70\n1\n 10\n0\n 20\n0\n 10\n10\n 20\n0\n 42\n1\n 10\n10\n 20\n10\n  0\nCIRCLE\n  8\n0\n 10\n50\n 20\n50\n 40\n5\n  0\nTEXT\n  8\n0\n 10\n5\n 20\n5\n 40\n3.5\n  1\nHello\n",
        );
        let mut ids = IdSource::default();
        let imp = parse(&text, &mut ids).expect("parse");
        assert_eq!(imp.shapes.len(), 4, "{:?}", imp.warnings);
        // The line takes the Walls layer colour (red).
        let line = &imp.shapes[0];
        assert_eq!(
            line.stroke.as_ref().map(|s| s.color),
            Some(Color::rgb8(255, 0, 0))
        );
        assert_eq!(imp.layers[0], "Walls");
        // Polyline: explicit blue, closed, with a bulge arc (curve segments).
        let poly = &imp.shapes[1];
        assert_eq!(
            poly.stroke.as_ref().map(|s| s.color),
            Some(Color::rgb8(0, 0, 255))
        );
        match &poly.kind {
            ShapeKind::Path { path, closed } => {
                assert!(*closed);
                assert!(path
                    .elements()
                    .iter()
                    .any(|e| matches!(e, PathEl::CurveTo(..))));
                // A bulge of 1 is a semicircle bulging to the right of the
                // segment from (10,0) to (10,10): it reaches x = 15.
                let b = path.bounding_box();
                assert!((b.x1 - 15.0).abs() < 0.05, "{b:?}");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(imp.shapes[2].kind, ShapeKind::Ellipse { .. }));
        match &imp.shapes[3].kind {
            ShapeKind::Text { spans, .. } => {
                assert_eq!(spans[0].text, "Hello");
                assert!((spans[0].size_pt - 3.5 / 25.4 * 72.0).abs() < 1e-9);
            }
            other => panic!("{other:?}"),
        }
        // Page from the content bounds plus a 5 mm margin.
        assert!(
            imp.size.width > 100.0 && imp.size.width < 115.0,
            "{:?}",
            imp.size
        );
    }

    #[test]
    fn inches_blocks_and_inserts() {
        let text = "  0\nSECTION\n  2\nHEADER\n  9\n$INSUNITS\n 70\n1\n  0\nENDSEC\n  0\nSECTION\n  2\nBLOCKS\n  0\nBLOCK\n  2\nBox\n 10\n0\n 20\n0\n  0\nLWPOLYLINE\n  8\n0\n 90\n4\n 70\n1\n 10\n0\n 20\n0\n 10\n1\n 20\n0\n 10\n1\n 20\n1\n 10\n0\n 20\n1\n  0\nENDBLK\n  0\nENDSEC\n  0\nSECTION\n  2\nENTITIES\n  0\nINSERT\n  8\n0\n  2\nBox\n 10\n2\n 20\n0\n 41\n2\n 42\n2\n  0\nENDSEC\n  0\nEOF\n";
        let mut ids = IdSource::default();
        let imp = parse(text, &mut ids).expect("parse");
        assert_eq!(imp.shapes.len(), 1, "{:?}", imp.warnings);
        let g = &imp.shapes[0];
        assert!(matches!(g.kind, ShapeKind::Group { .. }));
        let b = g.bounds();
        // 1 inch box scaled by 2 at x = 2 in: 50.8 .. 101.6 mm, plus the 5 mm
        // margin shift (origin at the content's lower-left).
        assert!((b.width() - 50.8).abs() < 1e-6, "{b:?}");
        assert!((b.x0 - 5.0).abs() < 1e-6, "{b:?}");
    }

    #[test]
    fn mtext_codes_are_stripped() {
        assert_eq!(
            strip_mtext_codes("\\fArial|b1;Hello\\PWorld"),
            "Hello\nWorld"
        );
        assert_eq!(strip_mtext_codes("{\\H2x;Big} 90%%d"), "Big 90\u{b0}");
        assert_eq!(strip_mtext_codes("\\S1^2;"), "1/2");
    }

    #[test]
    fn export_round_trips_a_filled_rectangle_and_text() {
        let mut doc = Document::new("t", Size::new(100.0, 50.0));
        let mut ids = doc.ids().clone();
        let mut r = Shape::new(
            ids.shape(),
            ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 40.0, 30.0),
                radius: 0.0,
                corners: None,
            },
        );
        r.fill = Fill::Solid(Color::rgb8(0, 0, 255));
        r.stroke = Some(Stroke::new(Color::rgb8(255, 0, 0), 0.5));
        let mut t = Shape::new(
            ids.shape(),
            ShapeKind::Text {
                spans: vec![TextSpan::new("Hi there", "Arial", 12.0)],
                origin: Point::new(50.0, 40.0),
                frame: None,
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            },
        );
        t.fill = Fill::Solid(Color::BLACK);
        doc.pages[0].layers[0].shapes = vec![r, t];
        doc.set_ids(ids);
        let out = page_to_dxf(&doc, 0);
        assert!(out.contains("LWPOLYLINE") && out.contains("HATCH") && out.contains("TEXT"));
        assert!(out.contains("$INSUNITS"));
        let mut ids2 = IdSource::default();
        let back = parse(&out, &mut ids2).expect("reparse");
        assert_eq!(back.shapes.len(), 3, "{:?}", back.warnings);
        // Hatch (fill), polyline (outline), text.
        let hatch = &back.shapes[0];
        assert_eq!(hatch.fill, Fill::Solid(Color::rgb8(0, 0, 255)));
        let hb = hatch.bounds();
        assert!(
            (hb.width() - 30.0).abs() < 1e-3 && (hb.height() - 20.0).abs() < 1e-3,
            "{hb:?}"
        );
        let outline = &back.shapes[1];
        let s = outline.stroke.as_ref().expect("stroke");
        assert_eq!(s.color, Color::rgb8(255, 0, 0));
        assert!((s.width - 0.5).abs() < 1e-6);
        match &back.shapes[2].kind {
            ShapeKind::Text { spans, .. } => assert_eq!(spans[0].text, "Hi there"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn malformed_input_is_an_error_or_empty_never_a_panic() {
        let mut ids = IdSource::default();
        assert!(parse("hello", &mut ids).is_err());
        assert!(parse("  0\nSECTION\n  2\nENTITIES\n  0\nLINE\n 10\nx\n", &mut ids).is_ok());
        let weird = dxf(
            "  0\nSPLINE\n 71\n3\n 10\n0\n 20\n0\n 10\n1\n 20\n1\n  0\nHATCH\n 70\n1\n 92\n0\n",
        );
        let imp = parse(&weird, &mut ids).expect("ok");
        assert!(imp.shapes.is_empty() || imp.shapes.len() <= 2);
        assert_eq!(aci_to_rgb(1), [255, 0, 0]);
        assert_eq!(rgb_to_aci([250, 5, 5]), 1);
        // A SECTION as the very last pair (found by mutation testing).
        assert!(parse(
            "  0\nSECTION\n  2\nENTITIES\n  0\nENDSEC\n  0\nSECTION\n",
            &mut ids
        )
        .is_ok());
        assert!(
            parse("  0\nSECTION", &mut ids).is_ok() || parse("  0\nSECTION", &mut ids).is_err()
        );
    }
}
