//! Miniatures of the objects for the Objects docker: each object drawn
//! alone, with its fill, outline and effects, small enough to sit beside
//! its name (see `docs/behavior/objects-panel.md`).
//!
//! The pictures are cached by drawing and object. A cached picture is
//! looked at again only when the drawing's revision moved on, and drawn
//! again only when the object itself changed (it is compared with the copy
//! the picture was drawn from), when the size changed, or when a symbol
//! it shows changed. Only rows on screen ask for a picture, at most a few
//! milliseconds of new pictures are drawn per frame (the rest wait for the
//! next frames, showing the old picture or the swatch meanwhile), and the
//! least recently shown pictures are dropped once they hold more than
//! [`MAX_BYTES`] of pixels (or number more than [`MAX_ENTRIES`]), so a drawing of thousands of objects neither
//! lags nor fills the memory.

use std::collections::HashMap;
use tracedraw_core::geometry::Size;
use tracedraw_core::{Document, Shape, ShapeId, ShapeKind, Symbol};

/// The smallest and largest thumbnail, interface points, and the size a
/// new installation starts with.
pub const MIN_SIZE: f32 = 16.0;
pub const MAX_SIZE: f32 = 96.0;
pub const DEFAULT_SIZE: f32 = 20.0;
/// How much one wheel notch (or one step of the control) changes the size.
pub const STEP: f32 = 4.0;
/// Pixels kept in textures before the least recently shown are dropped.
pub const MAX_BYTES: usize = 24 * 1024 * 1024;
/// Pictures kept at most, whatever their size (each keeps a copy of its
/// object to compare with).
pub const MAX_ENTRIES: usize = 4000;
/// Time for drawing new pictures in one frame, milliseconds; one picture
/// is always drawn when one is waiting.
const BUDGET_MS: f64 = 6.0;

/// A thumbnail size clamped to the range and rounded to whole points.
pub fn clamp_size(s: f32) -> f32 {
    if s.is_finite() {
        s.round().clamp(MIN_SIZE, MAX_SIZE)
    } else {
        DEFAULT_SIZE
    }
}

struct Entry {
    /// The object as it was drawn, to tell whether it changed.
    shape: Shape,
    /// The symbols it shows, as they were.
    symbols: Vec<(usize, Symbol)>,
    /// The picture's long side, pixels.
    px: u32,
    /// The drawing's revision when the picture was last found current.
    revision: u64,
    /// The picture; `None` when the object draws nothing.
    texture: Option<egui::TextureHandle>,
    bytes: usize,
    /// The frame it was last shown in.
    used: u64,
}

/// The cache of object thumbnails (one per application).
#[derive(Default)]
pub struct ObjectThumbs {
    entries: HashMap<(u64, ShapeId), Entry>,
    bytes: usize,
    frame: u64,
    /// Pictures drawn this frame, and when the first was started.
    drawn_now: usize,
    started: Option<web_time::Instant>,
    /// A row wanted a picture this frame that waits for the next one.
    pub waiting: bool,
    /// Pictures drawn since the start (for the tests and benchmarks).
    pub renders: u64,
}

/// What a row draws for its object.
pub enum Thumb {
    /// The picture, its size in pixels; it may be an older picture while
    /// the new one waits.
    Picture(egui::TextureId, [u32; 2]),
    /// The object draws nothing (an empty curve, an empty group).
    Empty,
    /// No picture yet: the row shows the swatch for now.
    Pending,
}

impl ObjectThumbs {
    /// Start a frame: the drawing budget starts again.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        self.drawn_now = 0;
        self.started = None;
        self.waiting = false;
    }

    /// Pixels held in textures.
    #[cfg(test)]
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Pictures held.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the time for new pictures this frame is used up.
    fn over_budget(&self) -> bool {
        if self.drawn_now == 0 {
            return false;
        }
        self.started
            .is_some_and(|t| t.elapsed().as_secs_f64() * 1000.0 > BUDGET_MS)
    }

    /// The picture of `shape` (an object of `doc`, the drawing `instance`
    /// at `revision`), `px` pixels on its long side.
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        doc: &Document,
        (instance, revision): (u64, u64),
        shape: &Shape,
        px: u32,
    ) -> Thumb {
        let key = (instance, shape.id);
        let frame = self.frame;
        if let Some(e) = self.entries.get_mut(&key) {
            e.used = frame;
            let current = e.px == px
                && (e.revision == revision || (e.shape == *shape && symbols_same(&e.symbols, doc)));
            if current {
                e.revision = revision;
                return shown(e);
            }
        }
        if self.over_budget() {
            self.waiting = true;
            return self.entries.get(&key).map(shown).unwrap_or(Thumb::Pending);
        }
        if self.drawn_now == 0 {
            self.started = Some(web_time::Instant::now());
        }
        self.drawn_now += 1;
        self.renders += 1;
        let texture = render(doc, shape, px).map(|(rgba, w, h)| {
            let image = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], &rgba);
            ctx.load_texture(
                format!("object-thumb:{instance}:{}", shape.id.raw()),
                image,
                egui::TextureOptions::LINEAR,
            )
        });
        let bytes = texture
            .as_ref()
            .map(|t| t.size()[0] * t.size()[1] * 4)
            .unwrap_or(0);
        let entry = Entry {
            shape: shape.clone(),
            symbols: symbols_of(shape, doc),
            px,
            revision,
            texture,
            bytes,
            used: frame,
        };
        if let Some(old) = self.entries.insert(key, entry) {
            self.bytes -= old.bytes;
        }
        self.bytes += bytes;
        self.entries.get(&key).map(shown).unwrap_or(Thumb::Pending)
    }

    /// Drop the least recently shown pictures while they hold more than
    /// `max_bytes` or number more than [`MAX_ENTRIES`]; the pictures shown
    /// this frame stay.
    pub fn trim(&mut self, max_bytes: usize) {
        let over = |t: &Self| t.bytes > max_bytes || t.entries.len() > MAX_ENTRIES;
        if !over(self) {
            return;
        }
        let mut order: Vec<((u64, ShapeId), u64)> =
            self.entries.iter().map(|(k, e)| (*k, e.used)).collect();
        order.sort_by_key(|(_, used)| *used);
        for (key, used) in order {
            if !over(self) || used == self.frame {
                break;
            }
            if let Some(e) = self.entries.remove(&key) {
                self.bytes -= e.bytes;
            }
        }
    }
}

fn shown(e: &Entry) -> Thumb {
    match &e.texture {
        Some(t) => {
            let [w, h] = t.size();
            Thumb::Picture(t.id(), [w as u32, h as u32])
        }
        None => Thumb::Empty,
    }
}

/// The symbols `shape` shows (its instances and those in its groups),
/// by index.
fn symbol_indices(shape: &Shape, out: &mut Vec<usize>) {
    match &shape.kind {
        ShapeKind::SymbolInstance { index } => {
            if !out.contains(index) {
                out.push(*index);
            }
        }
        ShapeKind::Group { children } => {
            for c in children {
                symbol_indices(c, out);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            symbol_indices(frame, out);
            for c in contents {
                symbol_indices(c, out);
            }
        }
        _ => {}
    }
}

fn symbols_of(shape: &Shape, doc: &Document) -> Vec<(usize, Symbol)> {
    let mut idx = Vec::new();
    symbol_indices(shape, &mut idx);
    idx.into_iter()
        .filter_map(|i| doc.symbols.get(i).map(|s| (i, s.clone())))
        .collect()
}

fn symbols_same(kept: &[(usize, Symbol)], doc: &Document) -> bool {
    kept.iter().all(|(i, s)| doc.symbols.get(*i) == Some(s))
}

/// `shape` drawn alone, at most `px` pixels on its long side, on a
/// transparent background: premultiplied RGBA and its size. `None` when
/// it draws nothing.
pub fn render(doc: &Document, shape: &Shape, px: u32) -> Option<(Vec<u8>, u32, u32)> {
    let b = shape.visual_bounds();
    let px = f64::from(px.clamp(4, 1024));
    let stroke = shape.stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
    let stroke = if stroke.is_finite() {
        stroke.max(0.0)
    } else {
        0.0
    };
    // A margin so that outlines on the edge show whole, and a smallest
    // side so that a line still shows.
    let pad = (b.width().max(b.height()) * 0.03)
        .max(stroke * 0.5)
        .max(0.05);
    let mut b = b.inflate(pad, pad);
    let longest = b.width().max(b.height());
    if !(longest.is_finite() && longest > 0.0) {
        return None;
    }
    let least = longest / 8.0;
    if b.width() < least {
        b = b.inflate((least - b.width()) / 2.0, 0.0);
    }
    if b.height() < least {
        b = b.inflate(0.0, (least - b.height()) / 2.0);
    }
    let zoom = px / longest;
    let w = (b.width() * zoom).ceil().clamp(1.0, px) as u32;
    let h = (b.height() * zoom).ceil().clamp(1.0, px) as u32;
    let page_size = doc
        .pages
        .first()
        .map(|p| p.size)
        .unwrap_or(Size::new(210.0, 297.0));
    let mut tmp = Document::new("thumb", page_size);
    let page = tmp.pages.first()?.id;
    let layer = tmp.pages.first()?.layers.first()?.id;
    let mut idx = Vec::new();
    symbol_indices(shape, &mut idx);
    if !idx.is_empty() {
        tmp.symbols = doc.symbols.clone();
    }
    let mut c = shape.clone();
    // A hidden object still shows what it is; the row dims it.
    c.visible = true;
    tmp.layer_mut(layer).ok()?.shapes.push(c);
    let pm = tracedraw_render::render_page(
        &tmp,
        page,
        &tracedraw_render::RenderOptions {
            width: w,
            height: h,
            view: tracedraw_render::ViewTransform {
                zoom,
                origin_x: -b.x0 * zoom,
                origin_y: b.y1 * zoom,
            },
            preview: None,
            wireframe: false,
            anti_alias: true,
            draft: true,
            focus: None,
            ..tracedraw_render::RenderOptions::default()
        },
    )?;
    let data = pm.data();
    if data.chunks_exact(4).all(|p| p[3] == 0) {
        return None;
    }
    Some((data.to_vec(), w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::{Color, Fill};

    fn square(x: f64, color: Color) -> Shape {
        let mut s = Shape::new(
            ShapeId(1000 + x as u64),
            ShapeKind::Rect {
                rect: Rect::new(x, 10.0, x + 40.0, 30.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(color);
        s.stroke = None;
        s
    }

    #[test]
    fn a_thumbnail_shows_the_object_in_its_colours() {
        let doc = Document::new("t", Size::new(210.0, 297.0));
        let s = square(10.0, Color::rgb8(255, 0, 0));
        let (rgba, w, h) = render(&doc, &s, 40).expect("a picture");
        // Twice as wide as tall: the long side is 40 pixels.
        assert_eq!(w, 40);
        assert!((19..=22).contains(&h), "{w}x{h}");
        let mid = ((h / 2 * w + w / 2) * 4) as usize;
        assert!(rgba[mid] > 200 && rgba[mid + 1] < 40 && rgba[mid + 3] == 255);
        // Hidden objects are drawn too.
        let mut hidden = s.clone();
        hidden.visible = false;
        assert!(render(&doc, &hidden, 40).is_some());
        // An empty group draws nothing.
        let mut g = s;
        g.kind = ShapeKind::Group { children: vec![] };
        assert!(render(&doc, &g, 40).is_none());
    }

    #[test]
    fn pictures_are_drawn_again_only_when_the_object_changes() {
        let ctx = egui::Context::default();
        let doc = Document::new("t", Size::new(210.0, 297.0));
        let mut thumbs = ObjectThumbs::default();
        let a = square(10.0, Color::rgb8(255, 0, 0));
        let b = square(60.0, Color::rgb8(0, 0, 255));
        thumbs.begin_frame();
        assert!(matches!(
            thumbs.get(&ctx, &doc, (1, 1), &a, 32),
            Thumb::Picture(..)
        ));
        assert!(matches!(
            thumbs.get(&ctx, &doc, (1, 1), &b, 32),
            Thumb::Picture(..)
        ));
        assert_eq!(thumbs.renders, 2);
        // The same revision: nothing is looked at or drawn.
        thumbs.begin_frame();
        thumbs.get(&ctx, &doc, (1, 1), &a, 32);
        thumbs.get(&ctx, &doc, (1, 1), &b, 32);
        assert_eq!(thumbs.renders, 2);
        // A new revision where only `b` changed: only `b` is drawn again.
        let mut b2 = b.clone();
        b2.fill = Fill::Solid(Color::rgb8(0, 255, 0));
        thumbs.begin_frame();
        thumbs.get(&ctx, &doc, (1, 2), &a, 32);
        thumbs.get(&ctx, &doc, (1, 2), &b2, 32);
        assert_eq!(thumbs.renders, 3);
        // Another size draws again; another drawing has its own pictures.
        thumbs.begin_frame();
        thumbs.get(&ctx, &doc, (1, 2), &a, 48);
        thumbs.get(&ctx, &doc, (2, 1), &a, 48);
        assert_eq!(thumbs.renders, 5);
        assert_eq!(thumbs.len(), 3);
    }

    #[test]
    fn the_least_recently_shown_pictures_go_first() {
        let ctx = egui::Context::default();
        let doc = Document::new("t", Size::new(210.0, 297.0));
        let mut thumbs = ObjectThumbs::default();
        let shapes: Vec<Shape> = (0..6)
            .map(|i| square(f64::from(i) * 50.0, Color::rgb8(0, 0, 0)))
            .collect();
        for s in &shapes {
            thumbs.begin_frame();
            thumbs.get(&ctx, &doc, (1, 1), s, 32);
        }
        let one = thumbs.bytes() / 6;
        assert!(one > 0);
        thumbs.trim(one * 3);
        assert!(thumbs.bytes() <= one * 3);
        assert_eq!(thumbs.len(), 3);
        // The newest stay: showing them again draws nothing.
        let before = thumbs.renders;
        thumbs.begin_frame();
        for s in &shapes[3..] {
            thumbs.get(&ctx, &doc, (1, 1), s, 32);
        }
        assert_eq!(thumbs.renders, before);
        // Those shown in this frame are never dropped.
        thumbs.trim(0);
        assert_eq!(thumbs.len(), 3);
    }

    #[test]
    fn sizes_are_clamped() {
        assert_eq!(clamp_size(2.0), MIN_SIZE);
        assert_eq!(clamp_size(500.0), MAX_SIZE);
        assert_eq!(clamp_size(f32::NAN), DEFAULT_SIZE);
        assert_eq!(clamp_size(30.4), 30.0);
    }
}
