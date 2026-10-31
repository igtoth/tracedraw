//! The Shelf (Window > Dockers > Shelf): objects kept apart from any
//! drawing, to place in this drawing or another one, now or after
//! TraceDraw restarts. Each item holds copies of the objects added
//! together, with the object styles they link to (a placed object keeps
//! its link only where the drawing has the same style) and every symbol
//! instance turned into the symbol's objects, so that an item never
//! depends on the drawing it came from. An item can hold colours
//! instead: the document palette and colour styles of a drawing, which
//! placing adds to another drawing's.
//!
//! The shelf is kept in the preferences folder as `shelf.json`
//! (`{"kind": "tracedraw-shelf", "version": 1, "items": [...]}`), in local
//! storage in a browser. A missing, damaged or foreign file gives an
//! empty shelf; items beyond the limits are left out.

use crate::app::App;
use crate::i18n::{tr, trf};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracedraw_core::document::Symbol;
use tracedraw_core::geometry::{Affine, Point, Rect, Size, Vec2};
use tracedraw_core::{Command, ObjectStyle, Shape, ShapeKind};

pub const KIND: &str = "tracedraw-shelf";
pub const VERSION: u32 = 1;
/// The most items the shelf keeps.
pub const MAX_ITEMS: usize = 500;
/// Larger shelf files are not read.
const MAX_FILE: u64 = 64 * 1024 * 1024;
/// Item names are cut to this many characters.
const MAX_NAME: usize = 120;
/// Symbols inside symbols are followed this deep.
const MAX_DEPTH: usize = 16;
/// The browser build's key: in the page's kept database (`web::kept`),
/// and in local storage, where versions before #1414 kept the shelf.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const WEB_KEY: &str = "tracedraw-shelf";

/// The shelf read from the browser's storage with the items added before
/// it could be read after it (at most [`MAX_ITEMS`]).
#[cfg_attr(not(any(target_arch = "wasm32", test)), allow(dead_code))]
fn merged(stored: Vec<ShelfItem>, added: Vec<ShelfItem>) -> Vec<ShelfItem> {
    let mut items = stored;
    items.extend(added);
    items.truncate(MAX_ITEMS);
    items
}

/// One item on the shelf.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShelfItem {
    pub name: String,
    /// The objects, in page millimetres as they were in their drawing.
    pub shapes: Vec<Shape>,
    /// The object styles the objects link to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<ObjectStyle>,
    /// A colour item: document palette colours and colour styles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<tracedraw_core::Color>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub color_styles: Vec<tracedraw_core::document::ColorStyle>,
    /// Tells items apart while TraceDraw runs (thumbnails, selection).
    #[serde(skip)]
    pub uid: u64,
}

impl ShelfItem {
    /// An item of colours (no objects).
    pub fn is_colors(&self) -> bool {
        self.shapes.is_empty() && !(self.colors.is_empty() && self.color_styles.is_empty())
    }

    /// Every colour the item holds: palette colours, then colour styles.
    pub fn all_colors(&self) -> Vec<tracedraw_core::Color> {
        self.colors
            .iter()
            .copied()
            .chain(self.color_styles.iter().map(|c| c.color))
            .collect()
    }

    /// The bounds of the item's objects, when they have finite ones.
    pub fn bounds(&self) -> Option<Rect> {
        self.shapes
            .iter()
            .map(Shape::bounds)
            .reduce(|a, b| a.union(b))
            .filter(|r| {
                [r.x0, r.y0, r.x1, r.y1].iter().all(|v| v.is_finite())
                    && r.width() >= 0.0
                    && r.height() >= 0.0
            })
    }
}

/// The shelf file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct ShelfFile {
    kind: String,
    version: u32,
    items: Vec<ShelfItem>,
}

impl Default for ShelfFile {
    fn default() -> Self {
        ShelfFile {
            kind: KIND.to_string(),
            version: VERSION,
            items: Vec::new(),
        }
    }
}

/// The shelf's items and the docker's state.
#[derive(Default)]
pub struct Shelf {
    pub items: Vec<ShelfItem>,
    /// Read from the preferences folder (it is read when first needed).
    loaded: bool,
    next_uid: u64,
    /// The item chosen in the docker.
    pub selected: Option<u64>,
    /// The item being renamed and the name typed so far.
    pub renaming: Option<(u64, String)>,
    /// The item being dragged onto the drawing.
    pub dragging: Option<u64>,
    /// Item thumbnails and the pixel size they were drawn at.
    pub thumbs: HashMap<u64, (u32, egui::TextureHandle)>,
}

/// Where the shelf is kept (desktop).
fn path() -> Option<PathBuf> {
    crate::settings::config_dir().map(|d| d.join("shelf.json"))
}

/// The item's name, cut to `MAX_NAME` characters, on one line.
fn clean_name(name: &str) -> String {
    let one_line: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(MAX_NAME)
        .collect();
    one_line.trim().to_string()
}

/// Read a shelf file's bytes: its items, or nothing when the bytes are
/// not a shelf. Items without objects are left out, names cleaned, and
/// at most `MAX_ITEMS` kept.
pub fn items_from_bytes(bytes: &[u8]) -> Option<Vec<ShelfItem>> {
    let file: ShelfFile = serde_json::from_slice(bytes).ok()?;
    if file.kind != KIND {
        return None;
    }
    if file.version > VERSION {
        log::warn!(
            "the shelf was written by a newer TraceDraw (version {}); reading what this one knows",
            file.version
        );
    }
    Some(
        file.items
            .into_iter()
            .filter(|i| !i.shapes.is_empty() || i.is_colors())
            .map(|mut i| {
                i.name = clean_name(&i.name);
                i
            })
            .take(MAX_ITEMS)
            .collect(),
    )
}

/// A shelf file holding `items`.
pub fn items_to_bytes(items: &[ShelfItem]) -> Option<Vec<u8>> {
    let file = ShelfFile {
        items: items.to_vec(),
        ..ShelfFile::default()
    };
    serde_json::to_vec(&file).ok()
}

impl Shelf {
    /// Read the shelf when it has not been read yet.
    pub fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        // Unit tests never read the preferences folder they share.
        if cfg!(test) {
            self.loaded = true;
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            // The page's database is read in the background: until then
            // the shelf waits, and items added meanwhile stay and are
            // saved with it (#1414).
            if !crate::web::kept::ready() {
                return;
            }
            self.loaded = true;
            let stored = crate::web::kept::get(WEB_KEY)
                .or_else(|| crate::web::storage_get(WEB_KEY).map(String::into_bytes))
                .and_then(|bytes| items_from_bytes(&bytes))
                .unwrap_or_default();
            let added = std::mem::take(&mut self.items);
            let save = !added.is_empty();
            self.set_items(merged(stored, added));
            if save {
                self.save();
            }
            return;
        }
        #[allow(unreachable_code)]
        {
            self.loaded = true;
            if let Some(p) = path() {
                self.load_from(&p);
            }
        }
    }

    /// Replace the items with those of the shelf file at `p` (none when
    /// it is missing or unreadable).
    pub fn load_from(&mut self, p: &Path) {
        self.loaded = true;
        let too_big = std::fs::metadata(p).is_ok_and(|m| m.len() > MAX_FILE);
        let items = if too_big {
            log::warn!("the shelf file {} is too large to read", p.display());
            Vec::new()
        } else {
            match std::fs::read(p) {
                Ok(bytes) => items_from_bytes(&bytes).unwrap_or_else(|| {
                    log::warn!(
                        "{} is not a shelf file; the shelf starts empty",
                        p.display()
                    );
                    Vec::new()
                }),
                Err(_) => Vec::new(),
            }
        };
        self.set_items(items);
    }

    fn set_items(&mut self, items: Vec<ShelfItem>) {
        self.items = items;
        for i in 0..self.items.len() {
            let uid = self.new_uid();
            if let Some(item) = self.items.get_mut(i) {
                item.uid = uid;
            }
        }
        self.selected = None;
        self.renaming = None;
        self.dragging = None;
        self.thumbs.clear();
    }

    fn new_uid(&mut self) -> u64 {
        self.next_uid += 1;
        self.next_uid
    }

    /// Keep the shelf (unit tests keep nothing, see `ensure_loaded`).
    pub fn save(&self) {
        if cfg!(test) {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Not read yet: kept in memory, saved with the stored shelf.
            if !self.loaded {
                return;
            }
            let Some(bytes) = items_to_bytes(&self.items) else {
                return;
            };
            // In the page's database (#1414); the copy an older version
            // left in local storage goes once the database holds it.
            let done: Box<dyn FnOnce()> = Box::new(|| crate::web::storage_remove(WEB_KEY));
            let kept = crate::storage_full::Kept::Shelf;
            if !crate::web::kept::put(WEB_KEY, &bytes, kept, Some(done)) {
                // No IndexedDB in this browser: local storage.
                if let Err(full) =
                    crate::web::storage_set(WEB_KEY, &String::from_utf8_lossy(&bytes))
                {
                    crate::storage_full::report(kept, full);
                }
            }
            return;
        }
        #[allow(unreachable_code)]
        if let Some(p) = path() {
            if let Err(e) = self.save_to(&p) {
                log::warn!("could not keep the shelf in {}: {e}", p.display());
            }
        }
    }

    /// Write the shelf file to `p`, through a temporary file so that a
    /// failed write leaves the previous shelf as it was.
    pub fn save_to(&self, p: &Path) -> std::io::Result<()> {
        let bytes = items_to_bytes(&self.items)
            .ok_or_else(|| std::io::Error::other("the shelf could not be written"))?;
        let tmp = p.with_extension("json.tmp");
        crate::files::write_creating(&tmp, bytes)?;
        std::fs::rename(&tmp, p)
    }

    pub fn index_of(&self, uid: u64) -> Option<usize> {
        self.items.iter().position(|i| i.uid == uid)
    }

    pub fn item(&self, uid: u64) -> Option<&ShelfItem> {
        self.items.iter().find(|i| i.uid == uid)
    }

    /// Add an item at the end; its uid, or `None` when the shelf is full
    /// or the item has no objects.
    pub fn push(&mut self, mut item: ShelfItem) -> Option<u64> {
        if self.items.len() >= MAX_ITEMS || (item.shapes.is_empty() && !item.is_colors()) {
            return None;
        }
        item.uid = self.new_uid();
        item.name = clean_name(&item.name);
        let uid = item.uid;
        self.items.push(item);
        Some(uid)
    }

    /// Take the item off the shelf.
    pub fn remove(&mut self, uid: u64) -> Option<ShelfItem> {
        let i = self.index_of(uid)?;
        self.thumbs.remove(&uid);
        if self.selected == Some(uid) {
            self.selected = None;
        }
        if self.renaming.as_ref().is_some_and(|(u, _)| *u == uid) {
            self.renaming = None;
        }
        Some(self.items.remove(i))
    }

    /// Give the item a new name; empty names are refused.
    pub fn rename(&mut self, uid: u64, name: &str) -> bool {
        let name = clean_name(name);
        match self.items.iter_mut().find(|i| i.uid == uid) {
            Some(item) if !name.is_empty() && item.name != name => {
                item.name = name;
                true
            }
            _ => false,
        }
    }

    /// Move the item to position `to` (clamped to the list).
    pub fn move_to(&mut self, uid: u64, to: usize) -> bool {
        let Some(from) = self.index_of(uid) else {
            return false;
        };
        let to = to.min(self.items.len().saturating_sub(1));
        if from == to {
            return false;
        }
        let item = self.items.remove(from);
        self.items.insert(to, item);
        true
    }
}

/// `shape` with every symbol instance in it (inside groups and clip
/// frames too) replaced by the symbol's objects. Instances of missing
/// symbols, symbols nested deeper than `MAX_DEPTH` and groups left empty
/// are dropped.
fn without_symbols(mut shape: Shape, symbols: &[Symbol], depth: usize) -> Option<Shape> {
    match &mut shape.kind {
        ShapeKind::SymbolInstance { index } => {
            if depth >= MAX_DEPTH {
                return None;
            }
            let def = symbols.get(*index)?;
            let mut copy = without_symbols(def.shape.clone(), symbols, depth + 1)?;
            copy.absorb(shape.transform);
            Some(copy)
        }
        ShapeKind::Group { children } => {
            let kept: Vec<Shape> = std::mem::take(children)
                .into_iter()
                .filter_map(|c| without_symbols(c, symbols, depth + 1))
                .collect();
            if kept.is_empty() {
                return None;
            }
            *children = kept;
            Some(shape)
        }
        ShapeKind::ClipFrame { frame, contents } => {
            let f = without_symbols((**frame).clone(), symbols, depth + 1)?;
            **frame = f;
            let kept: Vec<Shape> = std::mem::take(contents)
                .into_iter()
                .filter_map(|c| without_symbols(c, symbols, depth + 1))
                .collect();
            *contents = kept;
            Some(shape)
        }
        _ => Some(shape),
    }
}

/// The object styles `shape` and the objects inside it link to.
fn style_ids(shape: &Shape, out: &mut Vec<u32>) {
    if let Some(id) = shape.style {
        if !out.contains(&id) {
            out.push(id);
        }
    }
    match &shape.kind {
        ShapeKind::Group { children } => children.iter().for_each(|c| style_ids(c, out)),
        ShapeKind::ClipFrame { frame, contents } => {
            style_ids(frame, out);
            contents.iter().for_each(|c| style_ids(c, out));
        }
        _ => {}
    }
}

/// A shelf item made of `shapes` from a drawing with `symbols` and
/// `styles`, named `name`.
pub fn make_item(
    name: &str,
    shapes: Vec<Shape>,
    symbols: &[Symbol],
    styles: &[ObjectStyle],
) -> ShelfItem {
    let shapes: Vec<Shape> = shapes
        .into_iter()
        .filter_map(|s| without_symbols(s, symbols, 0))
        .collect();
    let mut ids = Vec::new();
    shapes.iter().for_each(|s| style_ids(s, &mut ids));
    let styles = styles
        .iter()
        .filter(|s| ids.contains(&s.id))
        .cloned()
        .collect();
    ShelfItem {
        name: clean_name(name),
        shapes,
        styles,
        colors: Vec::new(),
        color_styles: Vec::new(),
        uid: 0,
    }
}

/// A colour item holding a drawing's palette and colour styles.
pub fn make_color_item(
    name: &str,
    colors: &[tracedraw_core::Color],
    styles: &[tracedraw_core::document::ColorStyle],
) -> ShelfItem {
    ShelfItem {
        name: clean_name(name),
        shapes: Vec::new(),
        styles: Vec::new(),
        colors: colors.to_vec(),
        color_styles: styles.to_vec(),
        uid: 0,
    }
}

/// A colour item's picture: its colours as squares (at most 64, in rows
/// of 8), premultiplied RGBA.
fn color_thumbnail(colors: &[tracedraw_core::Color], px: u32) -> Option<(Vec<u8>, u32, u32)> {
    if colors.is_empty() {
        return None;
    }
    let n = colors.len().min(64);
    let cols = n.min(8);
    let rows = n.div_ceil(8);
    let cell = (px.clamp(8, 512) as usize / cols).max(2);
    let (w, h) = (cell * cols, cell * rows);
    let mut out = vec![0u8; w * h * 4];
    for (i, c) in colors.iter().take(n).enumerate() {
        let [r, g, b] = c.to_rgb8();
        let (cx, cy) = (i % cols * cell, i / cols * cell);
        for y in cy..cy + cell {
            for x in cx..cx + cell {
                // A one pixel gap between squares.
                if x == cx + cell - 1 || y == cy + cell - 1 {
                    continue;
                }
                let o = (y * w + x) * 4;
                out[o..o + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
    }
    Some((out, w as u32, h as u32))
}

/// A picture of the item, `px` pixels on its long side at most, on a
/// transparent background (premultiplied RGBA), with its pixel size.
pub fn thumbnail(item: &ShelfItem, px: u32) -> Option<(Vec<u8>, u32, u32)> {
    if item.is_colors() {
        return color_thumbnail(&item.all_colors(), px);
    }
    let b = item.bounds()?;
    let px = px.clamp(8, 512) as f64;
    // A margin so that outlines on the edge show whole.
    let pad = (b.width().max(b.height()) * 0.04).max(0.2);
    let b = b.inflate(pad, pad);
    let longest = b.width().max(b.height());
    if !(longest.is_finite() && longest > 0.0) {
        return None;
    }
    let zoom = px / longest;
    let w = (b.width() * zoom).ceil().clamp(1.0, px) as u32;
    let h = (b.height() * zoom).ceil().clamp(1.0, px) as u32;
    let mut tmp = tracedraw_core::Document::new("shelf", Size::new(b.width(), b.height()));
    let layer = tmp.pages.first()?.layers.first()?.id;
    let page = tmp.pages.first()?.id;
    for s in &item.shapes {
        let mut c = s.clone();
        c.transform_by(Affine::translate((-b.x0, -b.y0)));
        if let Ok(l) = tmp.layer_mut(layer) {
            l.shapes.push(c);
        }
    }
    let pm = tracedraw_render::render_page(
        &tmp,
        page,
        &tracedraw_render::RenderOptions {
            width: w,
            height: h,
            view: tracedraw_render::ViewTransform {
                zoom,
                origin_x: 0.0,
                origin_y: h as f64,
            },
            preview: None,
            wireframe: false,
            anti_alias: true,
            draft: true,
            focus: None,
            ..tracedraw_render::RenderOptions::default()
        },
    )?;
    Some((pm.data().to_vec(), w, h))
}

impl App {
    /// Copies of the selected objects as a new item at the end of the
    /// shelf, which becomes the chosen one; false when nothing is
    /// selected or the shelf is full.
    pub fn shelf_add_selection(&mut self) -> bool {
        self.shelf.ensure_loaded();
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return false;
        }
        let name = match shapes.as_slice() {
            [one] => one
                .name
                .clone()
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| crate::ui::dockers::kind_name(&one.kind)),
            many => trf("shelf.objects_n", &[("n", &many.len().to_string())]),
        };
        let doc = self.doc();
        let item = make_item(&name, shapes, &doc.symbols, &doc.object_styles);
        let name = item.name.clone();
        match self.shelf.push(item) {
            Some(uid) => {
                self.shelf.selected = Some(uid);
                self.shelf.save();
                self.status = trf("shelf.added", &[("name", &name)]);
                true
            }
            None => {
                self.status = if self.shelf.items.len() >= MAX_ITEMS {
                    trf("shelf.full", &[("n", &MAX_ITEMS.to_string())])
                } else {
                    tr("shelf.nothing_to_add")
                };
                false
            }
        }
    }

    /// Place copies of the item's objects on the active layer, centred
    /// on `at` (page millimetres) or on the middle of the view, selected,
    /// as one undo step; false when there is nothing to place or no layer
    /// to place on.
    pub fn shelf_place(&mut self, uid: u64, at: Option<Point>) -> bool {
        let Some(item) = self.shelf.item(uid).cloned() else {
            return false;
        };
        if item.is_colors() {
            return self.shelf_place_colors(&item);
        }
        let Some(bounds) = item.bounds() else {
            return false;
        };
        // The Welcome Screen shows no drawing to place on.
        if !self.has_document() || self.show_welcome {
            self.status = tr("shelf.no_drawing");
            return false;
        }
        let Some(layer) = self.active_layer() else {
            self.status = tr("shelf.no_layer");
            return false;
        };
        let target = at.unwrap_or_else(|| self.view.visible_page_rect(self.canvas_rect).center());
        let c = bounds.center();
        let d = Vec2::new(target.x - c.x, target.y - c.y);
        let d = if d.x.is_finite() && d.y.is_finite() {
            d
        } else {
            Vec2::ZERO
        };
        let here = self.doc().object_styles.clone();
        let mut ids = Vec::new();
        let mut cmds = Vec::new();
        for s in item.shapes {
            let id = self.engine.new_shape_id();
            let mut shape = crate::app::reid_pub(s, id, &mut self.engine);
            crate::styles::keep_links_to(&mut shape, &item.styles, &here);
            shape.transform_by(Affine::translate(d));
            ids.push(id);
            cmds.push(Command::AddShape { layer, shape });
        }
        if let Err(e) = self.engine.run_batch("Place from Shelf", &cmds) {
            self.status = e.to_string();
            return false;
        }
        self.select(ids);
        self.status = trf("shelf.placed", &[("name", &item.name)]);
        true
    }

    /// The drawing's document palette and colour styles as a new colour
    /// item; false when it has none or the shelf is full.
    pub fn shelf_add_colors(&mut self) -> bool {
        self.shelf.ensure_loaded();
        let doc = self.doc();
        if doc.palette.is_empty() && doc.color_styles.is_empty() {
            self.status = tr("shelf.no_colors");
            return false;
        }
        let n = doc.palette.len() + doc.color_styles.len();
        let name = trf("shelf.colors_n", &[("n", &n.to_string())]);
        let item = make_color_item(&name, &doc.palette, &doc.color_styles);
        let name = item.name.clone();
        match self.shelf.push(item) {
            Some(uid) => {
                self.shelf.selected = Some(uid);
                self.shelf.save();
                self.status = trf("shelf.added", &[("name", &name)]);
                true
            }
            None => {
                self.status = trf("shelf.full", &[("n", &MAX_ITEMS.to_string())]);
                false
            }
        }
    }

    /// A colour item placed: its colours join the document palette and
    /// its colour styles the drawing's (those it has by name are left as
    /// they are), as one undo step.
    fn shelf_place_colors(&mut self, item: &ShelfItem) -> bool {
        if !self.has_document() || self.show_welcome {
            self.status = tr("shelf.no_drawing");
            return false;
        }
        let mut palette = self.doc().palette.clone();
        for c in &item.colors {
            if !palette.contains(c) {
                palette.push(*c);
            }
        }
        let mut styles = self.doc().color_styles.clone();
        for c in &item.color_styles {
            if !styles.iter().any(|s| s.name == c.name) {
                styles.push(c.clone());
            }
        }
        let cmds = vec![
            Command::SetDocumentPalette { colors: palette },
            Command::SetColorStyles { styles },
        ];
        if let Err(e) = self.engine.run_batch("Place from Shelf", &cmds) {
            self.status = e.to_string();
            return false;
        }
        self.status = trf("shelf.placed", &[("name", &item.name)]);
        true
    }

    /// Take the item off the shelf.
    pub fn shelf_remove(&mut self, uid: u64) -> bool {
        let removed = self.shelf.remove(uid).is_some();
        if removed {
            self.shelf.save();
        }
        removed
    }

    /// Rename the item.
    pub fn shelf_rename(&mut self, uid: u64, name: &str) -> bool {
        let renamed = self.shelf.rename(uid, name);
        if renamed {
            self.shelf.save();
        }
        renamed
    }

    /// Move the item to position `to` in the shelf.
    pub fn shelf_move(&mut self, uid: u64, to: usize) -> bool {
        let moved = self.shelf.move_to(uid, to);
        if moved {
            self.shelf.save();
        }
        moved
    }

    /// The item's thumbnail texture, `px` pixels on its long side, drawn
    /// once (again when `px` changes).
    pub fn shelf_thumbnail(
        &mut self,
        ctx: &egui::Context,
        uid: u64,
        px: u32,
    ) -> Option<egui::TextureHandle> {
        if let Some((size, t)) = self.shelf.thumbs.get(&uid) {
            if *size == px {
                return Some(t.clone());
            }
        }
        let item = self.shelf.item(uid)?;
        let (rgba, w, h) = thumbnail(item, px)?;
        let image =
            egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], rgba.as_slice());
        let t = ctx.load_texture(format!("shelf:{uid}"), image, egui::TextureOptions::LINEAR);
        self.shelf.thumbs.insert(uid, (px, t.clone()));
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Color, Fill, ShapeId};

    fn rect(app: &mut App, x: f64, y: f64, w: f64, h: f64) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(x, y, x + w, y + h),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(Fill::Solid(Color::rgb8(200, 30, 30)));
        id
    }

    fn shape_count(app: &App) -> usize {
        app.doc()
            .all_layers()
            .map(|l| l.shapes.len())
            .sum::<usize>()
    }

    #[test]
    fn selections_go_on_the_shelf_and_come_back_anywhere() {
        let mut app = App::headless();
        let a = rect(&mut app, 10.0, 10.0, 20.0, 10.0);
        let b = rect(&mut app, 40.0, 10.0, 10.0, 10.0);
        // Nothing selected: nothing to add.
        app.select(vec![]);
        assert!(!app.shelf_add_selection());
        app.select(vec![a, b]);
        assert!(app.shelf_add_selection());
        let uid = app.shelf.selected.expect("chosen");
        let item = app.shelf.item(uid).expect("item").clone();
        assert_eq!(item.shapes.len(), 2);
        assert_eq!(item.name, trf("shelf.objects_n", &[("n", "2")]));
        // One object takes its kind's name.
        app.select(vec![b]);
        assert!(app.shelf_add_selection());
        assert_eq!(app.shelf.items[1].name, tr("kind.rectangle"));

        // Placing adds new objects around the point, selected, in one
        // undo step.
        let before = shape_count(&app);
        assert!(app.shelf_place(uid, Some(Point::new(100.0, 200.0))));
        assert_eq!(shape_count(&app), before + 2);
        assert_eq!(app.selection.len(), 2);
        assert!(!app.selection.contains(&a) && !app.selection.contains(&b));
        let placed = app.selection_bounds().expect("bounds");
        assert!((placed.center().x - 100.0).abs() < 1e-6);
        assert!((placed.center().y - 200.0).abs() < 1e-6);
        assert!((placed.width() - 40.0).abs() < 1e-6);
        app.undo();
        assert_eq!(shape_count(&app), before);

        // Nothing is placed while the Welcome Screen shows.
        app.show_welcome = true;
        assert!(!app.shelf_place(uid, None));
        assert_eq!(app.status, tr("shelf.no_drawing"));
        app.show_welcome = false;

        // Into another drawing, at the middle of the view.
        app.new_document();
        app.canvas_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        assert!(app.shelf_place(uid, None));
        assert_eq!(shape_count(&app), 2);
        let view = app.view.visible_page_rect(app.canvas_rect).center();
        let placed = app.selection_bounds().expect("bounds");
        assert!((placed.center().x - view.x).abs() < 1e-6);

        // Renaming, moving and removing.
        assert!(app.shelf_rename(uid, "  Two\nboxes "));
        assert_eq!(
            app.shelf.item(uid).map(|i| i.name.as_str()),
            Some("Two boxes")
        );
        assert!(!app.shelf_rename(uid, "   "));
        assert!(app.shelf_move(uid, 9));
        assert_eq!(app.shelf.index_of(uid), Some(1));
        app.shelf.selected = Some(uid);
        assert!(app.shelf_remove(uid));
        assert!(app.shelf.item(uid).is_none());
        assert_eq!(app.shelf.selected, None);
        assert!(!app.shelf_place(uid, None));
        assert!(!app.shelf_remove(uid));
    }

    #[test]
    fn colors_go_on_the_shelf_and_into_another_drawing() {
        use tracedraw_core::{document::ColorStyle, Color};
        let mut app = App::headless();
        assert!(!app.shelf_add_colors(), "no colours yet");
        let red = Color::rgb8(255, 0, 0);
        app.run(Command::SetDocumentPalette { colors: vec![red] });
        app.run(Command::SetColorStyles {
            styles: vec![ColorStyle {
                name: "Brand".into(),
                color: Color::rgb8(0, 0, 255),
                harmony: None,
            }],
        });
        assert!(app.shelf_add_colors());
        let uid = app.shelf.selected.expect("chosen");
        let item = app.shelf.item(uid).expect("item").clone();
        assert!(item.is_colors());
        assert_eq!(item.all_colors().len(), 2);
        let (px, w, h) = thumbnail(&item, 64).expect("picture");
        assert_eq!(px.len(), (w * h * 4) as usize);
        // Kept through the file.
        let back = items_from_bytes(&items_to_bytes(std::slice::from_ref(&item)).expect("bytes"))
            .expect("items");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].colors, vec![red]);
        // Another drawing gets them, once; undo takes them back.
        app.new_document();
        assert!(app.doc().palette.is_empty());
        assert!(app.shelf_place(uid, None));
        assert!(app.shelf_place(uid, None));
        assert_eq!(app.doc().palette, vec![red]);
        assert_eq!(app.doc().color_styles.len(), 1);
        app.undo();
        app.undo();
        assert!(app.doc().palette.is_empty());
    }

    #[test]
    fn symbols_become_objects_and_styles_travel_along() {
        let mut app = App::headless();
        let r = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
        app.select(vec![r]);
        app.new_style_from_selection();
        let style = app.doc().object_styles.first().cloned().expect("style");
        let other = rect(&mut app, 30.0, 0.0, 10.0, 10.0);
        app.select(vec![other]);
        app.create_symbol_from_selection();
        assert_eq!(app.doc().symbols.len(), 1);
        let instance = app
            .doc()
            .all_layers()
            .flat_map(|l| &l.shapes)
            .find(|s| matches!(s.kind, ShapeKind::SymbolInstance { .. }))
            .map(|s| s.id)
            .expect("instance");
        app.select(vec![r, instance]);
        assert!(app.shelf_add_selection());
        let item = app.shelf.items.last().cloned().expect("item");
        // The instance is now the symbol's rectangle, at its place.
        assert!(item
            .shapes
            .iter()
            .all(|s| !matches!(s.kind, ShapeKind::SymbolInstance { .. })));
        let b = item.bounds().expect("bounds");
        assert!((b.x1 - 40.0).abs() < 1e-6, "{b:?}");
        // Only the style in use goes along.
        assert_eq!(item.styles, vec![style.clone()]);

        // A drawing with the same style keeps the link; another drawing
        // gets the objects without it.
        let uid = item.uid;
        assert!(app.shelf_place(uid, None));
        let linked = app
            .selected_shapes()
            .iter()
            .filter(|s| s.style == Some(style.id))
            .count();
        assert_eq!(linked, 1);
        app.new_document();
        assert!(app.shelf_place(uid, None));
        assert!(app.selected_shapes().iter().all(|s| s.style.is_none()));
        assert!(app
            .selected_shapes()
            .iter()
            .all(|s| !matches!(s.kind, ShapeKind::SymbolInstance { .. })));
    }

    #[test]
    fn missing_or_self_nested_symbols_are_dropped() {
        let inner = Shape::new(ShapeId(1), ShapeKind::SymbolInstance { index: 0 });
        // Symbol 0 holds an instance of itself.
        let symbols = vec![Symbol {
            name: "loop".into(),
            shape: inner.clone(),
        }];
        assert!(without_symbols(inner.clone(), &symbols, 0).is_none());
        let missing = Shape::new(ShapeId(2), ShapeKind::SymbolInstance { index: 7 });
        assert!(without_symbols(missing.clone(), &symbols, 0).is_none());
        let group = Shape::new(
            ShapeId(3),
            ShapeKind::Group {
                children: vec![missing],
            },
        );
        assert!(without_symbols(group, &symbols, 0).is_none());
        let item = make_item("x", vec![inner], &symbols, &[]);
        assert!(item.shapes.is_empty());
        let mut shelf = Shelf::default();
        assert_eq!(shelf.push(item), None);
    }

    #[test]
    fn the_shelf_is_kept_between_sessions() {
        let dir = std::env::temp_dir().join(format!("tracedraw-shelf-{}", std::process::id()));
        let file = dir.join("shelf.json");
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 5.0, 5.0);
        app.select(vec![a]);
        assert!(app.shelf_add_selection());
        let uid = app.shelf.items[0].uid;
        assert!(app.shelf_rename(uid, "Logo"));
        app.shelf.save_to(&file).expect("save");

        let mut later = Shelf::default();
        later.load_from(&file);
        assert_eq!(later.items.len(), 1);
        assert_eq!(later.items[0].name, "Logo");
        assert_eq!(later.items[0].shapes, app.shelf.items[0].shapes);
        assert!(later.items[0].uid != 0);

        // Damaged, foreign and empty files give an empty shelf.
        for bad in [
            &b"{not json"[..],
            br#"{"kind": "tracedraw-workspace", "version": 1}"#,
            br#"{"kind": "tracedraw-shelf", "version": 1, "items": [{"name": "x", "shapes": []}]}"#,
            b"",
        ] {
            std::fs::write(&file, bad).expect("write");
            let mut s = Shelf::default();
            s.load_from(&file);
            assert!(s.items.is_empty(), "{}", String::from_utf8_lossy(bad));
        }
        // A missing file too.
        let mut s = Shelf::default();
        s.load_from(&dir.join("none.json"));
        assert!(s.items.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_are_cleaned_and_the_shelf_has_a_limit() {
        assert_eq!(clean_name("a\tb\n"), "a b");
        assert_eq!(clean_name(&"x".repeat(500)).chars().count(), MAX_NAME);
        let mut shelf = Shelf::default();
        let item = ShelfItem {
            name: "r".into(),
            shapes: vec![Shape::new(
                ShapeId(1),
                ShapeKind::Rect {
                    rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                    radius: 0.0,
                    corners: None,
                },
            )],
            styles: vec![],
            colors: vec![],
            color_styles: vec![],
            uid: 0,
        };
        for _ in 0..MAX_ITEMS {
            assert!(shelf.push(item.clone()).is_some());
        }
        assert_eq!(shelf.push(item.clone()), None);
        // Files with more items are cut to the limit.
        let many = vec![item; MAX_ITEMS + 3];
        let bytes = items_to_bytes(&many).expect("bytes");
        assert_eq!(items_from_bytes(&bytes).map(|i| i.len()), Some(MAX_ITEMS));
    }

    /// #1414: in a browser the stored shelf is read in the background;
    /// items added before it arrived come after the stored ones, within
    /// the limit, so neither is lost.
    #[test]
    fn items_added_before_the_stored_shelf_arrived_are_kept() {
        let named = |n: &str| ShelfItem {
            name: n.into(),
            shapes: vec![],
            styles: vec![],
            colors: vec![tracedraw_core::Color::BLACK],
            color_styles: vec![],
            uid: 0,
        };
        let items = merged(vec![named("a"), named("b")], vec![named("new")]);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "new"]);
        let full = merged(vec![named("old"); MAX_ITEMS], vec![named("new")]);
        assert_eq!(full.len(), MAX_ITEMS);
        // A big shelf (pictures) is kept whole: the browser's database
        // takes far more than the few megabytes of local storage.
        let bytes = items_to_bytes(&items).expect("bytes");
        assert_eq!(items_from_bytes(&bytes).map(|i| i.len()), Some(3));
    }

    #[test]
    fn thumbnails_show_the_objects() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 40.0, 20.0);
        app.select(vec![a]);
        assert!(app.shelf_add_selection());
        let item = app.shelf.items[0].clone();
        let (rgba, w, h) = thumbnail(&item, 64).expect("thumbnail");
        assert_eq!(w, 64);
        assert!(h < w && h >= 30, "{w} x {h}");
        assert_eq!(rgba.len(), (w * h * 4) as usize);
        // The middle is the rectangle's red.
        let i = (((h / 2) * w + w / 2) * 4) as usize;
        assert_eq!(&rgba[i..i + 4], &[200, 30, 30, 255]);
        // Through the texture cache.
        let ctx = egui::Context::default();
        let uid = item.uid;
        let t = app.shelf_thumbnail(&ctx, uid, 64).expect("texture");
        assert_eq!(t.size(), [64, h as usize]);
        assert!(app.shelf_thumbnail(&ctx, 999, 64).is_none());
    }
}
