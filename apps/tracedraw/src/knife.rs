//! The Knife tool. A cut, a straight line between two points (extended
//! across the objects) or a freehand stroke, splits the objects it
//! crosses: the selection, or every object on the page's unlocked layers.
//! Closed objects lose a band along the cut (as wide as the bar's gap, a
//! hair when the parts should meet) and each separate piece becomes an
//! object of its own (or they stay one object, combined); open curves are
//! split where the cut crosses them. Pieces keep the fill, outline and
//! opacity. A bitmap (or a cropped one: a clip frame holding one bitmap)
//! is cut the way a picture is cropped: each piece is the whole picture
//! behind an outline of its own along the cut, its pixels untouched, so
//! the Shape tool still edits each outline. Groups and text are not cut.
//! One undo step.

use crate::app::App;
use serde::{Deserialize, Serialize};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{BezPath, Line, ParamCurve, PathSeg, Point, Rect},
    shaping, Command, ShapeId,
};

/// How the cut is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnifeMode {
    #[default]
    TwoPoint,
    Freehand,
}

/// The Knife's bar.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KnifeSettings {
    pub mode: KnifeMode,
    /// Width taken out along the cut, mm (0: the parts meet).
    pub gap: f64,
    /// The pieces of each object stay one object.
    pub one_object: bool,
}

impl Default for KnifeSettings {
    fn default() -> Self {
        KnifeSettings {
            mode: KnifeMode::TwoPoint,
            gap: 0.0,
            one_object: false,
        }
    }
}

/// The band taken out when the parts should meet, mm.
const HAIR: f64 = 0.01;

/// The cut as a polyline: two points stretched across `reach` (a line), or
/// the stroke as drawn.
fn cut_line(points: &[Point], reach: Rect) -> Vec<Point> {
    match points {
        [a, b] => {
            let d = *b - *a;
            let len = d.hypot();
            if len < 1e-9 {
                return points.to_vec();
            }
            let far = (reach.width() + reach.height()) * 2.0 + (reach.center() - *a).hypot();
            let u = d / len;
            vec![*a - u * far, *b + u * far]
        }
        _ => points.to_vec(),
    }
}

/// Where the open path `path` crosses the polyline `cut`: (segment, t).
pub(crate) fn crossings(path: &BezPath, cut: &[Point]) -> Vec<(usize, f64)> {
    let mut out = Vec::new();
    for (i, seg) in path.segments().enumerate() {
        for w in cut.windows(2) {
            let line = Line::new(w[0], w[1]);
            for hit in seg.intersect_line(line) {
                if (0.0..=1.0).contains(&hit.line_t) {
                    out.push((i, hit.segment_t));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    out.dedup_by(|a, b| a.0 == b.0 && (a.1 - b.1).abs() < 1e-9);
    out
}

/// `path` (one open subpath) split at `cuts` (segment, t), in order.
pub(crate) fn split_open(path: &BezPath, cuts: &[(usize, f64)]) -> Vec<BezPath> {
    let segs: Vec<PathSeg> = path.segments().collect();
    let mut pieces = Vec::new();
    let mut cur = BezPath::new();
    let push_seg = |cur: &mut BezPath, s: PathSeg| {
        if cur.elements().is_empty() {
            cur.move_to(s.start());
        }
        match s {
            PathSeg::Line(l) => cur.line_to(l.p1),
            PathSeg::Quad(q) => cur.quad_to(q.p1, q.p2),
            PathSeg::Cubic(c) => cur.curve_to(c.p1, c.p2, c.p3),
        }
    };
    for (i, seg) in segs.iter().enumerate() {
        let mut from = 0.0;
        for &(_, t) in cuts.iter().filter(|(k, _)| *k == i) {
            if t > from + 1e-9 {
                push_seg(&mut cur, seg.subsegment(from..t));
            }
            if !cur.elements().is_empty() {
                pieces.push(std::mem::take(&mut cur));
            }
            from = t;
        }
        if from < 1.0 - 1e-9 {
            push_seg(&mut cur, seg.subsegment(from..1.0));
        }
    }
    if !cur.elements().is_empty() {
        pieces.push(cur);
    }
    pieces.retain(|p| p.elements().len() > 1);
    pieces
}

/// The subpaths of a path, each with whether it is closed.
fn subpaths(path: &BezPath) -> Vec<(BezPath, bool)> {
    tracedraw_core::effects::subpaths(path)
}

/// The pieces `s` (in page space) falls into along `cut` (page space),
/// each a page-space path with whether it is closed; None when the cut
/// does not split it.
fn pieces_of(s: &Shape, cut: &[Point], gap: f64) -> Option<Vec<(BezPath, bool)>> {
    let page = s.page_path();
    let closed_kind = match &s.kind {
        ShapeKind::Path { closed, .. } => *closed,
        ShapeKind::Rect { .. } | ShapeKind::Polygon { .. } => true,
        ShapeKind::Ellipse { arc, .. } => arc.as_ref().is_none_or(|a| a.pie),
        _ => return None,
    };
    let mut out = Vec::new();
    let mut split = false;
    if closed_kind {
        let band = shaping::stroke_band(cut, gap.max(HAIR), false);
        let parts = shaping::overlay_parts(&page, &band, shaping::Op::Trim);
        // Split means more than one piece, or a piece less (cut away).
        if parts.len() < 2 {
            return None;
        }
        out.extend(parts.into_iter().map(|p| (p, true)));
        split = true;
    } else {
        for (sub, closed) in subpaths(&page) {
            if closed {
                out.push((sub, true));
                continue;
            }
            let cuts = crossings(&sub, cut);
            if cuts.is_empty() {
                out.push((sub, false));
                continue;
            }
            let parts = split_open(&sub, &cuts);
            split |= parts.len() > 1;
            out.extend(parts.into_iter().map(|p| (p, false)));
        }
    }
    split.then_some(out)
}

/// The picture `s` shows and the outline it shows it through, on the
/// page: a bitmap's rectangle, or the frame of a clip frame that holds one
/// bitmap (a cropped picture). `None` for anything else.
fn picture_outline(s: &Shape) -> Option<(Shape, BezPath)> {
    match &s.kind {
        ShapeKind::Bitmap { .. } => {
            let mut inner = Shape::new(s.id, s.kind.clone());
            inner.fill = s.fill.clone();
            inner.stroke = s.stroke.clone();
            Some((inner, s.page_path()))
        }
        ShapeKind::ClipFrame { frame, contents }
            if contents.len() == 1 && matches!(contents[0].kind, ShapeKind::Bitmap { .. }) =>
        {
            Some((contents[0].clone(), s.transform * frame.page_path()))
        }
        _ => None,
    }
}

/// The cut `points` (page space) run on past both ends, far beyond
/// `reach`, along their first and last stretches.
fn run_on(points: &[Point], reach: Rect) -> Vec<Point> {
    let far = (reach.width() + reach.height()) * 4.0 + 10.0;
    let mut out = points.to_vec();
    if let ([a, b, ..], [.., y, z]) = (points, points) {
        let (d0, d1) = (*a - *b, *z - *y);
        if d0.hypot() > 1e-12 && d1.hypot() > 1e-12 {
            out.insert(0, *a + d0 / d0.hypot() * far);
            out.push(*z + d1 / d1.hypot() * far);
        }
    }
    out
}

/// The region on one side of the cut `line` (already run on far past the
/// object): the line closed round by a loop far off to its left, so the
/// region and the rest of the plane share the cut exactly.
fn one_side(line: &[Point]) -> Option<BezPath> {
    let (s, e) = (*line.first()?, *line.last()?);
    let d = e - s;
    let len = d.hypot();
    if len < 1e-9 {
        return None;
    }
    let n = tracedraw_core::geometry::Vec2::new(-d.y, d.x) / len * (len * 4.0);
    let mut path = BezPath::new();
    path.move_to(s);
    for p in &line[1..] {
        path.line_to(*p);
    }
    path.line_to(e + n);
    path.line_to(s + n);
    path.close_path();
    Some(path)
}

/// The outlines (page space) the picture outline `outline` falls into
/// along `cut`: with Split (`gap` 0) the two sides of the cut, whose
/// outlines meet exactly along it; with a gap, the outline less the band
/// as wide as the gap. `None` when the cut does not split it.
fn picture_pieces(
    outline: &BezPath,
    points: &[Point],
    cut: &[Point],
    gap: f64,
) -> Option<Vec<BezPath>> {
    // Split as closed objects are: more than one piece, or none.
    let band = shaping::stroke_band(cut, gap.max(HAIR), false);
    let parts = shaping::overlay_parts(outline, &band, shaping::Op::Trim);
    if parts.len() < 2 {
        return None;
    }
    if gap > 0.0 {
        return Some(parts);
    }
    let reach = outline
        .elements()
        .iter()
        .filter_map(|e| e.end_point())
        .fold(None::<Rect>, |r, p| {
            Some(r.map_or(Rect::from_points(p, p), |r| r.union_pt(p)))
        })?;
    let line = run_on(if points.len() == 2 { cut } else { points }, reach);
    let side = one_side(&line)?;
    let mut exact = shaping::overlay_parts(outline, &side, shaping::Op::Intersect);
    exact.extend(shaping::overlay_parts(outline, &side, shaping::Op::Trim));
    Some(if exact.len() >= 2 {
        exact.iter().map(|p| onto_corners(p, outline)).collect()
    } else {
        parts
    })
}

/// `piece` with each node that lies within a ten-thousandth of a mm of a
/// node of `outline` put on it: the overlay works on a fine grid, and the
/// pieces' corners should be the picture's own.
fn onto_corners(piece: &BezPath, outline: &BezPath) -> BezPath {
    use tracedraw_core::geometry::PathEl;
    let corners: Vec<Point> = outline
        .elements()
        .iter()
        .filter_map(|e| e.end_point())
        .collect();
    let snap = |p: Point| {
        corners
            .iter()
            .copied()
            .find(|c| (*c - p).hypot() < 1e-4)
            .unwrap_or(p)
    };
    let mut out = BezPath::new();
    for el in piece.elements() {
        out.push(match *el {
            PathEl::MoveTo(p) => PathEl::MoveTo(snap(p)),
            PathEl::LineTo(p) => PathEl::LineTo(snap(p)),
            PathEl::QuadTo(a, p) => PathEl::QuadTo(a, snap(p)),
            PathEl::CurveTo(a, b, p) => PathEl::CurveTo(a, b, snap(p)),
            PathEl::ClosePath => PathEl::ClosePath,
        });
    }
    out
}

/// A cropped picture: `picture` (a bitmap in its own space, untouched)
/// shown through the closed outline `outline` in the object's space, as a
/// cropped bitmap is kept (a clip frame whose frame has no fill or
/// outline), with new ids for the frame and the picture.
fn cropped(app: &mut App, picture: &Shape, outline: BezPath) -> ShapeKind {
    let mut frame = Shape::new(
        app.engine.new_shape_id(),
        ShapeKind::Path {
            path: outline,
            closed: true,
        },
    );
    frame.fill = tracedraw_core::Fill::None;
    frame.stroke = None;
    let mut inner = picture.clone();
    inner.id = app.engine.new_shape_id();
    ShapeKind::ClipFrame {
        frame: Box::new(frame),
        contents: vec![inner],
    }
}

impl App {
    /// Cut along `points` (page space): two points for a straight cut,impl App {
    /// Cut along `points` (page space): two points for a straight cut,
    /// more for a freehand one.
    pub fn knife_cut(&mut self, points: &[Point]) {
        if points.len() < 2 {
            return;
        }
        let targets: Vec<Shape> = if self.selection.is_empty() {
            self.doc()
                .page(self.page)
                .map(|p| {
                    p.layers
                        .iter()
                        .filter(|l| !l.locked && l.visible)
                        .flat_map(|l| l.shapes.clone())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            self.selected_shapes()
        };
        let Some(layer) = self.active_layer() else {
            return;
        };
        let gap = self.knife.gap.clamp(0.0, 1000.0);
        let mut cmds = Vec::new();
        let mut new_ids: Vec<ShapeId> = Vec::new();
        for s in targets {
            if s.locked {
                continue;
            }
            let b = s.bounds();
            let cut = cut_line(points, b.inflate(1.0, 1.0));
            let reach = cut
                .iter()
                .fold(Rect::from_points(cut[0], cut[0]), |r, p| r.union_pt(*p))
                .inflate(gap + 1.0, gap + 1.0);
            if reach.intersect(b).is_zero_area() && !b.is_zero_area() {
                continue;
            }
            if let Some((picture, outline)) = picture_outline(&s) {
                // A picture is cut as it is cropped: each piece the whole
                // picture behind an outline of its own along the cut.
                let Some(pieces) = picture_pieces(&outline, points, &cut, gap) else {
                    continue;
                };
                let inv = s.transform.inverse();
                let mut outlines: Vec<BezPath> = pieces.iter().map(|p| inv * p.clone()).collect();
                if self.knife.one_object {
                    let mut all = BezPath::new();
                    for p in &outlines {
                        all.extend(p.elements().iter().copied());
                    }
                    outlines = vec![all];
                }
                let mut it = outlines.into_iter();
                if let Some(o) = it.next() {
                    let kind = cropped(self, &picture, o);
                    cmds.push(Command::SetShapeKind { shape: s.id, kind });
                    new_ids.push(s.id);
                }
                for o in it {
                    let mut piece = s.clone();
                    piece.id = self.engine.new_shape_id();
                    piece.kind = cropped(self, &picture, o);
                    new_ids.push(piece.id);
                    cmds.push(Command::AddShape {
                        layer,
                        shape: piece,
                    });
                }
                continue;
            }
            let Some(pieces) = pieces_of(&s, &cut, gap) else {
                continue;
            };
            let inv = s.transform.inverse();
            let local = |p: &BezPath| inv * p.clone();
            let pieces: Vec<(BezPath, bool)> = if self.knife.one_object {
                let mut all = BezPath::new();
                let mut closed = true;
                for (p, c) in &pieces {
                    all.extend(local(p).elements().iter().copied());
                    closed &= *c;
                }
                vec![(all, closed)]
            } else {
                pieces.iter().map(|(p, c)| (local(p), *c)).collect()
            };
            let mut it = pieces.into_iter();
            // The first piece keeps the object (its id, its place).
            if let Some((p, closed)) = it.next() {
                cmds.push(Command::SetShapeKind {
                    shape: s.id,
                    kind: ShapeKind::Path { path: p, closed },
                });
                new_ids.push(s.id);
            }
            for (p, closed) in it {
                let id = self.engine.new_shape_id();
                let mut piece = Shape::new(id, ShapeKind::Path { path: p, closed });
                piece.transform = s.transform;
                piece.fill = s.fill.clone();
                piece.stroke = s.stroke.clone();
                piece.opacity = s.opacity;
                cmds.push(Command::AddShape {
                    layer,
                    shape: piece,
                });
                new_ids.push(id);
            }
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Knife", &cmds) {
            self.status = e.to_string();
            return;
        }
        self.select(new_ids);
    }

    /// The two-point knife's drag ended at `b`.
    pub fn knife(&mut self, a: Point, b: Point) {
        self.knife_cut(&[a, b]);
    }
}

/// Points along a straight drag, for tests.
#[cfg(test)]
fn line(a: Point, b: Point, n: usize) -> Vec<Point> {
    (0..=n).map(|i| a.lerp(b, i as f64 / n as f64)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Affine, Shape as _};

    fn square(app: &mut App) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 40.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("shape")
    }

    fn count(app: &App) -> usize {
        app.doc().pages[0].layers[0].shapes.len()
    }

    /// A straight cut through the middle: two halves that meet, one undo
    /// step; with a gap they are that far apart; kept as one object they
    /// are one.
    #[test]
    fn straight_cuts_split_closed_objects() {
        let mut app = App::headless();
        square(&mut app);
        app.select(Vec::new());
        app.knife(Point::new(20.0, 10.0), Point::new(20.0, 30.0));
        assert_eq!(count(&app), 2);
        let bounds: Vec<Rect> = app.doc().pages[0].layers[0]
            .shapes
            .iter()
            .map(|s| s.bounds())
            .collect();
        assert!(
            bounds.iter().all(|b| (b.width() - 20.0).abs() < 0.05),
            "{bounds:?}"
        );
        app.undo();
        assert_eq!(count(&app), 1);
        app.knife.gap = 4.0;
        app.knife(Point::new(20.0, 10.0), Point::new(20.0, 30.0));
        let mut xs: Vec<f64> = app.doc().pages[0].layers[0]
            .shapes
            .iter()
            .map(|s| s.bounds().x1)
            .collect();
        xs.sort_by(f64::total_cmp);
        assert!((xs[0] - 18.0).abs() < 0.1, "{xs:?}");
        app.undo();
        app.knife.gap = 0.0;
        app.knife.one_object = true;
        app.knife(Point::new(20.0, 10.0), Point::new(20.0, 30.0));
        assert_eq!(count(&app), 1);
        // A cut beside the object leaves it alone.
        app.undo();
        app.knife(Point::new(60.0, 0.0), Point::new(60.0, 10.0));
        assert_eq!(count(&app), 1);
    }

    /// A freehand cut across a square: the pieces on either side of the
    /// stroke; an open line is split where the cut crosses it.
    #[test]
    fn freehand_cuts_and_open_curves() {
        let mut app = App::headless();
        square(&mut app);
        app.select(Vec::new());
        // A bent stroke from the left side out the bottom.
        let mut stroke = line(Point::new(-5.0, 30.0), Point::new(20.0, 20.0), 10);
        stroke.extend(line(Point::new(20.0, 20.0), Point::new(25.0, -5.0), 10));
        app.knife_cut(&stroke);
        assert_eq!(count(&app), 2);
        app.undo();
        // An open line across a vertical cut: two lines.
        let mut path = BezPath::new();
        path.move_to(Point::new(0.0, 60.0));
        path.line_to(Point::new(40.0, 60.0));
        let l = app
            .new_shape(ShapeKind::Path {
                path,
                closed: false,
            })
            .expect("line");
        app.select(vec![l]);
        app.knife(Point::new(10.0, 50.0), Point::new(10.0, 70.0));
        let lines: Vec<Rect> = app
            .selection
            .iter()
            .filter_map(|id| app.doc().shape_on_page(*id))
            .map(|s| s.bounds())
            .collect();
        assert_eq!(lines.len(), 2);
        let mut widths: Vec<f64> = lines.iter().map(|b| b.width()).collect();
        widths.sort_by(f64::total_cmp);
        assert!(
            (widths[0] - 10.0).abs() < 1e-6 && (widths[1] - 30.0).abs() < 1e-6,
            "{widths:?}"
        );
        // Open pieces stay open.
        for id in &app.selection {
            let s = app.doc().find_shape(*id).expect("piece");
            assert!(matches!(s.kind, ShapeKind::Path { closed: false, .. }));
        }
    }

    /// A 40 x 20 pixel picture on a 40 x 20 mm rectangle, each pixel its
    /// own colour.
    fn picture(app: &mut App) -> ShapeId {
        let img = image::RgbaImage::from_fn(40, 20, |x, y| {
            image::Rgba([x as u8 * 6, y as u8 * 12, 100, 255])
        });
        app.new_shape(ShapeKind::Bitmap {
            rect: Rect::new(0.0, 0.0, 40.0, 20.0),
            width_px: 40,
            height_px: 20,
            png: crate::bitmap_fx::encode(&img).expect("png").into(),
            fx: None,
        })
        .expect("bitmap")
    }

    /// Each cropped picture on the page: its outline on the page and its
    /// picture (kind and transform on the page).
    fn pieces(app: &App) -> Vec<(BezPath, ShapeKind, Affine)> {
        app.doc().pages[0].layers[0]
            .shapes
            .iter()
            .map(|s| match &s.kind {
                ShapeKind::ClipFrame { frame, contents } => {
                    assert_eq!(contents.len(), 1);
                    assert!(matches!(frame.kind, ShapeKind::Path { closed: true, .. }));
                    assert!(frame.stroke.is_none());
                    (
                        s.transform * frame.page_path(),
                        contents[0].kind.clone(),
                        s.transform * contents[0].transform,
                    )
                }
                k => panic!("{k:?}"),
            })
            .collect()
    }

    fn area(p: &BezPath) -> f64 {
        p.area().abs()
    }

    /// The Knife cuts a bitmap as the picture is cropped: each piece is
    /// the whole picture, its pixels untouched, behind an outline of its
    /// own; with Split the outlines meet exactly on the cut, with a gap
    /// they are that far apart; one undo step (#1333).
    #[test]
    fn a_cut_crops_a_bitmap_into_pieces() {
        let mut app = App::headless();
        let id = picture(&mut app);
        let original = app.doc().find_shape(id).expect("picture").kind.clone();
        app.select(Vec::new());
        app.knife(Point::new(10.3, -5.0), Point::new(10.3, 25.0));
        let got = pieces(&app);
        assert_eq!(got.len(), 2);
        let mut xs: Vec<(f64, f64)> = got
            .iter()
            .map(|(o, kind, t)| {
                assert_eq!(*kind, original, "the pixels are untouched");
                assert_eq!(*t, Affine::IDENTITY);
                let b = o.bounding_box();
                assert!(b.y0.abs() < 1e-5 && (b.y1 - 20.0).abs() < 1e-5, "{b:?}");
                (b.x0, b.x1)
            })
            .collect();
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        // To the overlay's grid (about a millionth of a mm).
        assert!(
            xs[0].0.abs() < 1e-5 && (xs[1].1 - 40.0).abs() < 1e-5,
            "{xs:?}"
        );
        assert_eq!(xs[0].1, xs[1].0, "the outlines meet exactly");
        assert!((xs[0].1 - 10.3).abs() < 1e-5, "{xs:?}");
        let total: f64 = got.iter().map(|(o, _, _)| area(o)).sum();
        assert!((total - 800.0).abs() < 1e-3, "{total}");
        assert_eq!(app.selection.len(), 2);
        // The Shape tool edits each piece's outline.
        for piece in app.selection.clone() {
            let target = app.node_target(piece);
            assert_ne!(target, piece);
            assert!(matches!(
                app.doc().find_shape(target).map(|s| &s.kind),
                Some(ShapeKind::Path { .. })
            ));
        }
        // Drawn, the pieces show the picture with no seam along the cut
        // (at 10 pixels per mm the cut runs through column 103).
        let page = app.doc().pages[0].id;
        let after = tracedraw_render::render_page_image(app.doc(), page, 254.0).expect("after");
        app.undo();
        assert_eq!(count(&app), 1);
        assert_eq!(app.doc().find_shape(id).expect("back").kind, original);
        let before = tracedraw_render::render_page_image(app.doc(), page, 254.0).expect("before");
        let w = before.width() as usize;
        let seam = before
            .data()
            .chunks(4)
            .zip(after.data().chunks(4))
            .enumerate()
            .filter(|(i, _)| (95..=110).contains(&(i % w)))
            .filter(|(_, (a, b))| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
            .count();
        // The picture's top or bottom edge row may be smoothed differently
        // behind an outline, as any cropped picture is; nothing more.
        assert!(seam <= 16 * 2, "{seam}");
        // A slanted freehand cut: two pieces covering the picture.
        let stroke = line(Point::new(-2.0, -3.0), Point::new(42.0, 24.0), 20);
        app.knife_cut(&stroke);
        let got = pieces(&app);
        assert_eq!(got.len(), 2);
        let total: f64 = got.iter().map(|(o, _, _)| area(o)).sum();
        assert!((total - 800.0).abs() < 1e-3, "{total}");
        app.undo();
        // With a gap the outlines are that far apart.
        app.knife.gap = 4.0;
        app.knife(Point::new(20.0, -5.0), Point::new(20.0, 25.0));
        let mut xs: Vec<(f64, f64)> = pieces(&app)
            .iter()
            .map(|(o, _, _)| (o.bounding_box().x0, o.bounding_box().x1))
            .collect();
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(
            (xs[0].1 - 18.0).abs() < 0.05 && (xs[1].0 - 22.0).abs() < 0.05,
            "{xs:?}"
        );
        app.undo();
        // Kept as one object: one picture behind an outline of two parts.
        app.knife.one_object = true;
        app.knife(Point::new(20.0, -5.0), Point::new(20.0, 25.0));
        let got = pieces(&app);
        assert_eq!(got.len(), 1);
        let subpaths = got[0]
            .0
            .elements()
            .iter()
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
            .count();
        assert_eq!(subpaths, 2);
        app.undo();
        app.knife = Default::default();
        // A cut beside the picture leaves it alone.
        app.knife(Point::new(60.0, 0.0), Point::new(60.0, 10.0));
        assert_eq!(count(&app), 1);
        assert!(matches!(
            app.doc().find_shape(id).map(|s| &s.kind),
            Some(ShapeKind::Bitmap { .. })
        ));
    }

    /// A turned picture is cut where it is drawn and its pieces keep its
    /// turn; a piece (a cropped picture) is cut again along its outline
    /// (#1333).
    #[test]
    fn a_turned_or_cropped_picture_is_cut_where_it_is_drawn() {
        let mut app = App::headless();
        let id = picture(&mut app);
        let turn = Affine::rotate_about(0.4, Point::new(20.0, 10.0));
        app.engine
            .run(&Command::TransformShapes {
                shapes: vec![id],
                transform: turn,
            })
            .expect("turn");
        app.select(Vec::new());
        app.knife(Point::new(20.0, -10.0), Point::new(20.0, 30.0));
        let got = pieces(&app);
        assert_eq!(got.len(), 2);
        for (outline, _, t) in &got {
            assert!((t.as_coeffs()[1] - turn.as_coeffs()[1]).abs() < 1e-9);
            let b = outline.bounding_box();
            assert!(b.x1 < 20.0 + 1e-5 || b.x0 > 20.0 - 1e-5, "{b:?}");
        }
        // Cut the pieces across: four cropped pictures, all of the picture.
        app.select(Vec::new());
        app.knife(Point::new(-10.0, 10.0), Point::new(50.0, 10.0));
        let got = pieces(&app);
        assert_eq!(got.len(), 4);
        let total: f64 = got.iter().map(|(o, _, _)| area(o)).sum();
        assert!((total - 800.0).abs() < 1e-3, "{total}");
        app.undo();
        assert_eq!(pieces(&app).len(), 2, "one undo step");
    }
}
