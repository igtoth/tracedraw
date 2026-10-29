//! Writes the showcase drawings the README screenshots are made from:
//! `cargo run -p tracedraw-cli --example showcase -- <out-dir>`
//!
//! - `poster.tdraw`: an illustrated poster on five layers (gradients, a
//!   contour glow, a blend, a drop shadow, artistic text).
//! - `effects.tdraw`: six live effects side by side.
//! - `type.tdraw`: a page of type (headline, two-column paragraph text with
//!   a drop cap, text on a path and a table).
//! - `fills.tdraw`: every fill type.
//! - `photo.tdraw`: the poster rendered and placed as a bitmap.
//! - `leaf.tdraw`: a leaf of smooth curves, for node editing.
//! - `brushes.tdraw`: a landscape painted with the painterly brushes.
//! - `room.tdraw`: a room in one-point perspective (floor tiles, wall
//!   panels and ceiling lights laid onto the planes of the default grid).
//!
//! Everything is drawn here from scratch; the fonts are open fonts (Inter
//! and Lora) and fall back to the system's when missing.

use std::path::{Path, PathBuf};
use tracedraw_core::live::Effect;
use tracedraw_core::{
    effects::Distort, Affine, BezPath, Color, Document, Fill, Fountain, FountainKind, Layer,
    LayerId, Mesh, ParagraphStyle, Pattern, PatternTile, Point, Rect, Shadow, Shape, ShapeKind,
    Size, Stop, Stroke, Table, TextAlign, TextOnPath, TextSpan, Texture, TextureKind, Vec2,
};

const SANS: &str = "Inter";
const SERIF: &str = "Lora";

fn hex(s: &str) -> Color {
    Color::from_hex(s).unwrap_or(Color::BLACK)
}

/// A document under construction, adding to its current layer.
struct Build {
    doc: Document,
    layer: LayerId,
}

impl Build {
    fn new(title: &str, size: Size) -> Self {
        let doc = Document::new(title, size);
        let layer = doc.pages[0].layers[0].id;
        Build { doc, layer }
    }

    /// Rename the first layer, or add a new one on top, and draw on it.
    fn layer(&mut self, name: &str) {
        let page = &self.doc.pages[0];
        let first_empty = page.layers.len() == 1 && page.layers[0].shapes.is_empty();
        if first_empty {
            self.doc.pages[0].layers[0].name = name.into();
            self.layer = self.doc.pages[0].layers[0].id;
            return;
        }
        let id = self.doc.ids_mut().layer();
        self.doc.pages[0].layers.push(Layer::new(id, name));
        self.layer = id;
    }

    /// A new shape with no fill and no outline.
    fn shape(&mut self, kind: ShapeKind) -> Shape {
        let id = self.doc.ids_mut().shape();
        let mut s = Shape::new(id, kind);
        s.fill = Fill::None;
        s.stroke = None;
        s
    }

    fn add(&mut self, mut s: Shape, name: &str) {
        s.name = Some(name.into());
        let layer = self.layer;
        if let Ok(l) = self.doc.layer_mut(layer) {
            l.shapes.push(s);
        }
    }

    fn save(&self, dir: &Path, file: &str) {
        let path: PathBuf = dir.join(file);
        match tracedraw_io::save_native(&self.doc, &path) {
            Ok(()) => println!("wrote {}", path.display()),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64, radius: f64) -> ShapeKind {
    ShapeKind::Rect {
        rect: Rect::new(x0, y0, x1, y1),
        radius,
        corners: None,
    }
}

fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> ShapeKind {
    ShapeKind::Ellipse {
        rect: Rect::new(cx - rx, cy - ry, cx + rx, cy + ry),
        arc: None,
    }
}

fn polygon(pts: &[(f64, f64)]) -> ShapeKind {
    let mut p = BezPath::new();
    for (i, &(x, y)) in pts.iter().enumerate() {
        if i == 0 {
            p.move_to((x, y));
        } else {
            p.line_to((x, y));
        }
    }
    p.close_path();
    ShapeKind::Path {
        path: p,
        closed: true,
    }
}

fn stops(list: &[(f64, &str)]) -> Vec<Stop> {
    list.iter()
        .map(|&(pos, c)| Stop::new(pos, hex(c)))
        .collect()
}

fn fountain(kind: FountainKind, angle: f64, list: &[(f64, &str)]) -> Fill {
    let mut f = Fountain::two(kind, Color::BLACK, Color::WHITE, angle);
    f.stops = stops(list);
    Fill::Fountain(f)
}

fn span(text: &str, family: &str, size: f64) -> TextSpan {
    TextSpan::new(text, family, size)
}

fn artistic(spans: Vec<TextSpan>, x: f64, y: f64, align: TextAlign) -> ShapeKind {
    ShapeKind::Text {
        spans,
        origin: Point::new(x, y),
        frame: None,
        align,
        para: ParagraphStyle::default(),
        on_path: None,
    }
}

/// Paragraph text in a frame whose top-left corner is `top_left` (the
/// text's origin is the frame's bottom-left corner).
fn paragraph(
    spans: Vec<TextSpan>,
    top_left: (f64, f64),
    size: (f64, f64),
    align: TextAlign,
    para: ParagraphStyle,
) -> ShapeKind {
    ShapeKind::Text {
        spans,
        origin: Point::new(top_left.0, top_left.1 - size.1),
        frame: Some(Size::new(size.0, size.1)),
        align,
        para,
        on_path: None,
    }
}

/// A small deterministic random sequence for the stars.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / ((1u64 << 31) as f64)
    }
}

/// A pine tree: three stacked triangles on `base` at `x`.
fn pine(x: f64, base: f64, h: f64) -> ShapeKind {
    let w = h * 0.42;
    let mut p = BezPath::new();
    for (i, (top, bottom, half)) in [(1.0, 0.45, 0.62), (0.78, 0.22, 0.82), (0.55, 0.0, 1.0)]
        .iter()
        .enumerate()
    {
        let _ = i;
        p.move_to((x, base + h * top));
        p.line_to((x + w * half / 2.0 * 1.15, base + h * bottom));
        p.line_to((x - w * half / 2.0 * 1.15, base + h * bottom));
        p.close_path();
    }
    p.move_to((x - h * 0.03, base - 2.0));
    p.line_to((x + h * 0.03, base - 2.0));
    p.line_to((x + h * 0.03, base + 0.5));
    p.line_to((x - h * 0.03, base + 0.5));
    p.close_path();
    ShapeKind::Path {
        path: p,
        closed: true,
    }
}

fn poster(dir: &Path) -> Document {
    let mut b = Build::new("Quiet Lake", Size::new(210.0, 297.0));
    let horizon = 112.0;

    b.layer("Sky");
    let mut sky = b.shape(rect(0.0, horizon - 1.0, 210.0, 297.0, 0.0));
    sky.fill = fountain(
        FountainKind::Linear,
        90.0,
        &[
            (0.0, "#F7A66E"),
            (0.32, "#E9798B"),
            (0.68, "#6A4E9C"),
            (1.0, "#22245A"),
        ],
    );
    b.add(sky, "Sky");
    let mut rng = Lcg(7);
    for i in 0..22 {
        let x = 6.0 + rng.next() * 198.0;
        let y = 215.0 + rng.next() * 78.0;
        let r = 0.35 + rng.next() * 0.55;
        let mut star = b.shape(ellipse(x, y, r, r));
        star.fill = Fill::Solid(Color::WHITE);
        star.opacity = 0.45 + rng.next() * 0.45;
        b.add(star, &format!("Star {}", i + 1));
    }
    let mut sun = b.shape(ellipse(105.0, 158.0, 21.0, 21.0));
    sun.fill = fountain(
        FountainKind::Radial,
        0.0,
        &[(0.0, "#FFF8DC"), (0.42, "#FFE39C"), (0.71, "#FFB761")],
    );
    sun.effects.push(Effect::Contour {
        steps: 6,
        offset: 3.2,
        outside: true,
        to_center: false,
        fill_to: hex("#EE8F86"),
        outline_to: None,
        color_blend: 0,
        corners: None,
        accel_objects: 0.0,
        accel_colors: 0.0,
    });
    b.add(sun, "Sun");
    for (i, (x, y, s)) in [(52.0, 214.0, 1.0), (61.0, 208.0, 0.75), (148.0, 222.0, 0.9)]
        .iter()
        .enumerate()
    {
        let mut p = BezPath::new();
        p.move_to((x - 4.5 * s, y + 1.6 * s));
        p.quad_to((x - 2.0 * s, y + 2.4 * s), (*x, *y));
        p.quad_to((x + 2.0 * s, y + 2.4 * s), (x + 4.5 * s, y + 1.6 * s));
        let mut bird = b.shape(ShapeKind::Path {
            path: p,
            closed: false,
        });
        let mut st = Stroke::new(hex("#2A2145"), 0.55);
        st.cap = tracedraw_core::LineCap::Round;
        bird.stroke = Some(st);
        b.add(bird, &format!("Bird {}", i + 1));
    }

    b.layer("Mountains");
    let mut far = b.shape(polygon(&[
        (0.0, horizon),
        (0.0, 150.0),
        (18.0, 162.0),
        (34.0, 155.0),
        (52.0, 179.0),
        (70.0, 160.0),
        (88.0, 168.0),
        (104.0, 151.0),
        (122.0, 171.0),
        (141.0, 188.0),
        (160.0, 166.0),
        (178.0, 173.0),
        (196.0, 158.0),
        (210.0, 164.0),
        (210.0, horizon),
    ]));
    far.fill = fountain(
        FountainKind::Linear,
        90.0,
        &[(0.0, "#55467F"), (1.0, "#8C6CA9")],
    );
    b.add(far, "Far range");
    let mut near = b.shape(polygon(&[
        (0.0, horizon - 0.5),
        (0.0, 132.0),
        (22.0, 141.0),
        (40.0, 131.0),
        (62.0, 149.0),
        (84.0, 134.0),
        (100.0, 141.0),
        (124.0, 126.0),
        (150.0, 147.0),
        (170.0, 133.0),
        (190.0, 143.0),
        (210.0, 130.0),
        (210.0, horizon - 0.5),
    ]));
    near.fill = fountain(
        FountainKind::Linear,
        90.0,
        &[(0.0, "#28214D"), (1.0, "#3E3268")],
    );
    b.add(near, "Near range");

    b.layer("Lake");
    let mut water = b.shape(rect(0.0, 30.0, 210.0, horizon, 0.0));
    water.fill = fountain(
        FountainKind::Linear,
        90.0,
        &[(0.0, "#1A1C47"), (0.62, "#5E4787"), (1.0, "#D98B8F")],
    );
    b.add(water, "Water");
    let mut glint = b.shape(rect(85.0, horizon - 3.4, 125.0, horizon - 1.4, 1.0));
    glint.fill = Fill::Solid(hex("#FFE0A0"));
    glint.opacity = 0.9;
    let mut glint_end = b.shape(rect(100.0, 74.0, 110.0, 75.0, 0.5));
    glint_end.fill = Fill::Solid(hex("#FFB56B"));
    glint_end.opacity = 0.35;
    glint.effects.push(Effect::Blend {
        end: Box::new(glint_end),
        steps: 9,
        accel_objects: 0.0,
        accel_colors: 0.0,
        rotation: 0.0,
        path: None,
        rotate_on_path: false,
        end_id: None,
        options: Default::default(),
    });
    b.add(glint, "Sun reflection");

    b.layer("Shore");
    let mut shore_path = BezPath::new();
    shore_path.move_to((0.0, 0.0));
    shore_path.line_to((0.0, 50.0));
    shore_path.curve_to((60.0, 62.0), (140.0, 26.0), (210.0, 38.0));
    shore_path.line_to((210.0, 0.0));
    shore_path.close_path();
    let mut shore = b.shape(ShapeKind::Path {
        path: shore_path,
        closed: true,
    });
    shore.fill = Fill::Solid(hex("#130F2A"));
    b.add(shore, "Shore");
    for (i, (x, base, h)) in [
        (9.0, 47.0, 30.0),
        (21.0, 49.0, 48.0),
        (33.0, 50.0, 36.0),
        (176.0, 36.0, 31.0),
        (189.0, 36.5, 54.0),
        (201.0, 37.0, 38.0),
    ]
    .iter()
    .enumerate()
    {
        let mut tree = b.shape(pine(*x, *base, *h));
        tree.fill = Fill::Solid(hex("#0B0920"));
        b.add(tree, &format!("Pine {}", i + 1));
    }

    b.layer("Type");
    let mut title_span = span("QUIET LAKE", SANS, 50.0);
    title_span.bold = true;
    title_span.tracking_pct = 16.0;
    let mut title = b.shape(artistic(vec![title_span], 105.0, 247.0, TextAlign::Center));
    title.fill = Fill::Solid(Color::WHITE);
    title.shadow = Some(Shadow {
        offset: Vec2::new(0.0, -1.0),
        opacity: 0.35,
        blur: 2.0,
        color: hex("#140F2E"),
        merge: tracedraw_core::live::MergeMode::Normal,
        ..Default::default()
    });
    b.add(title, "Title");
    let mut sub_span = span("an evening drawn in vectors", SERIF, 16.0);
    sub_span.italic = true;
    let mut sub = b.shape(artistic(vec![sub_span], 105.0, 234.0, TextAlign::Center));
    sub.fill = Fill::Solid(hex("#FFE6D4"));
    b.add(sub, "Subtitle");
    let mut foot_span = span("TRACEDRAW  \u{b7}  SHOWCASE", SANS, 8.5);
    foot_span.bold = true;
    foot_span.tracking_pct = 40.0;
    let mut foot = b.shape(artistic(vec![foot_span], 105.0, 12.0, TextAlign::Center));
    foot.fill = Fill::Solid(Color::WHITE);
    foot.opacity = 0.7;
    b.add(foot, "Footer");
    b.save(dir, "poster.tdraw");
    b.doc
}

/// The poster rendered to pixels and placed as a bitmap, for the bitmap
/// effects.
fn photo(dir: &Path, poster: &Document) {
    let Some(page) = poster.pages.first() else {
        return;
    };
    let Some(pm) = tracedraw_render::render_page_image(poster, page.id, 150.0) else {
        eprintln!("photo: render failed");
        return;
    };
    let (w, h) = (pm.width(), pm.height());
    let png = match pm.encode_png() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("photo: {e}");
            return;
        }
    };
    let mut b = Build::new("Photo", Size::new(210.0, 297.0));
    b.layer("Photo");
    let width = 170.0;
    let height = width * h as f64 / w as f64;
    let y0 = (297.0 - height) / 2.0;
    let s = b.shape(ShapeKind::Bitmap {
        rect: Rect::new(20.0, y0, 20.0 + width, y0 + height),
        width_px: w,
        height_px: h,
        png: png.into(),
        fx: None,
    });
    b.add(s, "Quiet Lake (bitmap)");
    b.save(dir, "photo.tdraw");
}

/// A light rounded panel with a label, for the samplers.
fn cell(b: &mut Build, r: Rect, label: &str) {
    let mut bg = b.shape(rect(r.x0, r.y0, r.x1, r.y1, 4.0));
    bg.fill = Fill::Solid(hex("#F2F4F8"));
    b.add(bg, &format!("{label} panel"));
    let mut l = span(label, SANS, 11.0);
    l.bold = true;
    let mut t = b.shape(artistic(vec![l], r.x0 + 6.0, r.y0 + 6.0, TextAlign::Left));
    t.fill = Fill::Solid(hex("#2B3045"));
    b.add(t, &format!("{label} label"));
}

fn heading(b: &mut Build, title: &str, sub: &str, top: f64) {
    let mut h = span(title, SANS, 28.0);
    h.bold = true;
    let mut t = b.shape(artistic(vec![h], 20.0, top, TextAlign::Left));
    t.fill = Fill::Solid(hex("#1F2437"));
    b.add(t, "Heading");
    let mut s = span(sub, SERIF, 12.0);
    s.italic = true;
    let mut t = b.shape(artistic(vec![s], 20.0, top - 10.0, TextAlign::Left));
    t.fill = Fill::Solid(hex("#5A6278"));
    b.add(t, "Subheading");
}

/// The six panels of the samplers: three columns, two rows.
fn grid() -> Vec<Rect> {
    let mut out = Vec::new();
    for row in 0..2 {
        for col in 0..3 {
            let x0 = 20.0 + col as f64 * 88.5;
            let y1 = 162.0 - row as f64 * 80.0;
            out.push(Rect::new(x0, y1 - 70.0, x0 + 80.0, y1));
        }
    }
    out
}

fn star(cx: f64, cy: f64, r: f64, points: u32, sharpness: f64) -> ShapeKind {
    ShapeKind::Polygon {
        rect: Rect::new(cx - r, cy - r, cx + r, cy + r),
        points,
        sharpness,
        complex: None,
        twist: 0.0,
    }
}

fn effects(dir: &Path) {
    let mut b = Build::new("Live effects", Size::new(297.0, 210.0));
    b.layer("Effects");
    heading(
        &mut b,
        "Live effects",
        "Each effect stays editable from its panel and the property bar.",
        186.0,
    );
    let cells = grid();
    let names = [
        "Blend",
        "Contour",
        "Extrude",
        "Drop Shadow",
        "Envelope",
        "Distort",
    ];
    for (r, name) in cells.iter().zip(names) {
        cell(&mut b, *r, name);
    }

    // Blend: a red circle into a blue star.
    let r = cells[0];
    let cy = r.center().y + 5.0;
    let mut start = b.shape(ellipse(r.x0 + 14.0, cy, 5.0, 5.0));
    start.fill = Fill::Solid(hex("#FF5A5F"));
    let mut end = b.shape(star(r.x1 - 17.0, cy, 13.0, 5, 0.5));
    end.fill = Fill::Solid(hex("#3D7BFF"));
    start.effects.push(Effect::Blend {
        end: Box::new(end),
        steps: 7,
        accel_objects: 0.0,
        accel_colors: 0.0,
        rotation: 0.0,
        path: None,
        rotate_on_path: false,
        end_id: None,
        options: Default::default(),
    });
    b.add(start, "Blend");

    // Contour: concentric rounded squares from amber to pink.
    let r = cells[1];
    let c = Point::new(r.center().x, r.center().y + 5.0);
    let mut sq = b.shape(rect(c.x - 9.0, c.y - 9.0, c.x + 9.0, c.y + 9.0, 3.0));
    sq.fill = Fill::Solid(hex("#FFB000"));
    sq.effects.push(Effect::Contour {
        steps: 5,
        offset: 2.6,
        outside: true,
        to_center: false,
        fill_to: hex("#FF3D7F"),
        outline_to: None,
        color_blend: 0,
        corners: None,
        accel_objects: 0.0,
        accel_colors: 0.0,
    });
    b.add(sq, "Contour");

    // Extrude: "3D" with a shaded depth.
    let r = cells[2];
    let mut s = span("3D", SANS, 64.0);
    s.bold = true;
    let mut t = b.shape(artistic(
        vec![s],
        r.center().x - 4.0,
        r.y0 + 22.0,
        TextAlign::Center,
    ));
    t.fill = Fill::Solid(hex("#34C38F"));
    t.effects.push(Effect::Extrude {
        depth: Vec2::new(5.0, 4.5),
        vanishing: None,
        amount: 1.0,
        shade_from: Some(hex("#1F8A63")),
        shade_to: Some(hex("#0B3D2B")),
        light_angle: 45.0,
        light_intensity: 0.5,
        bevel: 0.0,
        kind: None,
        on_object: false,
        solid: None,
        space: Default::default(),
    });
    b.add(t, "Extrude");

    // Drop shadow: a card with an avatar and two text lines.
    let r = cells[3];
    let c = Point::new(r.center().x, r.center().y + 5.0);
    let mut card = b.shape(rect(c.x - 26.0, c.y - 15.0, c.x + 26.0, c.y + 15.0, 3.0));
    card.fill = Fill::Solid(Color::WHITE);
    card.shadow = Some(Shadow {
        offset: Vec2::new(1.5, -2.2),
        opacity: 0.35,
        blur: 3.0,
        color: hex("#1F2437"),
        merge: tracedraw_core::live::MergeMode::Normal,
        ..Default::default()
    });
    b.add(card, "Card");
    let mut avatar = b.shape(ellipse(c.x - 15.0, c.y + 3.0, 6.0, 6.0));
    avatar.fill = fountain(
        FountainKind::Linear,
        45.0,
        &[(0.0, "#7B5CFF"), (1.0, "#3D7BFF")],
    );
    b.add(avatar, "Avatar");
    for (i, (w, dy)) in [(28.0, 5.5), (20.0, 0.5)].iter().enumerate() {
        let mut line = b.shape(rect(
            c.x - 6.0,
            c.y + dy,
            c.x - 6.0 + w,
            c.y + dy + 2.6,
            1.3,
        ));
        line.fill = Fill::Solid(hex(if i == 0 { "#C9CEDA" } else { "#E1E4EC" }));
        b.add(line, "Line");
    }
    let mut bar = b.shape(rect(c.x - 21.0, c.y - 10.0, c.x + 21.0, c.y - 6.5, 1.7));
    bar.fill = Fill::Solid(hex("#3D7BFF"));
    b.add(bar, "Button");

    // Envelope: a word bent into an arch.
    let r = cells[4];
    let mut s = span("ARCH", SANS, 46.0);
    s.bold = true;
    let mut t = b.shape(artistic(
        vec![s],
        r.center().x,
        r.center().y - 2.0,
        TextAlign::Center,
    ));
    t.fill = fountain(
        FountainKind::Linear,
        0.0,
        &[(0.0, "#7B5CFF"), (1.0, "#FF5A9E")],
    );
    let lb = t.local_path();
    let bb = tracedraw_core::geometry::Shape::bounding_box(&lb);
    if bb.width() > 1.0 {
        use tracedraw_core::envelope::{Frame, Mapping, Mode};
        let mut frame = Frame::around(bb);
        let lift = bb.height() * 0.55;
        // BL, BM, BR, RM, TR, TM, TL, LM: raise the middles, drop the sides,
        // each side one arc.
        for (i, d) in [(1, lift), (5, lift), (3, -lift * 0.15), (7, -lift * 0.15)] {
            frame.move_node(i, Vec2::new(0.0, d), Mode::SingleArc);
        }
        t.effects
            .push(Effect::from_envelope(&frame, false, Mapping::Putty));
    }
    b.add(t, "Envelope");

    // Distort: a twisted star.
    let r = cells[5];
    let mut st = b.shape(star(r.center().x, r.center().y + 5.0, 21.0, 9, 0.55));
    st.fill = fountain(
        FountainKind::Radial,
        0.0,
        &[(0.0, "#FFE45C"), (0.7, "#FF7A3D")],
    );
    st.effects.push(Effect::Distort(Distort::Twister {
        angle_deg: 140.0,
        center: None,
    }));
    b.add(st, "Distort");
    b.save(dir, "effects.tdraw");
}

const BODY: &str = "Vector drawings are made of instructions rather than pixels: a curve is a few points and the handles that bend it, so it stays sharp at any size and on any press. TraceDraw keeps every object editable for as long as you need it. A contour can change its number of steps after the fact, a blend can follow a new path, and paragraph text reflows the moment its frame is resized.\nType works the same way. Paragraph text runs in columns with justification, hyphenation, indents, tabs and drop caps, and it can wrap around the objects it meets. Artistic text takes effects like any other shape, and every character can still be retyped. When a page is done, it goes out as PDF, SVG, EPS or a bitmap, with fonts turned into curves or kept as text.\nNothing here needs a plug-in or an account. Drawings open from the files you already have, stay on your computer, and save to an open format that any program can read.";

fn typography(dir: &Path) {
    let mut b = Build::new("The shape of words", Size::new(210.0, 297.0));
    b.layer("Text");
    let mut kicker = span("FIELD NOTES", SANS, 9.0);
    kicker.bold = true;
    kicker.tracking_pct = 30.0;
    let mut t = b.shape(artistic(vec![kicker], 20.0, 268.0, TextAlign::Left));
    t.fill = Fill::Solid(hex("#E0475B"));
    b.add(t, "Kicker");
    let mut head = span("The shape of words", SERIF, 38.0);
    head.bold = true;
    let mut t = b.shape(artistic(vec![head], 20.0, 251.0, TextAlign::Left));
    t.fill = Fill::Solid(hex("#1F2437"));
    b.add(t, "Headline");
    let mut deck = span(
        "Paragraph text in two columns, a drop cap, hyphenation and text on a path, all live and editable.",
        SERIF,
        13.0,
    );
    deck.italic = true;
    let mut t = b.shape(paragraph(
        vec![deck],
        (20.0, 241.0),
        (170.0, 18.0),
        TextAlign::Left,
        ParagraphStyle {
            leading_pct: 115.0,
            ..ParagraphStyle::default()
        },
    ));
    t.fill = Fill::Solid(hex("#5A6278"));
    b.add(t, "Deck");
    let mut rule = BezPath::new();
    rule.move_to((20.0, 218.0));
    rule.line_to((190.0, 218.0));
    let mut line = b.shape(ShapeKind::Path {
        path: rule,
        closed: false,
    });
    line.stroke = Some(Stroke::new(hex("#D0D4DE"), 0.35));
    b.add(line, "Rule");
    let mut t = b.shape(paragraph(
        vec![span(BODY, SERIF, 10.5)],
        (20.0, 210.0),
        (170.0, 74.0),
        TextAlign::Justify,
        ParagraphStyle {
            leading_pct: 125.0,
            columns: 2,
            gutter: 8.0,
            drop_cap_lines: 3,
            hyphenate: true,
            space_after: 2.0,
            ..ParagraphStyle::default()
        },
    ));
    t.fill = Fill::Solid(hex("#2B3045"));
    b.add(t, "Body");

    // Text on a wave.
    let pts: Vec<Point> = (0..=8)
        .map(|i| {
            let x = 20.0 + i as f64 * 170.0 / 8.0;
            let y = 88.0 + 7.0 * (i as f64 * std::f64::consts::PI / 2.0).sin();
            Point::new(x, y)
        })
        .collect();
    let wave = tracedraw_core::geometry::smooth_path(&pts, false);
    let mut w = b.shape(ShapeKind::Path {
        path: wave.clone(),
        closed: false,
    });
    let mut ws = Stroke::new(hex("#E0475B"), 0.3);
    ws.dash = vec![4.0, 3.0];
    w.stroke = Some(ws);
    b.add(w, "Wave");
    let mut s = span(
        "Text follows any curve you draw, and stays text.",
        SANS,
        15.0,
    );
    s.bold = true;
    // The path is kept in the text's local space, relative to its origin.
    let origin = Point::new(20.0, 88.0);
    let local_wave = Affine::translate(-origin.to_vec2()) * wave;
    let mut on = b.shape(ShapeKind::Text {
        spans: vec![s],
        origin,
        frame: None,
        align: TextAlign::Left,
        para: ParagraphStyle::default(),
        on_path: Some(TextOnPath {
            path: local_wave,
            offset: 4.0,
            distance: 1.6,
            mirror: false,
            ..Default::default()
        }),
    });
    on.fill = Fill::Solid(hex("#1F2437"));
    b.add(on, "Text on path");

    // A table of tools.
    let mut table = Table::new(
        Rect::new(20.0, 22.0, 190.0, 64.0),
        4,
        3,
        Some(Stroke::new(hex("#C9CEDA"), 0.25)),
    );
    table.col_widths = vec![40.0, 30.0, 100.0];
    table.padding = 2.0;
    let rows = [
        ["Tool", "Key", "What it does"],
        ["Pick", "Space", "Select, move, scale and rotate objects"],
        ["Shape", "F10", "Edit nodes, handles and segments"],
        ["Text", "F8", "Artistic and paragraph text"],
    ];
    for cellv in table.cells.iter_mut() {
        let (r, c) = (cellv.row as usize, cellv.col as usize);
        let text = rows
            .get(r)
            .and_then(|row| row.get(c))
            .copied()
            .unwrap_or("");
        let mut s = span(text, SANS, 10.0);
        s.bold = r == 0;
        if r == 0 {
            s.fill = Some(Fill::Solid(Color::WHITE));
            cellv.fill = Some(Fill::Solid(hex("#1F2437")));
        } else if r % 2 == 0 {
            cellv.fill = Some(Fill::Solid(hex("#F5F6FA")));
        }
        cellv.text = vec![s];
    }
    let mut t = b.shape(ShapeKind::Table(table));
    t.fill = Fill::Solid(hex("#2B3045"));
    b.add(t, "Tools table");
    b.save(dir, "type.tdraw");
}

fn fills(dir: &Path) {
    let mut b = Build::new("Fills", Size::new(297.0, 210.0));
    b.layer("Fills");
    heading(
        &mut b,
        "Fills",
        "Uniform, gradient, pattern, texture and mesh fills, each with its editor.",
        186.0,
    );
    let tiles: Vec<Rect> = (0..8)
        .map(|i| {
            let (row, col) = (i / 4, i % 4);
            let x0 = 20.0 + col as f64 * 66.0;
            let y1 = 160.0 - row as f64 * 78.0;
            Rect::new(x0, y1 - 56.0, x0 + 56.0, y1)
        })
        .collect();
    let labels = [
        "Uniform", "Linear", "Radial", "Conical", "Square", "Pattern", "Texture", "Mesh",
    ];
    for (i, (r, label)) in tiles.iter().zip(labels).enumerate() {
        let fill = match i {
            0 => Fill::Solid(hex("#3D7BFF")),
            1 => fountain(
                FountainKind::Linear,
                30.0,
                &[(0.0, "#FF5A5F"), (0.5, "#FFB000"), (1.0, "#FFE45C")],
            ),
            2 => fountain(
                FountainKind::Radial,
                0.0,
                &[(0.0, "#FFFFFF"), (0.7, "#7B5CFF")],
            ),
            3 => fountain(
                FountainKind::Conical,
                0.0,
                &[
                    (0.0, "#FF5A5F"),
                    (0.25, "#FFB000"),
                    (0.5, "#34C38F"),
                    (0.75, "#3D7BFF"),
                    (1.0, "#7B5CFF"),
                ],
            ),
            4 => fountain(
                FountainKind::Square,
                0.0,
                &[(0.0, "#E9FFF6"), (1.0, "#1F8A63")],
            ),
            5 => Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Waves,
                front: hex("#3D7BFF"),
                back: hex("#E8F0FF"),
                size_mm: 8.0,
                placement: Default::default(),
            }),
            6 => Fill::Texture(Texture {
                kind: TextureKind::Marble,
                color_a: hex("#F4EFE6"),
                color_b: hex("#8C7B6B"),
                scale: 14.0,
                seed: 3,
            }),
            _ => {
                let mut m = Mesh::new(*r, 2, 2, Color::WHITE);
                let colors = [
                    "#3D7BFF", "#7B5CFF", "#FF5A9E", "#34C38F", "#FFFFFF", "#FF5A5F", "#FFE45C",
                    "#FFB000", "#FF7A3D",
                ];
                for (n, c) in m.nodes.iter_mut().zip(colors) {
                    n.color = hex(c);
                }
                m.smooth = true;
                Fill::Mesh(m)
            }
        };
        let mut s = b.shape(rect(r.x0, r.y0, r.x1, r.y1, 5.0));
        s.fill = fill;
        b.add(s, label);
        let mut l = span(label, SANS, 11.0);
        l.bold = true;
        let mut t = b.shape(artistic(vec![l], r.x0, r.y0 - 9.0, TextAlign::Left));
        t.fill = Fill::Solid(hex("#2B3045"));
        b.add(t, &format!("{label} label"));
    }
    b.save(dir, "fills.tdraw");
}

/// A leaf drawn with smooth curves, for node editing.
fn leaf(dir: &Path) {
    let mut b = Build::new("Leaf", Size::new(297.0, 210.0));
    b.layer("Leaf");
    let outline = tracedraw_core::geometry::smooth_path(
        &[
            Point::new(252.0, 121.0),
            Point::new(207.0, 162.0),
            Point::new(140.0, 172.0),
            Point::new(84.0, 150.0),
            Point::new(56.0, 108.0),
            Point::new(88.0, 70.0),
            Point::new(150.0, 53.0),
            Point::new(212.0, 76.0),
        ],
        true,
    );
    let mut l = b.shape(ShapeKind::Path {
        path: outline,
        closed: true,
    });
    l.fill = fountain(
        FountainKind::Linear,
        35.0,
        &[(0.0, "#0E7A4C"), (0.55, "#2FBF71"), (1.0, "#9BE15D")],
    );
    l.stroke = Some(Stroke::new(hex("#0B5E3B"), 0.5));
    b.add(l, "Leaf");
    let vein = tracedraw_core::geometry::smooth_path(
        &[
            Point::new(58.0, 108.0),
            Point::new(112.0, 111.0),
            Point::new(175.0, 116.0),
            Point::new(246.0, 121.0),
        ],
        false,
    );
    let mut v = b.shape(ShapeKind::Path {
        path: vein,
        closed: false,
    });
    let mut vs = Stroke::new(hex("#E6FFF1"), 0.9);
    vs.cap = tracedraw_core::LineCap::Round;
    v.stroke = Some(vs);
    v.opacity = 0.85;
    b.add(v, "Vein");
    let stem = tracedraw_core::geometry::smooth_path(
        &[
            Point::new(57.0, 108.0),
            Point::new(36.0, 92.0),
            Point::new(24.0, 64.0),
        ],
        false,
    );
    let mut st = b.shape(ShapeKind::Path {
        path: stem,
        closed: false,
    });
    let mut ss = Stroke::new(hex("#0B5E3B"), 1.6);
    ss.cap = tracedraw_core::LineCap::Round;
    st.stroke = Some(ss);
    b.add(st, "Stem");
    for (i, (x, y, r)) in [(120.0, 140.0, 4.0), (176.0, 92.0, 2.6), (205.0, 138.0, 1.8)]
        .iter()
        .enumerate()
    {
        let mut d = b.shape(ellipse(*x, *y, *r, *r));
        d.fill = fountain(
            FountainKind::Radial,
            0.0,
            &[(0.0, "#FFFFFF"), (0.7, "#BFF5D6")],
        );
        d.opacity = 0.8;
        b.add(d, &format!("Dew drop {}", i + 1));
    }
    b.save(dir, "leaf.tdraw");
}

/// A brushstroke along smooth points.
fn brushstroke(
    b: &mut Build,
    pts: &[(f64, f64)],
    color: &str,
    size: f64,
    brush: tracedraw_core::painterly::PainterlyBrush,
    seed: u32,
    name: &str,
) {
    let points: Vec<Point> = pts.iter().map(|(x, y)| Point::new(*x, *y)).collect();
    let path = tracedraw_core::geometry::smooth_path(&points, false);
    let mut s = b.shape(ShapeKind::Path {
        path,
        closed: false,
    });
    let mut st = Stroke::new(hex(color), size);
    st.painterly = Some(tracedraw_core::painterly::Painterly {
        brush,
        transparency: 0.1,
        seed,
    });
    s.stroke = Some(st);
    b.add(s, name);
}

fn brushes(dir: &Path) {
    use tracedraw_core::painterly::PainterlyBrush as P;
    let mut b = Build::new("Brushes", Size::new(297.0, 210.0));
    b.layer("Paper");
    let mut paper = b.shape(rect(0.0, 0.0, 297.0, 210.0, 0.0));
    paper.fill = fountain(
        FountainKind::Linear,
        90.0,
        &[(0.0, "#F7EFE2"), (1.0, "#FFF9F0")],
    );
    b.add(paper, "Paper");
    b.layer("Sky");
    for (i, y) in [182.0, 170.0, 158.0].iter().enumerate() {
        brushstroke(
            &mut b,
            &[
                (14.0, *y),
                (90.0, y + 4.0),
                (180.0, y - 3.0),
                (283.0, y + 2.0),
            ],
            ["#3F82D0", "#5E9BDD", "#86B6E8"][i],
            18.0,
            P::Watercolor,
            10 + i as u32,
            "Sky wash",
        );
    }
    brushstroke(
        &mut b,
        &[(40.0, 168.0), (70.0, 172.0), (100.0, 167.0)],
        "#FFFFFF",
        14.0,
        P::Cloud,
        21,
        "Cloud",
    );
    brushstroke(
        &mut b,
        &[(180.0, 176.0), (215.0, 180.0), (250.0, 175.0)],
        "#FFFFFF",
        12.0,
        P::Cloud,
        22,
        "Cloud",
    );
    let mut sun = b.shape(ellipse(232.0, 150.0, 13.0, 13.0));
    sun.fill = fountain(
        FountainKind::Radial,
        0.0,
        &[(0.0, "#FFE48A"), (1.0, "#F6B44B")],
    );
    b.add(sun, "Sun");
    b.layer("Hills");
    brushstroke(
        &mut b,
        &[
            (10.0, 112.0),
            (70.0, 132.0),
            (140.0, 118.0),
            (210.0, 136.0),
            (290.0, 120.0),
        ],
        "#5E8C6A",
        22.0,
        P::InkWash,
        31,
        "Far hills",
    );
    brushstroke(
        &mut b,
        &[
            (10.0, 92.0),
            (80.0, 104.0),
            (160.0, 88.0),
            (230.0, 100.0),
            (290.0, 90.0),
        ],
        "#2F6B3F",
        22.0,
        P::DryBrush,
        32,
        "Near hills",
    );
    b.layer("Meadow");
    for (i, y) in [70.0, 56.0, 42.0, 28.0].iter().enumerate() {
        brushstroke(
            &mut b,
            &[
                (8.0, *y),
                (100.0, y + 3.0),
                (200.0, y - 2.0),
                (290.0, y + 1.0),
            ],
            ["#6BAA4E", "#5C9A42", "#4C8A36", "#3E7A2C"][i],
            14.0,
            if i % 2 == 0 { P::Bristle } else { P::Fur },
            40 + i as u32,
            "Grass",
        );
    }
    brushstroke(
        &mut b,
        &[(30.0, 52.0), (120.0, 60.0), (210.0, 48.0), (270.0, 58.0)],
        "#F2C14E",
        10.0,
        P::Speckle,
        51,
        "Flowers",
    );
    brushstroke(
        &mut b,
        &[(40.0, 30.0), (120.0, 36.0), (200.0, 26.0), (260.0, 34.0)],
        "#E86A92",
        8.0,
        P::Spray,
        52,
        "Blossoms",
    );
    b.layer("Title");
    brushstroke(
        &mut b,
        &[(22.0, 18.0), (60.0, 16.0), (100.0, 19.0)],
        "#2E2A26",
        5.0,
        P::Charcoal,
        61,
        "Signature",
    );
    brushstroke(
        &mut b,
        &[(196.0, 22.0), (225.0, 16.0), (255.0, 24.0), (282.0, 17.0)],
        "#C04A7A",
        7.0,
        P::Ribbon,
        62,
        "Ribbon",
    );
    b.save(dir, "brushes.tdraw");
}

fn room(dir: &Path) {
    // The default one-point grid of Draw in Perspective on this page.
    let (w, h) = (297.0, 210.0);
    let vp = Point::new(w * 0.5, h * 0.55);
    let (l, r, bt, tp) = (w * 0.15, w * 0.15 + w * 0.7, h * 0.08, h * 0.08 + h * 0.75);
    let f = 0.7;
    let toward = |p: Point| p + (vp - p) * f;
    let (lb, rb, lt, rt) = (
        Point::new(l, bt),
        Point::new(r, bt),
        Point::new(l, tp),
        Point::new(r, tp),
    );
    let unit = [
        Point::new(0.0, 0.0),
        Point::new(1.0, 0.0),
        Point::new(1.0, 1.0),
        Point::new(0.0, 1.0),
    ];
    let planes = [
        [lb, rb, toward(rb), toward(lb)],
        [lb, toward(lb), toward(lt), lt],
        [toward(rb), rb, rt, toward(rt)],
        [toward(lt), toward(rt), rt, lt],
        [toward(lb), toward(rb), toward(rt), toward(lt)],
    ];
    let mut b = Build::new("Room", Size::new(w, h));
    let laid = |b: &mut Build,
                plane: usize,
                u0: f64,
                w0: f64,
                u1: f64,
                w1: f64,
                fill: Fill,
                name: &str| {
        let q = planes[plane];
        let Some(hm) = tracedraw_core::live::homography(&unit, &q) else {
            return;
        };
        let m = |u: f64, v: f64| tracedraw_core::live::map_homography(&hm, Point::new(u, v));
        let corners = [m(u0, w0), m(u1, w0), m(u1, w1), m(u0, w1)];
        let mut bx = Rect::from_points(corners[0], corners[1]);
        for c in &corners[2..] {
            bx = bx.union_pt(*c);
        }
        let mut s = b.shape(rect(bx.x0, bx.y0, bx.x1, bx.y1, 0.0));
        s.fill = fill;
        s.stroke = Some(Stroke::new(hex("#33415C"), 0.3));
        s.effects.push(Effect::Perspective { corners });
        b.add(s, name);
    };
    b.layer("Back wall");
    let mut back = b.shape(rect(
        toward(lb).x,
        toward(lb).y,
        toward(rt).x,
        toward(rt).y,
        0.0,
    ));
    back.fill = fountain(
        FountainKind::Linear,
        90.0,
        &[(0.0, "#F4D9C6"), (1.0, "#FBEBDD")],
    );
    b.add(back, "Back wall");
    b.layer("Floor");
    let n = 6;
    for i in 0..n {
        for j in 0..n {
            let dark = (i + j) % 2 == 0;
            let (u0, w0) = (i as f64 / n as f64, j as f64 / n as f64);
            laid(
                &mut b,
                0,
                u0,
                w0,
                u0 + 1.0 / n as f64,
                w0 + 1.0 / n as f64,
                Fill::Solid(hex(if dark { "#2F4858" } else { "#E8E2D6" })),
                "Tile",
            );
        }
    }
    b.layer("Walls");
    laid(
        &mut b,
        1,
        0.0,
        0.0,
        1.0,
        1.0,
        Fill::Solid(hex("#9BC1BC")),
        "Left wall",
    );
    laid(
        &mut b,
        2,
        0.0,
        0.0,
        1.0,
        1.0,
        Fill::Solid(hex("#ED6A5A")),
        "Right wall",
    );
    laid(
        &mut b,
        3,
        0.0,
        0.0,
        1.0,
        1.0,
        Fill::Solid(hex("#F7F3E3")),
        "Ceiling",
    );
    for k in 0..3 {
        let u = 0.1 + k as f64 * 0.3;
        laid(
            &mut b,
            1,
            u,
            0.35,
            u + 0.2,
            0.75,
            fountain(
                FountainKind::Linear,
                90.0,
                &[(0.0, "#5D9CEC"), (1.0, "#BFE0FF")],
            ),
            "Window",
        );
        laid(
            &mut b,
            2,
            u,
            0.3,
            u + 0.18,
            0.8,
            Fill::Solid(hex("#F4E1A1")),
            "Door",
        );
        laid(
            &mut b,
            3,
            0.35,
            0.15 + k as f64 * 0.28,
            0.65,
            0.25 + k as f64 * 0.28,
            Fill::Solid(hex("#FFF4B8")),
            "Light",
        );
    }
    b.save(dir, "room.tdraw");
}

fn main() {
    tracedraw_text::install();
    let dir = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "showcase".into()));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("{}: {e}", dir.display());
        return;
    }
    let p = poster(&dir);
    photo(&dir, &p);
    effects(&dir);
    typography(&dir);
    fills(&dir);
    leaf(&dir);
    brushes(&dir);
    room(&dir);
}
