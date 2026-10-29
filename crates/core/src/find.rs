//! Finding objects by their properties (Edit > Find and Replace, Find
//! Objects by properties) and replacing a fill colour, outline colour,
//! outline width or font in the objects found.
//!
//! A query is a set of optional criteria; an object is found when it meets
//! every criterion that is set. Criteria about the look (fill, outline,
//! font) are met by the objects that carry that look: inside a group the
//! group's members are tested, and the group is found when one of its
//! members is. The replacement builds ordinary `SetFill`, `SetStroke` and
//! `SetShapeKind` commands, which the caller runs as one batch (one undo
//! step).

use crate::{Color, Command, Document, Fill, Layer, Shape, ShapeId, ShapeKind};

/// The kinds of object a query can look for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
    Rectangle,
    Ellipse,
    /// Polygons and stars.
    Polygon,
    Curve,
    ArtisticText,
    ParagraphText,
    Group,
    Bitmap,
    ClipFrame,
    Table,
    Symbol,
}

impl ObjectType {
    pub const ALL: [ObjectType; 11] = [
        ObjectType::Rectangle,
        ObjectType::Ellipse,
        ObjectType::Polygon,
        ObjectType::Curve,
        ObjectType::ArtisticText,
        ObjectType::ParagraphText,
        ObjectType::Group,
        ObjectType::Bitmap,
        ObjectType::ClipFrame,
        ObjectType::Table,
        ObjectType::Symbol,
    ];

    /// The type of `kind`.
    pub fn of(kind: &ShapeKind) -> ObjectType {
        match kind {
            ShapeKind::Rect { .. } => ObjectType::Rectangle,
            ShapeKind::Ellipse { .. } => ObjectType::Ellipse,
            ShapeKind::Polygon { .. } => ObjectType::Polygon,
            ShapeKind::Path { .. } => ObjectType::Curve,
            ShapeKind::Text { frame: None, .. } => ObjectType::ArtisticText,
            ShapeKind::Text { .. } => ObjectType::ParagraphText,
            ShapeKind::Group { .. } => ObjectType::Group,
            ShapeKind::Bitmap { .. } => ObjectType::Bitmap,
            ShapeKind::ClipFrame { .. } => ObjectType::ClipFrame,
            ShapeKind::Table(_) => ObjectType::Table,
            ShapeKind::SymbolInstance { .. } => ObjectType::Symbol,
        }
    }

    /// A stable name, used for the interface strings (`find.type.<key>`).
    pub fn key(self) -> &'static str {
        match self {
            ObjectType::Rectangle => "rectangle",
            ObjectType::Ellipse => "ellipse",
            ObjectType::Polygon => "polygon",
            ObjectType::Curve => "curve",
            ObjectType::ArtisticText => "artistic_text",
            ObjectType::ParagraphText => "paragraph_text",
            ObjectType::Group => "group",
            ObjectType::Bitmap => "bitmap",
            ObjectType::ClipFrame => "clip_frame",
            ObjectType::Table => "table",
            ObjectType::Symbol => "symbol",
        }
    }
}

/// What the fill or the outline of a found object must be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorQuery {
    /// No fill, or no outline.
    Nothing,
    /// This colour: a uniform fill of it, a gradient or mesh using it,
    /// or an outline of it.
    Color(Color),
}

/// The criteria; `None` leaves a property out of the search.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectQuery {
    pub object_type: Option<ObjectType>,
    pub fill: Option<ColorQuery>,
    pub outline: Option<ColorQuery>,
    /// Outline width in millimetres.
    pub outline_width: Option<f64>,
    /// Font family name, compared without regard to case.
    pub font: Option<String>,
}

/// How close two outline widths must be to count as the same (mm).
pub const WIDTH_TOLERANCE: f64 = 0.0005;

impl ObjectQuery {
    /// True when no criterion is set (such a query finds nothing).
    pub fn is_empty(&self) -> bool {
        self.object_type.is_none()
            && self.fill.is_none()
            && self.outline.is_none()
            && self.outline_width.is_none()
            && self.font.is_none()
    }

    fn has_look(&self) -> bool {
        self.fill.is_some()
            || self.outline.is_some()
            || self.outline_width.is_some()
            || self.font.is_some()
    }

    /// Does `s` itself (not its members) meet the look criteria?
    fn look_matches(&self, s: &Shape) -> bool {
        if let Some(q) = self.fill {
            let ok = match q {
                ColorQuery::Nothing => matches!(s.fill, Fill::None),
                ColorQuery::Color(c) => fill_uses(&s.fill, c),
            };
            if !ok {
                return false;
            }
        }
        if let Some(q) = self.outline {
            let ok = match (q, &s.stroke) {
                (ColorQuery::Nothing, None) => true,
                (ColorQuery::Color(c), Some(st)) => same_color(st.color, c),
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        if let Some(w) = self.outline_width {
            match &s.stroke {
                Some(st) if (st.width - w).abs() <= WIDTH_TOLERANCE => {}
                _ => return false,
            }
        }
        if let Some(f) = &self.font {
            match &s.kind {
                ShapeKind::Text { spans, .. } => {
                    if !spans
                        .iter()
                        .any(|sp| sp.font_family.eq_ignore_ascii_case(f))
                    {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }

    /// Does `s` meet the query, counting its members for the look?
    pub fn matches(&self, s: &Shape) -> bool {
        if self.is_empty() {
            return false;
        }
        if let Some(t) = self.object_type {
            if ObjectType::of(&s.kind) != t {
                return false;
            }
        }
        if !self.has_look() {
            return true;
        }
        if is_container(&s.kind) {
            return !self.matching_members(s).is_empty();
        }
        self.look_matches(s)
    }

    /// The objects inside `s` (or `s` itself when it is not a group or a
    /// frame) that carry the look the query asks for; the ones a
    /// replacement changes.
    pub fn matching_members<'a>(&self, s: &'a Shape) -> Vec<&'a Shape> {
        let mut out = Vec::new();
        fn walk<'a>(q: &ObjectQuery, s: &'a Shape, out: &mut Vec<&'a Shape>) {
            match &s.kind {
                ShapeKind::Group { children } => children.iter().for_each(|c| walk(q, c, out)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(q, frame, out);
                    contents.iter().for_each(|c| walk(q, c, out));
                }
                _ => {
                    if q.look_matches(s) {
                        out.push(s);
                    }
                }
            }
        }
        walk(self, s, &mut out);
        out
    }
}

fn is_container(k: &ShapeKind) -> bool {
    matches!(k, ShapeKind::Group { .. } | ShapeKind::ClipFrame { .. })
}

/// Are `a` and `b` the same colour? Colours in the same model match when
/// each value is within half a percent; colours in different models when
/// they show as the same 8-bit RGB.
pub fn same_color(a: Color, b: Color) -> bool {
    const E: f32 = 0.005;
    let near = |x: f32, y: f32| (x - y).abs() <= E;
    match (a, b) {
        (
            Color::Rgb { r, g, b },
            Color::Rgb {
                r: r2,
                g: g2,
                b: b2,
            },
        ) => near(r, r2) && near(g, g2) && near(b, b2),
        (
            Color::Cmyk { c, m, y, k },
            Color::Cmyk {
                c: c2,
                m: m2,
                y: y2,
                k: k2,
            },
        ) => near(c, c2) && near(m, m2) && near(y, y2) && near(k, k2),
        (Color::Gray { v }, Color::Gray { v: v2 }) => near(v, v2),
        (Color::Registration, Color::Registration) => true,
        (
            Color::Spot { ink, tint, .. },
            Color::Spot {
                ink: i2, tint: t2, ..
            },
        ) => ink == i2 && near(tint, t2),
        (Color::Registration, _) | (_, Color::Registration) => false,
        (Color::Spot { .. }, _) | (_, Color::Spot { .. }) => false,
        (a, b) => a.to_rgb8() == b.to_rgb8(),
    }
}

/// Does `fill` use `c` (a uniform fill of it, a gradient stop or a mesh
/// node of it)?
pub fn fill_uses(fill: &Fill, c: Color) -> bool {
    match fill {
        Fill::Solid(x) => same_color(*x, c),
        Fill::Fountain(f) => f.stops.iter().any(|s| same_color(s.color, c)),
        Fill::Mesh(m) => m.nodes.iter().any(|n| same_color(n.color, c)),
        _ => false,
    }
}

/// Where to look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    /// The layers of one page (by index).
    #[default]
    Page,
    /// Every page and the master layers.
    Document,
}

/// One object found: its page (None for a master layer) and its id. Only
/// top-level objects are listed (the ones that can be selected).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub page: Option<usize>,
    pub id: ShapeId,
}

/// The top-level objects that meet `q`, bottom to top, page by page.
/// `page` is the page searched with `Scope::Page`.
pub fn find(doc: &Document, q: &ObjectQuery, scope: Scope, page: usize) -> Vec<Hit> {
    let mut out = Vec::new();
    if q.is_empty() {
        return out;
    }
    let mut scan = |layers: &[Layer], page: Option<usize>| {
        for l in layers {
            for s in &l.shapes {
                if q.matches(s) {
                    out.push(Hit { page, id: s.id });
                }
            }
        }
    };
    match scope {
        Scope::Page => {
            if let Some(p) = doc.pages.get(page) {
                scan(&p.layers, Some(page));
            }
        }
        Scope::Document => {
            for (i, p) in doc.pages.iter().enumerate() {
                scan(&p.layers, Some(i));
            }
            scan(&doc.master, None);
        }
    }
    out
}

/// What to put in place of what was found; `None` keeps that property.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Replacement {
    pub fill: Option<Color>,
    pub outline: Option<Color>,
    /// Outline width in millimetres.
    pub outline_width: Option<f64>,
    pub font: Option<String>,
}

impl Replacement {
    pub fn is_empty(&self) -> bool {
        self.fill.is_none()
            && self.outline.is_none()
            && self.outline_width.is_none()
            && self.font.is_none()
    }
}

/// Swap `from` for `to` everywhere in `fill`; with no `from` the fill
/// becomes a uniform fill of `to`.
fn replace_in_fill(fill: &Fill, from: Option<Color>, to: Color) -> Fill {
    let Some(from) = from else {
        return Fill::Solid(to);
    };
    let mut f = fill.clone();
    match &mut f {
        Fill::Solid(x) if same_color(*x, from) => *x = to,
        Fill::Fountain(g) => g
            .stops
            .iter_mut()
            .filter(|s| same_color(s.color, from))
            .for_each(|s| s.color = to),
        Fill::Mesh(m) => m
            .nodes
            .iter_mut()
            .filter(|n| same_color(n.color, from))
            .for_each(|n| n.color = to),
        _ => {}
    }
    f
}

/// Is `s` locked, or on a locked layer?
fn locked_ids(doc: &Document) -> std::collections::HashSet<ShapeId> {
    doc.all_layers()
        .filter(|l| l.locked)
        .flat_map(|l| l.shapes.iter().map(|s| s.id))
        .collect()
}

/// The commands that apply `r` to the objects of `hits` found with `q`:
/// in each, the members that carry the queried look. A fill colour
/// replaces the queried fill colour (or, when the query has none, the
/// whole fill); an outline colour or width changes existing outlines
/// only; a font replaces the queried font (or every font of the text).
/// Locked objects and objects on locked layers are left alone.
pub fn replace_commands(
    doc: &Document,
    hits: &[Hit],
    q: &ObjectQuery,
    r: &Replacement,
) -> Vec<Command> {
    let mut cmds = Vec::new();
    if r.is_empty() {
        return cmds;
    }
    let locked = locked_ids(doc);
    let from_fill = match q.fill {
        Some(ColorQuery::Color(c)) => Some(c),
        _ => None,
    };
    let from_outline = match q.outline {
        Some(ColorQuery::Color(c)) => Some(c),
        _ => None,
    };
    for h in hits {
        if locked.contains(&h.id) {
            continue;
        }
        let Some(top) = doc.find_shape(h.id) else {
            continue;
        };
        if top.locked || !q.matches(top) {
            continue;
        }
        let members: Vec<&Shape> = if q.has_look() {
            q.matching_members(top)
        } else {
            // A type-only query: every member of the object.
            all_members(top)
        };
        for s in members {
            if s.locked {
                continue;
            }
            if let Some(to) = r.fill {
                let fill = replace_in_fill(&s.fill, from_fill, to);
                if fill != s.fill {
                    cmds.push(Command::SetFill {
                        shapes: vec![s.id],
                        fill,
                    });
                }
            }
            if let Some(st) = &s.stroke {
                let mut ns = st.clone();
                if let Some(to) = r.outline {
                    if from_outline.is_none_or(|f| same_color(ns.color, f)) {
                        ns.color = to;
                    }
                }
                if let Some(w) = r.outline_width {
                    if w.is_finite() && w >= 0.0 {
                        ns.width = w;
                    }
                }
                if &ns != st {
                    cmds.push(Command::SetStroke {
                        shapes: vec![s.id],
                        stroke: Some(ns),
                    });
                }
            }
            if let (Some(to), ShapeKind::Text { spans, .. }) = (&r.font, &s.kind) {
                if !to.is_empty() {
                    let mut kind = s.kind.clone();
                    if let ShapeKind::Text { spans: ns, .. } = &mut kind {
                        for sp in ns.iter_mut() {
                            if q.font
                                .as_ref()
                                .is_none_or(|f| sp.font_family.eq_ignore_ascii_case(f))
                            {
                                sp.font_family = to.clone();
                            }
                        }
                        if ns != spans {
                            cmds.push(Command::SetShapeKind { shape: s.id, kind });
                        }
                    }
                }
            }
        }
    }
    cmds
}

/// Every object inside `s` that is not a group or frame (or `s`).
fn all_members(s: &Shape) -> Vec<&Shape> {
    let mut out = Vec::new();
    fn walk<'a>(s: &'a Shape, out: &mut Vec<&'a Shape>) {
        match &s.kind {
            ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, out)),
            ShapeKind::ClipFrame { frame, contents } => {
                walk(frame, out);
                contents.iter().for_each(|c| walk(c, out));
            }
            _ => out.push(s),
        }
    }
    walk(s, &mut out);
    out
}

/// The query that finds objects looking like `s`: its fill colour (or no
/// fill), outline colour and width (or no outline), the first font of a
/// text, and its type. Gradient and other fills leave the fill out. A
/// group or clip frame gives its type and the look of its first member.
pub fn query_from(top: &Shape) -> ObjectQuery {
    let s = all_members(top).first().copied().unwrap_or(top);
    let fill = match &s.fill {
        Fill::None => Some(ColorQuery::Nothing),
        Fill::Solid(c) => Some(ColorQuery::Color(*c)),
        _ => None,
    };
    let (outline, outline_width) = match &s.stroke {
        None => (Some(ColorQuery::Nothing), None),
        Some(st) => (Some(ColorQuery::Color(st.color)), Some(st.width)),
    };
    let font = match &s.kind {
        ShapeKind::Text { spans, .. } => spans.first().map(|sp| sp.font_family.clone()),
        _ => None,
    };
    ObjectQuery {
        object_type: Some(ObjectType::of(&top.kind)),
        fill,
        outline,
        outline_width,
        font,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, Rect, Stroke, TextSpan};

    fn add(e: &mut Engine, page: usize, s: Shape) -> ShapeId {
        let id = s.id;
        let layer = e.document().pages[page].layers[0].id;
        e.run(&Command::AddShape { layer, shape: s }).unwrap();
        id
    }

    fn rect(e: &mut Engine, fill: Fill, stroke: Option<Stroke>) -> ShapeId {
        let id = e.new_shape_id();
        let mut s = Shape::new(
            id,
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = fill;
        s.stroke = stroke;
        add(e, 0, s)
    }

    fn red() -> Color {
        Color::cmyk_pct(0.0, 100.0, 100.0, 0.0)
    }
    fn blue() -> Color {
        Color::cmyk_pct(100.0, 100.0, 0.0, 0.0)
    }
    fn outline(c: Color, w: f64) -> Stroke {
        Stroke {
            color: c,
            width: w,
            ..Stroke::default()
        }
    }

    #[test]
    fn same_color_compares_within_a_model_and_across() {
        assert!(same_color(red(), Color::cmyk_pct(0.0, 99.7, 100.0, 0.0)));
        assert!(!same_color(red(), Color::cmyk_pct(0.0, 98.0, 100.0, 0.0)));
        let rgb = Color::rgb8(10, 200, 30);
        assert!(same_color(rgb, Color::rgb8(10, 200, 30)));
        let [r, g, b] = red().to_rgb8();
        assert!(same_color(red(), Color::rgb8(r, g, b)));
        assert!(!same_color(Color::Registration, Color::BLACK));
    }

    #[test]
    fn finds_by_fill_outline_width_and_type() {
        let mut e = Engine::new(Document::default());
        let a = rect(&mut e, Fill::Solid(red()), Some(outline(blue(), 0.5)));
        let b = rect(&mut e, Fill::Solid(blue()), Some(outline(red(), 1.0)));
        let c = rect(&mut e, Fill::None, None);
        let doc = e.document().clone();
        let doc = &doc;
        let ids = |q: ObjectQuery| -> Vec<ShapeId> {
            find(doc, &q, Scope::Page, 0).iter().map(|h| h.id).collect()
        };
        let q = ObjectQuery {
            fill: Some(ColorQuery::Color(red())),
            ..Default::default()
        };
        assert_eq!(ids(q), vec![a]);
        let q = ObjectQuery {
            outline: Some(ColorQuery::Color(red())),
            ..Default::default()
        };
        assert_eq!(ids(q), vec![b]);
        let q = ObjectQuery {
            outline_width: Some(0.5),
            ..Default::default()
        };
        assert_eq!(ids(q), vec![a]);
        let q = ObjectQuery {
            fill: Some(ColorQuery::Nothing),
            outline: Some(ColorQuery::Nothing),
            ..Default::default()
        };
        assert_eq!(ids(q), vec![c]);
        let q = ObjectQuery {
            object_type: Some(ObjectType::Rectangle),
            ..Default::default()
        };
        assert_eq!(ids(q), vec![a, b, c]);
        let q = ObjectQuery {
            object_type: Some(ObjectType::Ellipse),
            ..Default::default()
        };
        assert!(ids(q).is_empty());
        assert!(ids(ObjectQuery::default()).is_empty(), "no criteria");
    }

    #[test]
    fn finds_across_pages_and_inside_groups() {
        let mut e = Engine::new(Document::default());
        let a = rect(&mut e, Fill::Solid(red()), None);
        let b = rect(&mut e, Fill::Solid(blue()), None);
        e.run(&Command::Group { shapes: vec![a, b] }).unwrap();
        let group = e.document().pages[0].layers[0].shapes[0].id;
        e.run(&Command::AddPage {
            name: None,
            size: crate::Size::new(100.0, 100.0),
            layer: None,
        })
        .unwrap();
        let id = e.new_shape_id();
        let mut s = Shape::new(
            id,
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 5.0, 5.0),
                arc: None,
            },
        );
        s.fill = Fill::Solid(red());
        let other = add(&mut e, 1, s);
        let q = ObjectQuery {
            fill: Some(ColorQuery::Color(red())),
            ..Default::default()
        };
        let page = find(e.document(), &q, Scope::Page, 0);
        assert_eq!(
            page,
            vec![Hit {
                page: Some(0),
                id: group
            }]
        );
        let all = find(e.document(), &q, Scope::Document, 0);
        assert_eq!(all.len(), 2);
        assert_eq!(
            all[1],
            Hit {
                page: Some(1),
                id: other
            }
        );
        // Replacing touches the red member only, across both pages, in
        // one undo step.
        let r = Replacement {
            fill: Some(Color::rgb8(0, 255, 0)),
            ..Default::default()
        };
        let cmds = replace_commands(e.document(), &all, &q, &r);
        assert_eq!(cmds.len(), 2);
        e.run_batch("Replace", &cmds).unwrap();
        let green = Fill::Solid(Color::rgb8(0, 255, 0));
        assert_eq!(e.document().find_shape(a).unwrap().fill, green);
        assert_eq!(
            e.document().find_shape(b).unwrap().fill,
            Fill::Solid(blue())
        );
        assert_eq!(e.document().find_shape(other).unwrap().fill, green);
        e.undo().unwrap();
        assert_eq!(e.document().find_shape(a).unwrap().fill, Fill::Solid(red()));
        assert_eq!(
            e.document().find_shape(other).unwrap().fill,
            Fill::Solid(red())
        );
    }

    #[test]
    fn replaces_gradient_stops_outlines_and_fonts() {
        let mut e = Engine::new(Document::default());
        let mut g = crate::Fountain::two(crate::FountainKind::Linear, red(), blue(), 0.0);
        g.stops[1].color = red();
        let a = rect(&mut e, Fill::Fountain(g), Some(outline(red(), 0.2)));
        let b = rect(&mut e, Fill::Solid(blue()), Some(outline(blue(), 0.2)));
        let q = ObjectQuery {
            fill: Some(ColorQuery::Color(red())),
            ..Default::default()
        };
        let hits = find(e.document(), &q, Scope::Page, 0);
        assert_eq!(hits.len(), 1);
        let r = Replacement {
            fill: Some(blue()),
            outline_width: Some(1.5),
            ..Default::default()
        };
        e.run_batch("Replace", &replace_commands(e.document(), &hits, &q, &r))
            .unwrap();
        let sa = e.document().find_shape(a).unwrap();
        let Fill::Fountain(g) = &sa.fill else {
            panic!("still a gradient")
        };
        assert!(g.stops.iter().all(|s| same_color(s.color, blue())));
        assert_eq!(sa.stroke.as_ref().unwrap().width, 1.5);
        assert_eq!(sa.stroke.as_ref().unwrap().color, red(), "colour kept");
        let sb = e.document().find_shape(b).unwrap();
        assert_eq!(sb.stroke.as_ref().unwrap().width, 0.2, "not found");

        // Outline colour by outline colour.
        let q = ObjectQuery {
            outline: Some(ColorQuery::Color(blue())),
            ..Default::default()
        };
        let hits = find(e.document(), &q, Scope::Page, 0);
        let r = Replacement {
            outline: Some(red()),
            ..Default::default()
        };
        e.run_batch("Replace", &replace_commands(e.document(), &hits, &q, &r))
            .unwrap();
        let sb = e.document().find_shape(b).unwrap();
        assert_eq!(sb.stroke.as_ref().unwrap().color, red());

        // Fonts.
        let span = |f: &str| TextSpan::new("Hi", f, 12.0);
        let id = e.new_shape_id();
        let t = Shape::new(
            id,
            ShapeKind::Text {
                spans: vec![span("Arial"), span("Serif")],
                origin: crate::Point::new(0.0, 0.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        let tid = add(&mut e, 0, t);
        let q = ObjectQuery {
            font: Some("arial".into()),
            ..Default::default()
        };
        let hits = find(e.document(), &q, Scope::Page, 0);
        assert_eq!(hits.len(), 1);
        let r = Replacement {
            font: Some("Mono".into()),
            ..Default::default()
        };
        e.run_batch("Replace", &replace_commands(e.document(), &hits, &q, &r))
            .unwrap();
        let ShapeKind::Text { spans, .. } = &e.document().find_shape(tid).unwrap().kind else {
            panic!("text")
        };
        assert_eq!(spans[0].font_family, "Mono");
        assert_eq!(spans[1].font_family, "Serif", "other font kept");
    }

    #[test]
    fn locked_objects_are_found_but_not_changed() {
        let mut e = Engine::new(Document::default());
        let a = rect(&mut e, Fill::Solid(red()), None);
        e.run(&Command::SetLocked {
            shapes: vec![a],
            locked: true,
        })
        .unwrap();
        let q = ObjectQuery {
            fill: Some(ColorQuery::Color(red())),
            ..Default::default()
        };
        let hits = find(e.document(), &q, Scope::Page, 0);
        assert_eq!(hits.len(), 1);
        let r = Replacement {
            fill: Some(blue()),
            ..Default::default()
        };
        assert!(replace_commands(e.document(), &hits, &q, &r).is_empty());
    }

    #[test]
    fn query_from_an_object() {
        let mut s = Shape::new(
            ShapeId(7),
            ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 5.0, 5.0),
                arc: None,
            },
        );
        s.fill = Fill::Solid(red());
        s.stroke = Some(outline(blue(), 0.7));
        let q = query_from(&s);
        assert_eq!(q.object_type, Some(ObjectType::Ellipse));
        assert_eq!(q.fill, Some(ColorQuery::Color(red())));
        assert_eq!(q.outline, Some(ColorQuery::Color(blue())));
        assert_eq!(q.outline_width, Some(0.7));
        assert!(q.matches(&s));
        assert!(ObjectType::ALL.contains(&ObjectType::of(&s.kind)));
        // A group: its type, the look of its first member.
        let g = Shape::new(
            ShapeId(8),
            ShapeKind::Group {
                children: vec![s.clone()],
            },
        );
        let q = query_from(&g);
        assert_eq!(q.object_type, Some(ObjectType::Group));
        assert_eq!(q.fill, Some(ColorQuery::Color(red())));
    }
}
