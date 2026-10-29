//! The pattern pickers of the Properties panel's Fill part: the vector
//! and bitmap patterns a drawing already holds (fills of its objects, the
//! default fill a Create > Pattern Fill set), the bitmaps on its pages as
//! tiles to be, and the ways to make a new pattern (Object > Create >
//! Pattern Fill, a picture file imported through `crate::files`).

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::{Painter, Rect, Ui, Vec2};
use tracedraw_core::document::{Shape, ShapeKind};
use tracedraw_core::{Fill, Pattern, SharedBytes};

/// The patterns a drawing holds, each once, without their placements.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PatternLibrary {
    /// Vector patterns, in the order they were met.
    pub vector: Vec<Pattern>,
    /// Bitmap patterns, in the order they were met.
    pub bitmap: Vec<Pattern>,
    /// Bitmaps on the pages (width, height in pixels, the picture, its
    /// width on the page in mm): each can become a bitmap pattern's tile.
    pub pictures: Vec<(u32, u32, SharedBytes, f64)>,
}

impl PatternLibrary {
    /// A pattern as the library keeps it: its tile alone.
    fn plain(p: &Pattern) -> Pattern {
        p.clone().with_placement(Default::default())
    }

    fn add(&mut self, fill: &Fill) {
        let Fill::Pattern(p) = fill else {
            return;
        };
        let plain = Self::plain(p);
        let list = match p {
            Pattern::Vector { .. } => &mut self.vector,
            Pattern::Bitmap { .. } => &mut self.bitmap,
            Pattern::TwoColor { .. } => return,
        };
        if !list.contains(&plain) {
            list.push(plain);
        }
    }

    fn add_shape(&mut self, s: &Shape) {
        self.add(&s.fill);
        if let Some(st) = &s.stroke {
            if let Some(f) = st.fill.as_deref() {
                self.add(f);
            }
        }
        for f in &s.extra_fills {
            self.add(f);
        }
        match &s.kind {
            ShapeKind::Group { children } => children.iter().for_each(|c| self.add_shape(c)),
            ShapeKind::ClipFrame { frame, contents } => {
                self.add_shape(frame);
                contents.iter().for_each(|c| self.add_shape(c));
            }
            ShapeKind::Bitmap {
                rect,
                width_px,
                height_px,
                png,
                ..
            } if !self.pictures.iter().any(|(_, _, p, _)| *p == *png) => {
                self.pictures
                    .push((*width_px, *height_px, png.clone(), rect.width()));
            }
            _ => {}
        }
    }

    /// The bitmap pattern made of the page picture `i` of `pictures`: the
    /// tile as wide as the picture is on the page.
    pub fn picture_pattern(&self, i: usize) -> Option<Pattern> {
        let (w, h, png, width_mm) = self.pictures.get(i)?;
        Some(Pattern::Bitmap {
            png: png.clone(),
            width_px: *w,
            height_px: *h,
            size_mm: width_mm.max(0.1),
            placement: Default::default(),
        })
    }

    /// Where `p` stands in the library (its kind's list), if anywhere.
    pub fn position(&self, p: &Pattern) -> Option<usize> {
        let plain = Self::plain(p);
        match p {
            Pattern::Vector { .. } => self.vector.iter().position(|v| *v == plain),
            Pattern::Bitmap { .. } => self.bitmap.iter().position(|b| *b == plain),
            Pattern::TwoColor { .. } => None,
        }
    }
}

/// What a pattern picker asks the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternAction {
    /// Object > Create > Pattern Fill, Vector.
    CreateVector,
    /// Object > Create > Pattern Fill, Bitmap.
    CreateBitmap,
    /// A picture file as a bitmap pattern on the selection.
    ImportBitmap,
}

/// The pickers' link to the drawing: its patterns, fetched when first
/// needed (not every frame), and what the pickers asked for.
pub struct PatternPickers<'a> {
    library: Option<PatternLibrary>,
    fetch: &'a dyn Fn() -> PatternLibrary,
    pub action: Option<PatternAction>,
}

impl<'a> PatternPickers<'a> {
    pub fn new(fetch: &'a dyn Fn() -> PatternLibrary) -> Self {
        PatternPickers {
            library: None,
            fetch,
            action: None,
        }
    }

    pub fn library(&mut self) -> &PatternLibrary {
        self.library.get_or_insert_with(|| (self.fetch)())
    }
}

impl App {
    /// The vector and bitmap patterns the drawing holds, and its pictures.
    pub fn pattern_library(&self) -> PatternLibrary {
        let doc = self.doc();
        let mut lib = PatternLibrary::default();
        // The default fill first: a pattern just created lands there.
        lib.add(&doc.defaults.graphic.fill);
        for layer in doc.all_layers() {
            for s in &layer.shapes {
                lib.add_shape(s);
            }
        }
        lib
    }

    /// Fill > Bitmap pattern > Import: a picture file becomes a bitmap
    /// pattern on the selection (one undo step), its tile as wide as the
    /// picture at the resolution the file records (else 96 dpi).
    pub fn import_pattern_bitmap(&mut self, path: &std::path::Path) {
        let bytes = match crate::files::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.status = trf(
                    "status.could_not_read",
                    &[("p", &path.display().to_string()), ("e", &e.to_string())],
                );
                return;
            }
        };
        let img = match image::load_from_memory(&bytes) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.status = trf(
                    "status.could_not_read",
                    &[("p", &path.display().to_string()), ("e", &e.to_string())],
                );
                return;
            }
        };
        let (w, h) = (img.width(), img.height());
        let Some(png) = crate::bitmap_fx::encode(&img) else {
            return;
        };
        let dpi = crate::image_dpi::resolution(&bytes)
            .map(|(x, _)| x)
            .filter(|d| d.is_finite() && *d > 1.0)
            .unwrap_or(96.0);
        let fill = Fill::Pattern(Pattern::Bitmap {
            png: png.into(),
            width_px: w,
            height_px: h,
            size_mm: (f64::from(w) * 25.4 / dpi).max(0.1),
            placement: Default::default(),
        });
        self.apply_fill(fill);
    }

    /// Run what a pattern picker asked for.
    pub fn run_pattern_action(&mut self, action: PatternAction) {
        match action {
            PatternAction::CreateVector => {
                self.dialog = crate::ui::dialogs::Dialog::CreatePattern { vector: true };
            }
            PatternAction::CreateBitmap => {
                self.dialog = crate::ui::dialogs::Dialog::CreatePattern { vector: false };
            }
            PatternAction::ImportBitmap => {
                crate::files::Dialog::new()
                    .add_filter(
                        tr("file.images"),
                        &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff"],
                    )
                    .pick_file(self, |app, path| app.import_pattern_bitmap(&path));
            }
        }
    }
}

/// The name a picker shows for a library pattern: its number and size.
pub fn pattern_name(p: &Pattern, index: usize) -> String {
    let n = (index + 1).to_string();
    match p {
        Pattern::Vector { shapes, tile, .. } => trf(
            "docker.vector_pattern_n",
            &[
                ("i", &n),
                ("n", &shapes.len().to_string()),
                ("w", &format!("{:.1}", tile.width)),
                ("h", &format!("{:.1}", tile.height)),
            ],
        ),
        Pattern::Bitmap {
            width_px,
            height_px,
            ..
        } => trf(
            "docker.bitmap_pattern_n",
            &[
                ("i", &n),
                ("w", &width_px.to_string()),
                ("h", &height_px.to_string()),
            ],
        ),
        Pattern::TwoColor { tile, .. } => tile.name().to_string(),
    }
}

/// A small square button with a picture; true when clicked.
pub fn picture_button(
    ui: &mut Ui,
    pressed: bool,
    tip: &str,
    draw: impl FnOnce(&Painter, Rect),
) -> bool {
    use crate::ui::tips::RichTip;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 24.0), egui::Sense::click());
    if pressed {
        ui.painter().rect_filled(rect, 2.0, Tokens::tool_active());
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
    }
    draw(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
    );
    resp.rich_tip(tip).clicked()
}

/// A tile (a square with a mark in its bottom-left corner) beside its
/// mirror image: `vertical` false puts the copy to the right, flipped
/// left to right; true puts it above, flipped top to bottom.
pub fn draw_mirror(p: &Painter, r: Rect, vertical: bool) {
    let edge = egui::Stroke::new(1.0, Tokens::icon());
    let (a, b) = if vertical {
        (
            Rect::from_min_max(
                egui::pos2(r.left() + 4.0, r.center().y),
                egui::pos2(r.right() - 4.0, r.bottom()),
            ),
            Rect::from_min_max(
                egui::pos2(r.left() + 4.0, r.top()),
                egui::pos2(r.right() - 4.0, r.center().y),
            ),
        )
    } else {
        (
            Rect::from_min_max(
                egui::pos2(r.left(), r.top() + 4.0),
                egui::pos2(r.center().x, r.bottom() - 4.0),
            ),
            Rect::from_min_max(
                egui::pos2(r.center().x, r.top() + 4.0),
                egui::pos2(r.right(), r.bottom() - 4.0),
            ),
        )
    };
    p.rect_stroke(a, 0.0, edge, egui::StrokeKind::Inside);
    p.rect_stroke(b, 0.0, edge, egui::StrokeKind::Inside);
    // The mark: a triangle in a corner, mirrored in the copy.
    let tri = |r: Rect, flip_x: bool, flip_y: bool| {
        let x0 = if flip_x {
            r.right() - 1.5
        } else {
            r.left() + 1.5
        };
        let x1 = if flip_x {
            r.left() + 1.5
        } else {
            r.right() - 1.5
        };
        let y0 = if flip_y {
            r.top() + 1.5
        } else {
            r.bottom() - 1.5
        };
        let y1 = if flip_y {
            r.bottom() - 1.5
        } else {
            r.top() + 1.5
        };
        p.add(egui::Shape::convex_polygon(
            vec![egui::pos2(x0, y0), egui::pos2(x1, y0), egui::pos2(x0, y1)],
            Tokens::icon(),
            egui::Stroke::NONE,
        ));
    };
    tri(a, false, false);
    tri(b, !vertical, vertical);
}

/// Bricks: `columns` false shows rows shifted against each other, true
/// columns shifted against each other.
pub fn draw_shift(p: &Painter, r: Rect, columns: bool) {
    let edge = egui::Stroke::new(1.0, Tokens::icon());
    let n: i32 = 3;
    let (w, h) = (r.width() / n as f32, r.height() / n as f32);
    for row in 0..n {
        for col in -1..=n {
            let (dx, dy) = if columns {
                (0.0, if col.rem_euclid(2) == 1 { h / 2.0 } else { 0.0 })
            } else {
                (if row % 2 == 1 { w / 2.0 } else { 0.0 }, 0.0)
            };
            let cell = Rect::from_min_size(
                egui::pos2(
                    r.left() + col as f32 * w + dx,
                    r.top() + row as f32 * h + dy,
                ),
                Vec2::new(w, h),
            )
            .intersect(r);
            if cell.width() > 0.5 && cell.height() > 0.5 {
                p.rect_stroke(cell, 0.0, edge, egui::StrokeKind::Inside);
            }
        }
    }
}

/// A padlock, closed when `locked`.
pub fn draw_lock(p: &Painter, r: Rect, locked: bool) {
    let edge = egui::Stroke::new(1.2, Tokens::icon());
    let body = Rect::from_min_max(
        egui::pos2(r.left() + 3.0, r.center().y),
        egui::pos2(r.right() - 3.0, r.bottom() - 1.0),
    );
    p.rect_filled(body, 1.0, Tokens::icon());
    // The shackle: an arc over the body, lifted and open when unlocked.
    let c = egui::pos2(
        if locked {
            r.center().x
        } else {
            r.center().x + 3.0
        },
        r.center().y - 3.0,
    );
    let rad = 4.0;
    let pts: Vec<egui::Pos2> = (0..=8)
        .map(|i| {
            let a = std::f32::consts::PI * (1.0 - i as f32 / 8.0);
            egui::pos2(c.x + rad * a.cos(), c.y - rad * a.sin())
        })
        .collect();
    p.add(egui::Shape::line(pts, edge));
    p.line_segment(
        [
            egui::pos2(c.x - rad, c.y),
            egui::pos2(c.x - rad, body.top()),
        ],
        edge,
    );
    if locked {
        p.line_segment(
            [
                egui::pos2(c.x + rad, c.y),
                egui::pos2(c.x + rad, body.top()),
            ],
            edge,
        );
    }
}

/// The two-colour, vector and bitmap pattern kind pictures of the fill
/// type row (kinds 3, 6 and 7 of `fill_kind_picture`).
pub fn draw_pattern_kind(p: &Painter, r: Rect, kind: usize) {
    let edge = egui::Stroke::new(1.0, Tokens::icon());
    let sq = r.shrink(1.0);
    match kind {
        // Vector pattern: a tile of small shapes, a circle and a triangle.
        6 => {
            p.rect_stroke(sq, 0.0, edge, egui::StrokeKind::Inside);
            let q = sq.width() / 4.0;
            for (cx, cy) in [(1.0, 1.0), (3.0, 3.0)] {
                p.circle_filled(sq.min + Vec2::new(cx * q, cy * q), q * 0.7, Tokens::icon());
            }
            for (cx, cy) in [(3.0, 1.0), (1.0, 3.0)] {
                let c = sq.min + Vec2::new(cx * q, cy * q);
                p.add(egui::Shape::convex_polygon(
                    vec![
                        egui::pos2(c.x, c.y - q * 0.8),
                        egui::pos2(c.x + q * 0.8, c.y + q * 0.6),
                        egui::pos2(c.x - q * 0.8, c.y + q * 0.6),
                    ],
                    Tokens::icon(),
                    egui::Stroke::NONE,
                ));
            }
        }
        // Bitmap pattern: a tile of pixels, a small picture.
        _ => {
            p.rect_stroke(sq, 0.0, edge, egui::StrokeKind::Inside);
            let n = 4;
            let w = sq.width() / n as f32;
            let dark = [
                (0, 1),
                (1, 0),
                (1, 2),
                (2, 1),
                (2, 3),
                (3, 2),
                (0, 3),
                (3, 0),
            ];
            for (x, y) in dark {
                let cell = Rect::from_min_size(
                    sq.min + Vec2::new(x as f32 * w, y as f32 * w),
                    Vec2::splat(w),
                );
                p.rect_filled(cell.shrink(0.5), 0.0, Tokens::icon());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Rect, Size};
    use tracedraw_core::{Color, Command, PatternTile};

    fn vector(tile: f64) -> Pattern {
        let mut s = Shape::new(
            tracedraw_core::ShapeId(70),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, tile / 2.0, tile / 2.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(255, 0, 0));
        Pattern::Vector {
            shapes: vec![s],
            tile: Size::new(tile, tile),
            placement: Default::default(),
        }
    }

    /// The library holds each vector and bitmap pattern of the drawing
    /// once, whatever its placement, the default fill included, and the
    /// pictures on the page as tiles to be; two-colour patterns are not
    /// listed (they are built in).
    #[test]
    fn library_lists_the_drawings_patterns_once() {
        let mut app = App::headless();
        let mut ids = Vec::new();
        for x in [10.0, 40.0, 70.0] {
            ids.push(
                app.new_shape(ShapeKind::Rect {
                    rect: Rect::new(x, 10.0, x + 20.0, 30.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("rect"),
            );
        }
        let turned = vector(10.0).with_placement(tracedraw_core::style::PatternPlacement {
            rotation: 30.0,
            ..Default::default()
        });
        for (id, fill) in [
            (ids[0], Fill::Pattern(vector(10.0))),
            (ids[1], Fill::Pattern(turned)),
            (
                ids[2],
                Fill::Pattern(Pattern::TwoColor {
                    tile: PatternTile::Dots,
                    front: Color::BLACK,
                    back: Color::WHITE,
                    size_mm: 5.0,
                    placement: Default::default(),
                }),
            ),
        ] {
            app.run(Command::SetFill {
                shapes: vec![id],
                fill,
            });
        }
        let lib = app.pattern_library();
        assert_eq!(lib.vector.len(), 1, "the same tile twice is one entry");
        assert!(lib.bitmap.is_empty() && lib.pictures.is_empty());
        assert_eq!(lib.position(&vector(10.0)), Some(0));
        // Another tile size is another pattern; a created pattern (the
        // default fill) is listed too.
        app.set_default_properties(tracedraw_core::DefaultKind::Graphic, |p| {
            p.fill = Fill::Pattern(vector(20.0))
        });
        let lib = app.pattern_library();
        assert_eq!(lib.vector.len(), 2);
        assert_eq!(lib.position(&vector(20.0)), Some(0), "the default first");
        // A picture on the page can become a bitmap tile.
        let mut pm = tiny_skia::Pixmap::new(2, 2).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(0, 0, 255, 255));
        let png = pm.encode_png().unwrap();
        app.new_shape(ShapeKind::Bitmap {
            rect: Rect::new(0.0, 0.0, 12.0, 12.0),
            width_px: 2,
            height_px: 2,
            png: png.into(),
            fx: None,
        })
        .expect("bitmap");
        let lib = app.pattern_library();
        assert_eq!(lib.pictures.len(), 1);
        let p = lib.picture_pattern(0).unwrap();
        assert!(
            matches!(p, Pattern::Bitmap { width_px: 2, height_px: 2, size_mm, .. } if size_mm == 12.0)
        );
        assert_eq!(
            pattern_name(&p, 0),
            tr("docker.bitmap_pattern_n")
                .replace("{i}", "1")
                .replace("{w}", "2")
                .replace("{h}", "2")
        );
    }
}
