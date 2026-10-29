//! The Trace dialog's live preview: whenever a setting changes, the
//! selected bitmap is traced again off the interface thread and the
//! result drawn under the picture. Two stages per request: a quick trace
//! of a small copy, so the preview follows a slider at once, then the
//! trace of the whole picture, which is what OK places (the dialog's
//! counts come from it). A request that arrives while a trace runs
//! cancels it (the tracer looks at the flag all along, so it ends soon);
//! slider drags are debounced; a frame never waits for a trace and
//! nothing is traced again while nothing that changes the trace changed.
//! A change of the Colors tab's colour edits alone is not traced again:
//! the trace on show is given the new colours and drawn again, off the
//! interface thread too.

use crate::trace::{self, Settings, Stats, Traced};
use egui::{ColorImage, TextureHandle, TextureOptions};
use image::RgbaImage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use tracedraw_core::ShapeId;
use web_time::{Duration, Instant};

/// The longest side of the quick stage's copy, pixels.
pub const QUICK_SIDE: u32 = 240;

/// How long a slider must rest before its value is traced, so a drag
/// does not start a trace per frame.
pub const DEBOUNCE: Duration = Duration::from_millis(80);

/// Which trace a result is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Of the small copy: a first look while the whole picture is traced.
    Quick,
    /// Of the whole picture.
    Full,
}

/// What one request traces: the picture, the copy the preview shows (at
/// most `copy_side` across) and the settings.
#[derive(Clone)]
pub struct Request {
    pub shape: ShapeId,
    /// What is traced (the settings' [`Settings::traced_part`] and the
    /// picture), hashed.
    pub key: u64,
    /// The Colors tab's colour edits, hashed (see [`look_key`]).
    pub look: u64,
    pub settings: Settings,
    /// The whole picture.
    pub image: Arc<RgbaImage>,
    /// The preview copy's scale against the picture; the preview images
    /// come out at the copy's size.
    pub copy_scale: f32,
}

/// The key of the Colors tab's colour edits of `s`.
pub fn look_key(s: &Settings) -> u64 {
    crate::ui::bitmap_preview::settings_key(&s.recolored)
}

/// A finished trace and its pictures.
pub struct Outcome {
    pub key: u64,
    pub look: u64,
    pub shape: ShapeId,
    pub stage: Stage,
    /// The curves as traced, before the colour edits.
    pub raw: Arc<Vec<Traced>>,
    /// The curves in their shown colours, in the pixel space of what was
    /// traced (the whole picture for the full stage).
    pub traced: Arc<Vec<Traced>>,
    /// The distinct colours of `traced` and their curve counts (the
    /// Colors tab's list), worked out once.
    pub colors: Arc<Vec<([u8; 3], usize)>>,
    /// Mean centreline stroke width, pixels of what was traced.
    pub stroke_px: f64,
    pub stats: Stats,
    pub after: ColorImage,
    pub wire: ColorImage,
}

/// The result on show: its textures and counts.
#[derive(Clone)]
pub struct Shown {
    pub key: u64,
    pub look: u64,
    pub shape: ShapeId,
    pub stage: Stage,
    pub raw: Arc<Vec<Traced>>,
    pub traced: Arc<Vec<Traced>>,
    pub colors: Arc<Vec<([u8; 3], usize)>>,
    pub stroke_px: f64,
    pub stats: Stats,
    pub after: TextureHandle,
    pub wire: TextureHandle,
}

struct Job {
    key: u64,
    look: u64,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Outcome>,
}

/// The live preview's state, kept on the app while the dialog is open.
#[derive(Default)]
pub struct Live {
    pub shown: Option<Shown>,
    /// A request waiting for the debounce or for the running job to stop.
    pending: Option<(Request, Instant)>,
    job: Option<Job>,
    /// The whole picture decoded once per bitmap: (shape, PNG length).
    decoded: Option<((ShapeId, usize), Arc<RgbaImage>)>,
    /// How many traces were started, for tests and timings.
    pub started: usize,
    /// How many times a trace on show was only given new colours.
    pub recolored: usize,
}

impl Live {
    /// The whole picture of `shape`, decoded once.
    pub fn image(&mut self, shape: ShapeId, png: &[u8]) -> Option<Arc<RgbaImage>> {
        let id = (shape, png.len());
        if let Some((k, img)) = &self.decoded {
            if *k == id {
                return Some(img.clone());
            }
        }
        let img = Arc::new(crate::bitmap_fx::decode(png)?);
        self.decoded = Some((id, img.clone()));
        Some(img)
    }

    /// Ask for the trace of `req`; nothing happens while that trace is on
    /// show or being made. `dragging`: the pointer is held (a slider
    /// drag), so the request waits [`DEBOUNCE`] for the value to rest.
    pub fn want(&mut self, ctx: &egui::Context, req: Request, dragging: bool) {
        let same = |key: u64, look: u64| key == req.key && look == req.look;
        if self
            .shown
            .as_ref()
            .is_some_and(|s| same(s.key, s.look) && s.stage == Stage::Full)
        {
            self.pending = None;
            return;
        }
        if self.job.as_ref().is_some_and(|j| same(j.key, j.look)) {
            self.pending = None;
            return;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|(p, _)| same(p.key, p.look))
        {
            return;
        }
        let now = Instant::now();
        // A fresh change while a trace of other settings runs: that trace
        // is stale, stop it. One of the same settings with other colours
        // goes on: its curves are given the new colours once it is done.
        if let Some(job) = &self.job {
            if job.key != req.key {
                job.cancel.store(true, Ordering::Relaxed);
            }
        }
        let at = if dragging { now } else { now - DEBOUNCE };
        self.pending = Some((req, at));
        ctx.request_repaint();
    }

    /// Take finished traces and start the pending one when its time has
    /// come and no trace runs. Called once per frame while the dialog is
    /// open; returns whether a result arrived this frame.
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        let mut arrived = false;
        if let Some(job) = &self.job {
            let mut done = false;
            let mut results = Vec::new();
            // A cancelled trace's results are stale: not shown.
            let stale = job.cancel.load(Ordering::Relaxed);
            loop {
                match job.rx.try_recv() {
                    Ok(o) => {
                        if !stale {
                            results.push(o);
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
            if done {
                self.job = None;
            }
            for o in results {
                self.show(ctx, o);
                arrived = true;
            }
        }
        if let Some((req, at)) = &self.pending {
            let now = Instant::now();
            let wait = (*at + DEBOUNCE).saturating_duration_since(now);
            if self.job.is_some() {
                // Waiting for the cancelled trace to stop.
                ctx.request_repaint_after(Duration::from_millis(16));
            } else if wait.is_zero() {
                let req = req.clone();
                self.pending = None;
                self.start(ctx, req);
            } else {
                ctx.request_repaint_after(wait);
            }
        }
        arrived
    }

    fn show(&mut self, ctx: &egui::Context, o: Outcome) {
        let after = ctx.load_texture("trace_preview_after", o.after, TextureOptions::LINEAR);
        let wire = ctx.load_texture("trace_preview_wire", o.wire, TextureOptions::LINEAR);
        self.shown = Some(Shown {
            key: o.key,
            look: o.look,
            shape: o.shape,
            stage: o.stage,
            raw: o.raw,
            traced: o.traced,
            colors: o.colors,
            stroke_px: o.stroke_px,
            stats: o.stats,
            after,
            wire,
        });
    }

    fn start(&mut self, ctx: &egui::Context, req: Request) {
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        // The whole picture's trace of these settings on show: only its
        // colours change, no new trace.
        let base = self
            .shown
            .as_ref()
            .filter(|s| s.key == req.key && s.shape == req.shape && s.stage == Stage::Full)
            .map(|s| (s.raw.clone(), s.stroke_px));
        if base.is_some() {
            self.recolored += 1;
        } else {
            self.started += 1;
        }
        let (key, look) = (req.key, req.look);
        let flag = cancel.clone();
        let repaint = ctx.clone();
        let work = move || {
            let send = |o| {
                let _ = tx.send(o);
                repaint.request_repaint();
            };
            match base {
                Some((raw, stroke_px)) => recolor_only(req, raw, stroke_px, &flag, send),
                None => run(req, &flag, send),
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(work);
        #[cfg(target_arch = "wasm32")]
        work();
        self.job = Some(Job {
            key,
            look,
            cancel,
            rx,
        });
    }

    /// Whether a trace is being made or waiting to start.
    pub fn busy(&self) -> bool {
        self.job.is_some() || self.pending.is_some()
    }

    /// The whole picture's trace for `key` in the colours `look`, when it
    /// is in.
    pub fn full(&self, key: u64, look: u64) -> Option<&Shown> {
        self.shown
            .as_ref()
            .filter(|s| s.key == key && s.look == look && s.stage == Stage::Full)
    }

    /// Stop everything and drop the pictures: the dialog closed. The
    /// running trace sees its flag at its next step and ends.
    pub fn clear(&mut self) {
        if let Some(job) = &self.job {
            job.cancel.store(true, Ordering::Relaxed);
        }
        self.job = None;
        self.pending = None;
        self.shown = None;
        self.decoded = None;
    }

    /// Whether anything is kept.
    pub fn is_active(&self) -> bool {
        self.job.is_some() || self.pending.is_some() || self.shown.is_some()
    }
}

/// A copy of `img` at most `side` pixels across and its scale against it.
pub fn shrink(img: &RgbaImage, side: u32) -> (RgbaImage, f32) {
    let (w, h) = img.dimensions();
    let scale = (side as f32 / w.max(h).max(1) as f32).min(1.0);
    if scale >= 1.0 {
        return (img.clone(), 1.0);
    }
    let small = image::imageops::resize(
        img,
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
        image::imageops::FilterType::Triangle,
    );
    (small, scale)
}

fn color_image(img: &RgbaImage) -> ColorImage {
    ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw())
}

/// The pictures and counts of `raw` (traced from an image `tw` x `th`)
/// in the colours `req` gives, drawn at the copy's size.
fn outcome(
    req: &Request,
    stage: Stage,
    raw: Arc<Vec<Traced>>,
    (tw, th): (u32, u32),
    stroke_px: f64,
    cancel: &AtomicBool,
) -> Option<Outcome> {
    let (w, _) = req.image.dimensions();
    // The preview pictures come out at the copy's size.
    let copy_w = ((w as f32 * req.copy_scale).round() as u32).max(1);
    let scale = copy_w as f32 / tw.max(1) as f32;
    let traced = if req.settings.recolored.is_empty() {
        raw.clone()
    } else {
        let mut t = (*raw).clone();
        trace::recolor(&mut t, &req.settings);
        Arc::new(t)
    };
    let after = trace::render_traced_scaled(&traced, tw, th, scale, stroke_px);
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let wire = trace::render_wireframe(&traced, tw, th, scale);
    let stats = trace::stats(&traced);
    let colors = Arc::new(trace::color_counts(&traced));
    Some(Outcome {
        key: req.key,
        look: req.look,
        shape: req.shape,
        stage,
        raw,
        traced,
        colors,
        stroke_px,
        stats,
        after: color_image(&after),
        wire: color_image(&wire),
    })
}

/// The whole picture's trace `raw` given `req`'s colours: no tracing.
fn recolor_only(
    req: Request,
    raw: Arc<Vec<Traced>>,
    stroke_px: f64,
    cancel: &AtomicBool,
    mut send: impl FnMut(Outcome),
) {
    let size = req.image.dimensions();
    if let Some(o) = outcome(&req, Stage::Full, raw, size, stroke_px, cancel) {
        send(o);
    }
}

/// Trace `req` in two stages, `send` taking each result; stops once
/// `cancel` is set.
fn run(req: Request, cancel: &AtomicBool, mut send: impl FnMut(Outcome)) {
    let (w, h) = req.image.dimensions();
    // What changes the trace; the colour edits are put on after.
    let settings = req.settings.traced_part();
    let stage = |img: &RgbaImage, stage: Stage| -> Option<Outcome> {
        let raw = trace::trace_unless(img, &settings, Some(cancel))?;
        let stroke_px = trace::last_stroke_width_px();
        outcome(
            &req,
            stage,
            Arc::new(raw),
            img.dimensions(),
            stroke_px,
            cancel,
        )
    };
    // The quick look only when the picture is bigger than the copy.
    if w.max(h) > QUICK_SIDE {
        let (small, _) = shrink(&req.image, QUICK_SIDE);
        match stage(&small, Stage::Quick) {
            Some(o) => send(o),
            None => return,
        }
    }
    // In a browser the whole picture waits for OK; the preview shows the
    // quick trace.
    #[cfg(target_arch = "wasm32")]
    if w.max(h) > QUICK_SIDE && (w as u64) * (h as u64) > 400_000 {
        return;
    }
    if let Some(o) = stage(&req.image, Stage::Full) {
        send(o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disc(n: u32) -> RgbaImage {
        RgbaImage::from_fn(n, n, |x, y| {
            let c = n as f32 / 2.0;
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            if d < c * 0.6 {
                image::Rgba([0, 0, 0, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        })
    }

    fn request(img: RgbaImage, key: u64) -> Request {
        let (w, h) = img.dimensions();
        let copy_scale = (400.0 / w.max(h) as f32).min(1.0);
        Request {
            shape: ShapeId(7),
            key,
            look: 0,
            settings: trace::Preset::Logo.settings(),
            image: Arc::new(img),
            copy_scale,
        }
    }

    /// A big picture gives a quick result first and the full one after,
    /// both drawn at the copy's size; a small one only the full result.
    #[test]
    fn a_request_gives_a_quick_look_then_the_whole_trace() {
        let mut got = Vec::new();
        let cancel = AtomicBool::new(false);
        run(request(disc(600), 1), &cancel, |o| got.push(o));
        assert_eq!(
            got.iter().map(|o| o.stage).collect::<Vec<_>>(),
            vec![Stage::Quick, Stage::Full]
        );
        for o in &got {
            assert_eq!(o.after.size, [400, 400], "{:?}", o.stage);
            assert_eq!(o.wire.size, [400, 400]);
            assert_eq!(o.stats.curves, 1);
        }
        let mut got = Vec::new();
        run(request(disc(100), 2), &cancel, |o| got.push(o));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].stage, Stage::Full);
    }

    /// A cancelled request gives nothing back.
    #[test]
    fn a_cancelled_request_gives_nothing() {
        let mut got = Vec::new();
        let cancel = AtomicBool::new(true);
        run(request(disc(300), 3), &cancel, |o| got.push(o));
        assert!(got.is_empty());
    }

    /// The live preview starts one trace per distinct request, drops a
    /// request already on show, and debounces while the pointer is held.
    #[test]
    fn the_live_preview_traces_each_change_once() {
        let ctx = egui::Context::default();
        let mut live = Live::default();
        let img = Arc::new(disc(120));
        let req = |key: u64| Request {
            shape: ShapeId(1),
            key,
            look: 0,
            settings: trace::Preset::Logo.settings(),
            image: img.clone(),
            copy_scale: 1.0,
        };
        live.want(&ctx, req(1), false);
        live.poll(&ctx);
        assert_eq!(live.started, 1);
        assert!(live.busy());
        let t0 = Instant::now();
        while live.full(1, 0).is_none() && t0.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(2));
            live.poll(&ctx);
        }
        let shown = live.full(1, 0).expect("the trace");
        assert_eq!(shown.stats.curves, 1);
        assert!(!live.busy());
        // The same request again changes nothing.
        live.want(&ctx, req(1), false);
        live.poll(&ctx);
        assert_eq!(live.started, 1);
        // A drag's values wait for the debounce; the latest one wins.
        live.want(&ctx, req(2), true);
        live.poll(&ctx);
        assert_eq!(live.started, 1, "still resting");
        live.want(&ctx, req(3), true);
        std::thread::sleep(DEBOUNCE + Duration::from_millis(10));
        live.poll(&ctx);
        assert_eq!(live.started, 2);
        let t0 = Instant::now();
        while live.full(3, 0).is_none() && t0.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(2));
            live.poll(&ctx);
        }
        assert!(live.full(3, 0).is_some());
        live.clear();
        assert!(!live.is_active());
    }

    fn settle(live: &mut Live, ctx: &egui::Context, key: u64, look: u64) {
        let t0 = Instant::now();
        while live.full(key, look).is_none() && t0.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(2));
            live.poll(ctx);
        }
    }

    /// A change of the colour edits alone traces nothing again: the
    /// whole picture's trace on show is given the new colours and drawn
    /// again, with its colour list worked out once.
    #[test]
    fn new_colours_alone_are_not_traced_again() {
        let ctx = egui::Context::default();
        let mut live = Live::default();
        let img = Arc::new(disc(120));
        let req = |settings: Settings| Request {
            shape: ShapeId(1),
            key: 5,
            look: look_key(&settings),
            settings,
            image: img.clone(),
            copy_scale: 1.0,
        };
        let plain = trace::Preset::Logo.settings();
        live.want(&ctx, req(plain.clone()), false);
        live.poll(&ctx);
        settle(&mut live, &ctx, 5, look_key(&plain));
        let first = live.full(5, look_key(&plain)).expect("the trace").clone();
        assert_eq!(first.colors.as_slice(), &[([0, 0, 0], 1)]);
        assert_eq!(live.started, 1);
        let mut green = plain.clone();
        green.recolored = vec![([0, 0, 0], [0, 160, 0])];
        live.want(&ctx, req(green.clone()), false);
        live.poll(&ctx);
        settle(&mut live, &ctx, 5, look_key(&green));
        let shown = live.full(5, look_key(&green)).expect("the new colours");
        assert_eq!(live.started, 1, "not traced again");
        assert_eq!(live.recolored, 1);
        assert_eq!(shown.colors.as_slice(), &[([0, 160, 0], 1)]);
        assert_eq!(shown.traced[0].color.to_rgb8(), [0, 160, 0]);
        assert_eq!(shown.raw[0].color.to_rgb8(), [0, 0, 0]);
        assert_eq!(shown.traced[0].path, first.traced[0].path);
        // The same as tracing with the edits in the settings.
        let direct = trace::trace(&img, &green);
        assert_eq!(direct[0].color.to_rgb8(), [0, 160, 0]);
        assert_eq!(direct[0].path, shown.traced[0].path);
    }

    /// While a trace runs, closing the dialog (clear) raises its flag:
    /// the thread ends soon, nothing it made is shown.
    #[test]
    fn clearing_stops_the_running_trace() {
        let ctx = egui::Context::default();
        let mut live = Live::default();
        let big = RgbaImage::from_fn(1400, 1400, |x, y| {
            let n = ((x * 7919) ^ (y * 104_729)) % 31;
            let v = (if (x / 70 + y / 70) % 2 == 0 {
                30 + n
            } else {
                200 + n
            }) as u8;
            image::Rgba([v, 255 - v, v / 2, 255])
        });
        let mut settings = trace::Preset::Clipart.settings();
        settings.detail = 40.0;
        let req = Request {
            shape: ShapeId(2),
            key: 9,
            look: 0,
            settings,
            image: Arc::new(big),
            copy_scale: 0.3,
        };
        live.want(&ctx, req, false);
        live.poll(&ctx);
        assert_eq!(live.started, 1);
        let job = live.job.as_ref().expect("running");
        let flag = job.cancel.clone();
        // Keep a way to see the thread end: the receiver of its results.
        let (_, rx) = mpsc::channel::<Outcome>();
        let rx = std::mem::replace(&mut live.job.as_mut().expect("running").rx, rx);
        std::thread::sleep(Duration::from_millis(30));
        live.clear();
        assert!(flag.load(Ordering::Relaxed));
        assert!(!live.is_active());
        let t0 = Instant::now();
        loop {
            match rx.try_recv() {
                Err(TryRecvError::Disconnected) => break,
                _ => std::thread::sleep(Duration::from_millis(2)),
            }
            assert!(
                t0.elapsed() < Duration::from_secs(3),
                "the trace still runs after the dialog closed"
            );
        }
    }
}
