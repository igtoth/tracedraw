//! Keeps the rasterized page as an egui texture, re-rendering only when the
//! document, view, canvas size or drag preview changed. While the view
//! moves (a pan, a scroll, a zoom) the last render is shown shifted or
//! scaled into place and the page is rendered anew once the view has
//! rested for a moment, so a page of thousands of objects pans smoothly.
//! A page that takes long to draw is drawn on a thread of its own: the
//! window keeps answering, showing the last picture moved into place,
//! until the new one is ready (#345).

use crate::view::View;
use egui::{ColorImage, Context, Rect, TextureHandle, TextureId, TextureOptions};
use tracedraw_core::{geometry::Affine, Document, PageId, ShapeId};
use tracedraw_render::{render_page, Preview, RenderOptions, ViewTransform};

/// How long the view must rest, seconds, before a moved view is
/// rendered anew instead of showing the last render moved into place.
pub const SETTLE_SECONDS: f64 = 0.12;

/// A render that took longer than this, seconds, makes the next ones
/// run on a thread of their own.
const SLOW_RENDER: f64 = 0.03;

/// A render running on its own thread: what it is for, and where its
/// picture will come from.
#[cfg(not(target_arch = "wasm32"))]
struct Job {
    key: Key,
    page_rect: Option<tracedraw_core::geometry::Rect>,
    rx: std::sync::mpsc::Receiver<Option<(ColorImage, f64)>>,
}

/// Turns to render on a thread: one page job runs at a time in the whole
/// program (each holds a copy of the drawing and decodes on band threads
/// of its own; Multipage view used to start one per slow page at once,
/// and a job made stale kept running beside the next one). The page being
/// edited goes first: when it had to wait, the turn is kept for it for a
/// moment after the running job ends, and other pages wait.
#[cfg(not(target_arch = "wasm32"))]
struct Turns(std::sync::Mutex<TurnState>);

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct TurnState {
    busy: bool,
    /// The current page asked while a job ran.
    current_waits: bool,
    /// Until when a freed turn is kept for the current page.
    kept_until: Option<web_time::Instant>,
    /// Turns given to other pages, for tests.
    #[cfg(test)]
    to_others: usize,
}

/// How long a freed turn waits for the current page that asked for it.
#[cfg(not(target_arch = "wasm32"))]
const KEEP_FOR_CURRENT: std::time::Duration = std::time::Duration::from_secs(1);

#[cfg(not(target_arch = "wasm32"))]
impl Turns {
    const fn new() -> Self {
        Turns(std::sync::Mutex::new(TurnState {
            busy: false,
            current_waits: false,
            kept_until: None,
            #[cfg(test)]
            to_others: 0,
        }))
    }

    fn state(&self) -> std::sync::MutexGuard<'_, TurnState> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The turn for the current page (`current`) or another one, unless a
    /// job holds it or it is kept for the current page.
    fn take(&'static self, current: bool) -> Option<Turn> {
        let mut st = self.state();
        if st.busy {
            st.current_waits |= current;
            return None;
        }
        if !current && st.kept_until.is_some_and(|t| web_time::Instant::now() < t) {
            return None;
        }
        st.busy = true;
        #[cfg(test)]
        if !current {
            st.to_others += 1;
        }
        if current {
            st.current_waits = false;
            st.kept_until = None;
        }
        Some(Turn(self))
    }
}

/// A job's turn, given back when it is dropped.
#[cfg(not(target_arch = "wasm32"))]
struct Turn(&'static Turns);

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Turn {
    fn drop(&mut self) {
        let mut st = self.0.state();
        st.busy = false;
        if std::mem::take(&mut st.current_waits) {
            st.kept_until = Some(web_time::Instant::now() + KEEP_FOR_CURRENT);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
static RENDER_TURNS: Turns = Turns::new();

#[derive(Default)]
pub struct Raster {
    texture: Option<TextureHandle>,
    key: Option<Key>,
    /// The part of the page the texture shows, page mm.
    page_rect: Option<tracedraw_core::geometry::Rect>,
    /// While the view moves: the view last asked for and when (egui
    /// time) it will be rendered if it stays.
    settle: Option<(Key, f64)>,
    /// Proofed colours on a grid of screen colours (View > Proof Colors),
    /// made when first needed and dropped with the texture (the colour
    /// settings changed).
    proof: Option<std::sync::Arc<ProofLut>>,
    /// How long the last render took, seconds.
    last_seconds: f64,
    /// The render running on its own thread, if any.
    #[cfg(not(target_arch = "wasm32"))]
    job: Option<Job>,
    /// A picture whose canvas follows the zoom (a page beside the current
    /// one in the Multipage View): the last render stays moved and scaled
    /// into place while the view moves even though the canvas changed
    /// size, instead of rendering anew every frame, and once there is a
    /// picture the next ones are drawn on a thread of their own
    /// (#902, #903).
    pub follows_zoom: bool,
    /// No render this frame (the frame's time for them is spent): the
    /// last picture shows where its part of the page now is, and the
    /// render waits for a later frame (#903).
    pub defer: bool,
    /// The drawing as given, shared, for a render on its own thread to
    /// take instead of a copy of its own (no threads in the browser).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub snapshot: Option<std::sync::Arc<Document>>,
    /// Renders started so far, on this thread or another.
    renders: u64,
    /// Turns other than the program's, for tests.
    #[cfg(not(target_arch = "wasm32"))]
    turns: Option<&'static Turns>,
    /// Every render counts as slow, for tests.
    #[cfg(test)]
    always_slow: bool,
}

/// Screen colours as they would print, on a `N` x `N` x `N` grid of the
/// screen's red, green and blue; colours between are interpolated.
struct ProofLut {
    table: Vec<[f32; 3]>,
}

impl ProofLut {
    const N: usize = 17;

    fn new() -> Self {
        let n = Self::N;
        let step = 1.0 / (n - 1) as f32;
        let mut table = Vec::with_capacity(n * n * n);
        for r in 0..n {
            for g in 0..n {
                for b in 0..n {
                    table.push(tracedraw_core::color::proof_rgb([
                        r as f32 * step,
                        g as f32 * step,
                        b as f32 * step,
                    ]));
                }
            }
        }
        ProofLut { table }
    }

    /// The proofed colour of `rgb` (0 to 1), interpolated between the
    /// eight grid colours around it.
    fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let n = Self::N;
        let scale = (n - 1) as f32;
        let mut i0 = [0usize; 3];
        let mut t = [0.0f32; 3];
        for k in 0..3 {
            let x = (rgb[k].clamp(0.0, 1.0)) * scale;
            let i = (x.floor() as usize).min(n - 2);
            i0[k] = i;
            t[k] = x - i as f32;
        }
        let at = |r: usize, g: usize, b: usize| self.table[(r * n + g) * n + b];
        let mut out = [0.0f32; 3];
        for (dr, wr) in [(0, 1.0 - t[0]), (1, t[0])] {
            for (dg, wg) in [(0, 1.0 - t[1]), (1, t[1])] {
                for (db, wb) in [(0, 1.0 - t[2]), (1, t[2])] {
                    let w = wr * wg * wb;
                    if w == 0.0 {
                        continue;
                    }
                    let c = at(i0[0] + dr, i0[1] + dg, i0[2] + db);
                    for k in 0..3 {
                        out[k] += c[k] * w;
                    }
                }
            }
        }
        out
    }

    /// A premultiplied pixel, proofed.
    fn pixel(&self, r: u8, g: u8, b: u8, a: u8) -> egui::Color32 {
        if a == 0 {
            return egui::Color32::TRANSPARENT;
        }
        let af = a as f32 / 255.0;
        let un = |v: u8| (v as f32 / 255.0 / af).min(1.0);
        let p = self.apply([un(r), un(g), un(b)]);
        let q = |v: f32| (v.clamp(0.0, 1.0) * af * 255.0).round() as u8;
        egui::Color32::from_rgba_premultiplied(q(p[0]), q(p[1]), q(p[2]), a)
    }
}

/// A rendered pixmap as an egui image, proofed through `lut` when given.
fn to_image(pixmap: &tiny_skia::Pixmap, lut: Option<&ProofLut>) -> ColorImage {
    let (w, h) = (pixmap.width(), pixmap.height());
    let pixels: Vec<egui::Color32> = pixmap
        .pixels()
        .iter()
        .map(|p| match lut {
            Some(lut) => lut.pixel(p.red(), p.green(), p.blue(), p.alpha()),
            None => egui::Color32::from_rgba_premultiplied(p.red(), p.green(), p.blue(), p.alpha()),
        })
        .collect();
    ColorImage {
        size: [w as usize, h as usize],
        source_size: egui::vec2(w as f32, h as f32),
        pixels,
    }
}

#[derive(PartialEq, Clone, Debug)]
struct Key {
    revision: u64,
    page: PageId,
    zoom: f32,
    /// The page origin relative to the canvas: what the render uses. A
    /// canvas that moves (a bar docked elsewhere) without changing size
    /// still needs a new render when the view stays put.
    offset: egui::Vec2,
    size: (u32, u32),
    preview: Option<(Vec<ShapeId>, [f64; 6])>,
    wireframe: bool,
    simulate_overprints: bool,
    complex_effects: bool,
    proof: bool,
    focus: Option<(Vec<ShapeId>, u32)>,
    /// The Pixels view: the document pixels drawn (dpi, first column, top
    /// row).
    pixels: Option<(u64, i64, i64)>,
}

/// The document pixels that cover a screen rectangle: `w` by `h` pixels
/// whose top-left one is column `ix0`, row `iy1` (rows counted up from the
/// page's bottom edge, the row above `iy1` excluded), and where they land
/// on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelWindow {
    pub ix0: i64,
    pub iy1: i64,
    pub w: u32,
    pub h: u32,
    pub screen: Rect,
}

/// Largest side of the Pixels view texture.
const MAX_PIXELS: i64 = 8192;

/// The document pixels at `dpi` behind `rect`, when one of them spans at
/// least one physical screen pixel (`ppp` per point); below that the
/// normal render already shows them.
pub fn pixel_window(view: &View, rect: Rect, dpi: f64, ppp: f32) -> Option<PixelWindow> {
    if !dpi.is_finite() || dpi <= 0.0 || !view.zoom.is_finite() || view.zoom <= 0.0 {
        return None;
    }
    let k = dpi / 25.4;
    if (view.zoom * ppp) as f64 / k < 1.0 {
        return None;
    }
    let tl = view.to_page(rect.left_top());
    let br = view.to_page(rect.right_bottom());
    let ix0 = (tl.x * k).floor() as i64;
    let ix1 = (br.x * k).ceil() as i64;
    let iy0 = (br.y * k).floor() as i64;
    let iy1 = (tl.y * k).ceil() as i64;
    let (w, h) = (ix1 - ix0, iy1 - iy0);
    if w <= 0 || h <= 0 || w > MAX_PIXELS || h > MAX_PIXELS {
        return None;
    }
    let a = view.to_screen(tracedraw_core::geometry::Point::new(
        ix0 as f64 / k,
        iy1 as f64 / k,
    ));
    let b = view.to_screen(tracedraw_core::geometry::Point::new(
        ix1 as f64 / k,
        iy0 as f64 / k,
    ));
    Some(PixelWindow {
        ix0,
        iy1,
        w: w as u32,
        h: h as u32,
        screen: Rect::from_min_max(a, b),
    })
}

impl Raster {
    /// Drop the cached texture so the next frame re-renders (and the
    /// proofed colours, which the colour settings decide).
    pub fn invalidate(&mut self) {
        self.key = None;
        self.proof = None;
        self.settle = None;
        // A render under way is for what no longer holds.
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.job = None;
        }
    }

    /// True once a picture has been rendered.
    pub fn has_picture(&self) -> bool {
        self.texture.is_some()
    }

    /// How many renders this picture has started.
    pub fn render_count(&self) -> u64 {
        self.renders
    }

    #[allow(clippy::too_many_arguments)]
    pub fn texture(
        &mut self,
        ctx: &Context,
        doc: &Document,
        page: PageId,
        view: &View,
        rect: Rect,
        preview: Option<(Vec<ShapeId>, Affine)>,
        revision: u64,
        wireframe: bool,
        simulate_overprints: bool,
        complex_effects: bool,
        proof: bool,
        focus: Option<(Vec<ShapeId>, f32)>,
        pixel_dpi: Option<f64>,
    ) -> Option<(TextureId, Rect)> {
        let ppp = ctx.pixels_per_point();
        let window = pixel_dpi.and_then(|dpi| pixel_window(view, rect, dpi, ppp));
        let (w, h) = match window {
            Some(win) => (win.w, win.h),
            None => (
                (rect.width() * ppp).round().max(1.0) as u32,
                (rect.height() * ppp).round().max(1.0) as u32,
            ),
        };
        let screen = window.map_or(rect, |win| win.screen);
        let key = Key {
            revision,
            page,
            zoom: view.zoom,
            offset: view.origin - rect.min,
            size: (w, h),
            preview: preview.as_ref().map(|(s, t)| (s.clone(), t.as_coeffs())),
            wireframe,
            simulate_overprints,
            complex_effects,
            proof,
            focus: focus
                .as_ref()
                .map(|(ids, v)| (ids.clone(), (v * 1000.0) as u32)),
            pixels: window
                .zip(pixel_dpi)
                .map(|(win, dpi)| ((dpi * 1000.0).round() as u64, win.ix0, win.iy1)),
        };
        // A render on its thread: taken when ready; while it runs for the
        // view asked for (or the drawing changes faster than it draws), the
        // last picture shows, moved into place.
        #[cfg(not(target_arch = "wasm32"))]
        if self.poll_job(ctx) {
            let running_for_this = self.job.as_ref().is_some_and(|j| j.key == key);
            if running_for_this || window.is_none() {
                return self.shown_meanwhile(view, screen);
            }
        }
        if self.key.as_ref() == Some(&key) {
            if let Some(t) = &self.texture {
                self.settle = None;
                return Some((t.id(), screen));
            }
        }
        // The view moved (pan, scroll, zoom) and nothing else changed:
        // show the last render moved into place until the view rests.
        if window.is_none() {
            if let (Some(old), Some(t), Some(page_rect)) =
                (self.key.as_ref(), self.texture.as_ref(), self.page_rect)
            {
                if old.same_but_view(&key, self.follows_zoom) {
                    let now = ctx.input(|i| i.time);
                    let due = match &self.settle {
                        Some((k, due)) if *k == key => *due,
                        _ => now + SETTLE_SECONDS,
                    };
                    if now < due {
                        self.settle = Some((key, due));
                        ctx.request_repaint_after(std::time::Duration::from_secs_f64(due - now));
                        return Some((t.id(), view.rect_to_screen(page_rect)));
                    }
                }
            }
        }
        if self.defer {
            ctx.request_repaint();
            let t = self.texture.as_ref()?;
            let page_rect = self.page_rect?;
            return Some((t.id(), view.rect_to_screen(page_rect)));
        }
        self.settle = None;
        let vt = match (window, pixel_dpi) {
            // One render pixel per document pixel, the window's top-left
            // one at the corner.
            (Some(win), Some(dpi)) => ViewTransform {
                zoom: dpi / 25.4,
                origin_x: -(win.ix0 as f64),
                origin_y: win.iy1 as f64,
            },
            _ => ViewTransform {
                zoom: (view.zoom * ppp) as f64,
                origin_x: ((view.origin.x - rect.min.x) * ppp) as f64,
                origin_y: ((view.origin.y - rect.min.y) * ppp) as f64,
            },
        };
        let opts = RenderOptions {
            width: w,
            height: h,
            view: vt,
            preview: preview.map(|(shapes, transform)| Preview { shapes, transform }),
            wireframe,
            simulate_overprints,
            complex_effects,
            anti_alias: true,
            draft: true,
            focus,
        };
        let lut = proof.then(|| {
            self.proof
                .get_or_insert_with(|| std::sync::Arc::new(ProofLut::new()))
                .clone()
        });
        let page_rect = (window.is_none()).then(|| {
            tracedraw_core::geometry::Rect::new(
                -vt.origin_x / vt.zoom,
                (vt.origin_y - h as f64) / vt.zoom,
                (w as f64 - vt.origin_x) / vt.zoom,
                vt.origin_y / vt.zoom,
            )
        });
        // A slow page, with a picture to show meanwhile: drawn on a
        // thread of its own, the window answering all along. A page
        // beside the current one always is: many of them come due in the
        // same moment, once a zoom rests (#903).
        #[cfg(test)]
        if self.always_slow {
            self.last_seconds = 1.0;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if window.is_none()
            && (self.last_seconds > SLOW_RENDER || self.follows_zoom)
            && self.texture.is_some()
        {
            // Another page's job (or a stale one) is running: the last
            // picture shows meanwhile, and the job holding the turn
            // repaints when it ends. A page beside the current one
            // yields to it.
            let Some(turn) = self.turns.unwrap_or(&RENDER_TURNS).take(!self.follows_zoom) else {
                return self.shown_meanwhile(view, screen);
            };
            let doc = self
                .snapshot
                .take()
                .unwrap_or_else(|| std::sync::Arc::new(doc.clone()));
            let (tx, rx) = std::sync::mpsc::channel();
            let wake = ctx.clone();
            std::thread::spawn(move || {
                let t = web_time::Instant::now();
                let image = render_page(&doc, page, &opts).map(|pm| to_image(&pm, lut.as_deref()));
                let _ = tx.send(image.map(|i| (i, t.elapsed().as_secs_f64())));
                drop(doc);
                drop(turn);
                wake.request_repaint();
            });
            self.renders += 1;
            self.job = Some(Job { key, page_rect, rx });
            return self.shown_meanwhile(view, screen);
        }
        let t = web_time::Instant::now();
        self.renders += 1;
        let pixmap = render_page(doc, page, &opts)?;
        let image = to_image(&pixmap, lut.as_deref());
        self.last_seconds = t.elapsed().as_secs_f64();
        // Document pixels are blown up as blocks, the way they will look.
        let filter = if window.is_some() {
            TextureOptions::NEAREST
        } else {
            TextureOptions::LINEAR
        };
        match &mut self.texture {
            Some(t) => t.set(image, filter),
            None => self.texture = Some(ctx.load_texture("page", image, filter)),
        }
        self.key = Some(key);
        self.page_rect = page_rect;
        self.texture.as_ref().map(|t| (t.id(), screen))
    }

    /// The last picture, where its part of the page now is on screen
    /// (while a new one is drawn on its thread).
    #[cfg(not(target_arch = "wasm32"))]
    fn shown_meanwhile(&self, view: &View, screen: Rect) -> Option<(TextureId, Rect)> {
        let t = self.texture.as_ref()?;
        Some((
            t.id(),
            self.page_rect.map_or(screen, |r| view.rect_to_screen(r)),
        ))
    }

    /// A render on its thread: its picture taken when ready (`true` when
    /// it is still running).
    #[cfg(not(target_arch = "wasm32"))]
    fn poll_job(&mut self, ctx: &Context) -> bool {
        let Some(job) = &self.job else {
            return false;
        };
        match job.rx.try_recv() {
            Ok(result) => {
                let job = self.job.take().expect("job");
                if let Some((image, seconds)) = result {
                    self.last_seconds = seconds;
                    match &mut self.texture {
                        Some(t) => t.set(image, TextureOptions::LINEAR),
                        None => {
                            self.texture =
                                Some(ctx.load_texture("page", image, TextureOptions::LINEAR))
                        }
                    }
                    self.key = Some(job.key);
                    self.page_rect = job.page_rect;
                }
                false
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.job = None;
                false
            }
        }
    }
}

impl Key {
    /// True when `other` shows the same drawing the same way, only from
    /// another place or at another zoom (the canvas the same size, unless
    /// `any_size`).
    fn same_but_view(&self, other: &Key, any_size: bool) -> bool {
        self.revision == other.revision
            && self.page == other.page
            && (any_size || self.size == other.size)
            && self.preview == other.preview
            && self.wireframe == other.wireframe
            && self.simulate_overprints == other.simulate_overprints
            && self.complex_effects == other.complex_effects
            && self.proof == other.proof
            && self.focus == other.focus
            && self.pixels.is_none()
            && other.pixels.is_none()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    /// One page job at a time, and the current page first: another page
    /// that asks first every frame waits while the turn is kept for the
    /// current page.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn one_render_job_runs_at_a_time_and_the_current_page_goes_first() {
        static TURNS: Turns = Turns::new();
        let job = TURNS.take(false).expect("free");
        assert!(TURNS.take(false).is_none());
        // The current page asks while the job runs, and so waits.
        assert!(TURNS.take(true).is_none());
        std::thread::spawn(move || drop(job)).join().unwrap();
        // Freed: another page asking first does not get it.
        assert!(TURNS.take(false).is_none());
        let current = TURNS.take(true).expect("kept for the current page");
        drop(current);
        // Nobody waits: whoever asks.
        assert!(TURNS.take(false).is_some());
    }

    /// Two slow pages drawn every frame, the other page asked first and
    /// the drawing changing each frame: the current page's render starts
    /// within one release of the turn.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_current_page_gets_the_next_turn() {
        static TURNS: Turns = Turns::new();
        let ctx = Context::default();
        let doc = Document::new("t", Size::new(100.0, 50.0));
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 200.0));
        let view = View::default();
        let mut current = Raster {
            turns: Some(&TURNS),
            ..Default::default()
        };
        let mut other = Raster {
            turns: Some(&TURNS),
            follows_zoom: true,
            ..Default::default()
        };
        let draw = |r: &mut Raster, revision: u64| {
            let page = doc.pages[0].id;
            let input = egui::RawInput {
                time: Some(revision as f64),
                ..Default::default()
            };
            let mut full = ctx.run_ui(input, |ui| {
                r.texture(
                    ui.ctx(),
                    &doc,
                    page,
                    &view,
                    rect,
                    None,
                    revision,
                    false,
                    false,
                    false,
                    false,
                    None,
                    None,
                );
            });
            full.textures_delta.clear();
        };
        draw(&mut other, 0);
        draw(&mut current, 0);
        other.always_slow = true;
        current.always_slow = true;
        // The other page takes the turn first.
        draw(&mut other, 1);
        assert!(other.job.is_some());
        // From the first time the current page waits, no other page may
        // take a turn before it.
        let mut given = None;
        for revision in 2..2000 {
            draw(&mut other, revision);
            draw(&mut current, revision);
            if current.job.is_some() {
                break;
            }
            let st = TURNS.state();
            if st.current_waits && given.is_none() {
                given = Some(st.to_others);
            }
            if let Some(n) = given {
                assert_eq!(st.to_others, n, "another page went first");
            }
            drop(st);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(given.is_some(), "the current page had to wait");
        assert!(current.job.is_some(), "the current page got a turn");
    }
    use tracedraw_core::geometry::Size;

    #[test]
    fn proofed_pixels_follow_the_grid_of_proofed_colours() {
        let lut = ProofLut::new();
        // On the grid, the proofed colour itself.
        let blue = lut.apply([0.0, 0.0, 1.0]);
        let want = tracedraw_core::color::proof_rgb([0.0, 0.0, 1.0]);
        for (a, b) in blue.iter().zip(want) {
            assert!((a - b).abs() < 1e-5);
        }
        // Between, close to the colour proofed directly.
        let c = [0.31, 0.62, 0.18];
        let (got, want) = (lut.apply(c), tracedraw_core::color::proof_rgb(c));
        for (a, b) in got.iter().zip(want) {
            assert!((a - b).abs() < 0.03, "{got:?} {want:?}");
        }
        // Transparent stays transparent; half-covered pixels keep their
        // coverage.
        assert_eq!(lut.pixel(0, 0, 0, 0), egui::Color32::TRANSPARENT);
        let half = lut.pixel(0, 0, 128, 128);
        assert_eq!(half.a(), 128);
        assert!(half.b() <= 128);
    }

    /// One frame at egui time `time`: the page texture for `view` on the
    /// canvas `rect`.
    fn pass(
        ctx: &Context,
        raster: &mut Raster,
        doc: &Document,
        view: &View,
        rect: Rect,
        time: f64,
    ) -> (TextureId, Rect) {
        let page = doc.pages[0].id;
        let mut out = None;
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut full = ctx.run_ui(input, |ui| {
            out = raster.texture(
                ui.ctx(),
                doc,
                page,
                view,
                rect,
                None,
                1,
                false,
                false,
                false,
                false,
                None,
                None,
            );
        });
        full.textures_delta.clear();
        out.expect("texture")
    }

    /// A pan or a zoom shows the last render moved or scaled into place
    /// at once, and renders the page anew once the view has rested.
    #[test]
    fn a_moving_view_shows_the_last_render_until_it_rests() {
        let ctx = Context::default();
        let doc = Document::new("t", Size::new(100.0, 50.0));
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 200.0));
        let mut raster = Raster::default();
        let view = View::default();
        let (first, screen) = pass(&ctx, &mut raster, &doc, &view, rect, 0.0);
        assert_eq!(screen, rect);
        let key = raster.key.clone();
        // Panned 10 points right: the same texture, 10 points to the right,
        // nothing rendered.
        let mut moved = view;
        moved.pan(egui::vec2(10.0, 0.0));
        let (tex, screen) = pass(&ctx, &mut raster, &doc, &moved, rect, 0.01);
        assert_eq!(tex, first);
        assert_eq!(raster.key, key, "no new render yet");
        assert!((screen.min.x - 10.0).abs() < 1e-3, "{screen:?}");
        assert!(raster.settle.is_some());
        // Zoomed: the texture scaled about the page, still the old one.
        let mut zoomed = moved;
        zoomed.zoom *= 2.0;
        let (tex, screen) = pass(&ctx, &mut raster, &doc, &zoomed, rect, 0.05);
        assert_eq!(tex, first);
        assert!((screen.width() - 640.0).abs() < 1e-3, "{screen:?}");
        // The view rests: rendered anew for where it is now.
        let (_, screen) = pass(
            &ctx,
            &mut raster,
            &doc,
            &zoomed,
            rect,
            0.05 + SETTLE_SECONDS + 0.01,
        );
        assert_ne!(raster.key, key);
        assert_eq!(screen, rect);
        assert!(raster.settle.is_none());
        // Moving on at once re-arms the wait from the latest move.
        let mut again = zoomed;
        again.pan(egui::vec2(0.0, 5.0));
        let before = raster.key.clone();
        pass(&ctx, &mut raster, &doc, &again, rect, 1.0);
        pass(
            &ctx,
            &mut raster,
            &doc,
            &again,
            rect,
            1.0 + SETTLE_SECONDS / 2.0,
        );
        assert_eq!(raster.key, before);
        pass(
            &ctx,
            &mut raster,
            &doc,
            &again,
            rect,
            1.0 + SETTLE_SECONDS + 0.01,
        );
        assert_ne!(raster.key, before);
    }

    /// A slow page is drawn on a thread of its own: the frame that asks
    /// for it returns at once with the last picture moved into place, and
    /// the new picture is taken once it is ready (#345).
    #[test]
    fn a_slow_page_is_drawn_on_its_own_thread() {
        // Turns of its own: the program's are shared with the other tests
        // running at the same time, whose jobs could keep it waiting.
        static TURNS: Turns = Turns::new();
        let ctx = Context::default();
        let doc = Document::new("t", Size::new(100.0, 50.0));
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 200.0));
        let mut raster = Raster {
            turns: Some(&TURNS),
            ..Default::default()
        };
        let view = View::default();
        let (first, _) = pass(&ctx, &mut raster, &doc, &view, rect, 0.0);
        let key = raster.key.clone();
        // Taken for slow: the next render runs on a thread.
        raster.last_seconds = 1.0;
        let mut zoomed = view;
        zoomed.zoom *= 2.0;
        let t = SETTLE_SECONDS + 0.01;
        pass(&ctx, &mut raster, &doc, &zoomed, rect, 0.0);
        let (mut tex, mut screen) = pass(&ctx, &mut raster, &doc, &zoomed, rect, t);
        for _ in 0..500 {
            if raster.job.is_some() {
                break;
            }
            assert_eq!(tex, first, "the old picture shows while waiting");
            std::thread::sleep(std::time::Duration::from_millis(10));
            (tex, screen) = pass(&ctx, &mut raster, &doc, &zoomed, rect, t);
        }
        assert_eq!(tex, first);
        assert!(raster.job.is_some(), "drawn on its thread");
        assert_eq!(raster.key, key, "the old picture shows meanwhile");
        assert!((screen.width() - 640.0).abs() < 1e-3, "{screen:?}");
        // Ready a moment later: taken, for the view asked for.
        let mut done = false;
        for i in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let (_, screen) = pass(&ctx, &mut raster, &doc, &zoomed, rect, t + 0.01 * i as f64);
            if raster.job.is_none() && raster.key != key {
                assert_eq!(screen, rect);
                done = true;
                break;
            }
        }
        assert!(done, "the picture from the thread was taken");
        assert!(raster.last_seconds < 1.0, "its time measured");
    }

    /// A page beside the current one in the Multipage View is rendered
    /// on a canvas the size of its paper, which changes with the zoom:
    /// while the view moves, the last picture is scaled into place, the
    /// page's corner and size following the zoom, and nothing is rendered
    /// until the view rests (#902, #903).
    #[test]
    fn a_picture_that_follows_the_zoom_keeps_its_place_while_resized() {
        // Turns of its own, as above: a turn kept for another test's
        // current page would defer this page's render.
        static TURNS: Turns = Turns::new();
        let ctx = Context::default();
        let doc = Document::new("t", Size::new(100.0, 50.0));
        let mut raster = Raster {
            follows_zoom: true,
            turns: Some(&TURNS),
            ..Default::default()
        };
        let view = View {
            zoom: 2.0,
            origin: egui::pos2(10.0, 110.0),
        };
        let paper =
            |v: &View| v.rect_to_screen(tracedraw_core::geometry::Rect::new(0.0, 0.0, 100.0, 50.0));
        let (first, at) = pass(&ctx, &mut raster, &doc, &view, paper(&view), 0.0);
        assert_eq!(at, paper(&view));
        let renders = raster.render_count();
        // Zoomed out about a point: the paper is smaller, the canvas with
        // it. The same picture, exactly on the paper.
        let mut zoomed = view;
        zoomed.zoom_at(egui::pos2(300.0, 300.0), 0.8);
        let (tex, at) = pass(&ctx, &mut raster, &doc, &zoomed, paper(&zoomed), 0.02);
        assert_eq!(tex, first);
        assert_eq!(raster.render_count(), renders, "nothing rendered");
        let want = paper(&zoomed);
        assert!((at.min - want.min).length() < 1e-3, "{at:?} {want:?}");
        assert!((at.max - want.max).length() < 1e-3, "{at:?} {want:?}");
        // At rest: rendered for the new size.
        pass(
            &ctx,
            &mut raster,
            &doc,
            &zoomed,
            paper(&zoomed),
            0.02 + SETTLE_SECONDS + 0.01,
        );
        assert_eq!(raster.render_count(), renders + 1);
        // The main canvas keeps rendering when its size changes.
        let mut main = Raster {
            turns: Some(&TURNS),
            ..Default::default()
        };
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 200.0));
        pass(&ctx, &mut main, &doc, &view, rect, 0.0);
        let wider = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(330.0, 200.0));
        pass(&ctx, &mut main, &doc, &view, wider, 0.01);
        assert_eq!(main.render_count(), 2);
    }

    /// A deferred render shows the last picture where its part of the
    /// page now is and renders nothing; the next frame without the
    /// deferral renders.
    #[test]
    fn a_deferred_render_waits_for_a_later_frame() {
        let ctx = Context::default();
        let doc = Document::new("t", Size::new(100.0, 50.0));
        let page = doc.pages[0].id;
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(320.0, 200.0));
        let view = View::default();
        let mut raster = Raster {
            defer: true,
            ..Default::default()
        };
        let at_revision = |raster: &mut Raster, revision: u64| {
            raster.texture(
                &ctx, &doc, page, &view, rect, None, revision, false, false, false, false, None,
                None,
            )
        };
        // Nothing to show yet: nothing shown, nothing rendered.
        assert!(at_revision(&mut raster, 1).is_none());
        assert_eq!(raster.render_count(), 0);
        raster.defer = false;
        let (first, _) = at_revision(&mut raster, 1).expect("rendered");
        // The drawing changed: deferred, the old picture stays.
        raster.defer = true;
        let (tex, at) = at_revision(&mut raster, 2).expect("old picture");
        assert_eq!(tex, first);
        assert_eq!(at, rect);
        assert_eq!(raster.render_count(), 1);
        raster.defer = false;
        at_revision(&mut raster, 2).expect("rendered");
        assert_eq!(raster.render_count(), 2);
    }

    #[test]
    fn a_canvas_that_moves_renders_again() {
        let ctx = Context::default();
        let doc = Document::new("t", Size::new(100.0, 50.0));
        let view = View::default();
        let mut raster = Raster::default();
        let at = |x: f32, y: f32| Rect::from_min_size(egui::pos2(x, y), egui::vec2(320.0, 200.0));
        pass(&ctx, &mut raster, &doc, &view, at(40.0, 160.0), 0.0);
        let first = raster.key.clone();
        // Same canvas, same view: the cached texture.
        pass(&ctx, &mut raster, &doc, &view, at(40.0, 160.0), 0.01);
        assert_eq!(raster.key, first);
        // The canvas moves up by a bar's height and keeps its size: the
        // page sits elsewhere inside it. The last render is shown where
        // the page now is, and the page is rendered anew once the view
        // rests, with the new offset.
        let (_, screen) = pass(&ctx, &mut raster, &doc, &view, at(40.0, 107.0), 0.02);
        assert_eq!(
            screen,
            view.rect_to_screen(raster.page_rect.expect("page rect"))
        );
        pass(
            &ctx,
            &mut raster,
            &doc,
            &view,
            at(40.0, 107.0),
            0.02 + SETTLE_SECONDS + 0.01,
        );
        assert_ne!(raster.key, first);
        assert_eq!(
            raster.key.as_ref().map(|k| k.offset),
            Some(view.origin - egui::pos2(40.0, 107.0))
        );
    }

    #[test]
    fn the_pixels_view_draws_whole_document_pixels_as_blocks() {
        // 72 dpi: a document pixel is 0.3528 mm; at 10 screen pixels per
        // mm it covers 3.5 screen pixels.
        let view = View {
            zoom: 10.0,
            origin: egui::pos2(0.0, 500.0),
        };
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 500.0));
        let win = pixel_window(&view, rect, 72.0, 1.0).expect("window");
        assert_eq!(win.ix0, 0);
        // 50 mm of page height: 141.7 pixels, rounded up to whole ones.
        assert_eq!(win.iy1, 142);
        assert_eq!(win.w, 114);
        assert_eq!(win.h, 142);
        // Whole pixels: the blocks start on the page corner and may run
        // past the canvas a little.
        assert!(win.screen.min.x <= rect.min.x && win.screen.max.x >= rect.max.x);
        assert!(win.screen.min.y <= rect.min.y);
        assert!((win.screen.max.y - 500.0).abs() < 1e-3);
        // Zoomed out below one screen pixel per document pixel: the normal
        // render.
        let far = View { zoom: 1.0, ..view };
        assert_eq!(pixel_window(&far, rect, 72.0, 1.0), None);
        assert_eq!(pixel_window(&view, rect, 0.0, 1.0), None);
        assert_eq!(pixel_window(&view, rect, f64::NAN, 1.0), None);
        // A huge window (very high resolution) falls back too.
        assert_eq!(pixel_window(&view, rect, 1.0e6, 1.0), None);
    }

    #[test]
    fn the_pixels_view_texture_is_one_texel_per_document_pixel() {
        let ctx = Context::default();
        let doc = Document::new("t", Size::new(100.0, 50.0));
        let page = doc.pages[0].id;
        let view = View {
            zoom: 40.0,
            origin: egui::pos2(0.0, 400.0),
        };
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 400.0));
        let mut raster = Raster::default();
        let (_, at) = raster
            .texture(
                &ctx,
                &doc,
                page,
                &view,
                rect,
                None,
                1,
                false,
                false,
                false,
                false,
                None,
                Some(96.0),
            )
            .expect("texture");
        let size = raster.texture.as_ref().map(|t| t.size());
        let win = pixel_window(&view, rect, 96.0, 1.0).expect("window");
        assert_eq!(size, Some([win.w as usize, win.h as usize]));
        assert_eq!(at, win.screen);
        // Without the Pixels view: screen-sized, painted on the canvas.
        let (_, at) = raster
            .texture(
                &ctx, &doc, page, &view, rect, None, 1, false, false, false, false, None, None,
            )
            .expect("texture");
        assert_eq!(at, rect);
        assert_eq!(raster.texture.as_ref().map(|t| t.size()), Some([400, 400]));
    }

    /// The browser has no clock behind std::time: asking it the time
    /// panics, and the page froze on the first drawing it showed. Code
    /// outside tests asks web_time instead (mcp_link is not in the web
    /// build).
    #[test]
    fn the_web_build_never_asks_std_for_the_time() {
        fn scan(dir: &std::path::Path, found: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).expect("src").flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if p.is_dir() {
                    scan(&p, found);
                    continue;
                }
                if !name.ends_with(".rs") || name.ends_with("_tests.rs") || name == "mcp_link.rs" {
                    continue;
                }
                let text = std::fs::read_to_string(&p).expect("source");
                for (i, line) in text.lines().enumerate() {
                    if line.trim_start().starts_with("#[cfg(test)]") {
                        break;
                    }
                    if line.contains(concat!("std::time::", "Instant::now"))
                        || line.contains(concat!("std::time::", "SystemTime::now"))
                    {
                        found.push(format!("{}:{}", p.display(), i + 1));
                    }
                }
            }
        }
        let mut found = Vec::new();
        scan(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut found,
        );
        assert!(found.is_empty(), "use web_time: {found:?}");
    }
}
