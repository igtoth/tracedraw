//! Live preview of a bitmap effect's dialog on the drawing: while the
//! dialog is open and its Preview box is ticked, the selected objects
//! show the effect with the values the dialog holds, made again each
//! time a value changes. The work runs off the interface thread (the
//! canvas keeps answering; a busy mark shows in the dialog meanwhile),
//! starts once a changed value has rested for a moment, and never twice
//! for the same values. At most one preview is being made at any time: a
//! dropped one (the box cleared, Cancel) stops at its next effect, and the
//! next one waits until it has. Values whose making failed are not made
//! again until a value changes. In the browser the work runs on the
//! interface thread, so the box starts cleared there. OK keeps what the drawing shows as one undo step
//! without making it again; Cancel, Esc, the close button or clearing the
//! box put the drawing back exactly as it was, with no undo step.

use crate::app::App;
use crate::ui::effect_dialog::EffectState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use tracedraw_core::document::ShapeKind;
use tracedraw_core::{BitmapEffect, BitmapFxStack, Command, Shape, ShapeId, Symbol};
use web_time::Instant;

/// How long a changed value rests, seconds, before the preview is made
/// (a slider drag changes it every frame).
pub const DEBOUNCE: f64 = 0.08;
/// A value changing all along still gets a preview this often, seconds.
pub const MAX_WAIT: f64 = 0.3;

/// An object a bitmap effect works on: a bitmap's stack (its original
/// pixels and effects), or a vector object drawn as its picture.
#[derive(Clone)]
pub enum FxTarget {
    Bitmap { id: ShapeId, stack: BitmapFxStack },
    Vector { shape: Box<Shape> },
}

impl FxTarget {
    /// The command that gives the target its effect list changed by `f`:
    /// the stack rendered again (grown first by what the effects reach
    /// when `inflate`), or the vector object's picture made anew at `dpi`.
    /// `None` when there is nothing to draw, or `cancel` was raised.
    pub fn command(
        self,
        f: impl FnOnce(&mut Vec<BitmapEffect>),
        symbols: &[Symbol],
        inflate: bool,
        dpi: f64,
        cancel: Option<&AtomicBool>,
    ) -> Option<Command> {
        let cancelled = || cancel.is_some_and(|c| c.load(Ordering::Relaxed));
        match self {
            FxTarget::Vector { shape } => {
                let mut effects = shape
                    .raster_fx
                    .as_ref()
                    .map(|fx| fx.effects.clone())
                    .unwrap_or_default();
                f(&mut effects);
                let fx = if effects.is_empty() {
                    None
                } else {
                    Some(super::vector_raster_fx_until(
                        &shape, symbols, effects, inflate, dpi, cancel,
                    )?)
                };
                if cancelled() {
                    return None;
                }
                Some(Command::SetRasterFx {
                    shape: shape.id,
                    fx,
                })
            }
            FxTarget::Bitmap { id, mut stack } => {
                f(&mut stack.effects);
                let kind = if stack.effects.is_empty() {
                    ShapeKind::Bitmap {
                        rect: stack.rect,
                        width_px: stack.width_px,
                        height_px: stack.height_px,
                        png: stack.png,
                        fx: None,
                    }
                } else {
                    let (img, r) = super::render_stack(&stack, inflate, cancel)?;
                    if cancelled() {
                        return None;
                    }
                    let png = crate::bitmap_fx::encode(&img)?;
                    ShapeKind::Bitmap {
                        rect: r,
                        width_px: img.width(),
                        height_px: img.height(),
                        png: png.into(),
                        fx: Some(Box::new(stack)),
                    }
                };
                Some(Command::SetShapeKind { shape: id, kind })
            }
        }
    }
}

/// What an effect dialog does to an object's effect list.
#[derive(Debug, Clone, PartialEq)]
pub enum FxChange {
    /// Add the effect at the end (the Effects menu).
    Add(BitmapEffect),
    /// Change the effect at the index (Edit in the FX section).
    Replace(usize, BitmapEffect),
}

impl FxChange {
    pub fn apply(&self, list: &mut Vec<BitmapEffect>) {
        match self {
            FxChange::Add(e) => list.push(e.clone()),
            FxChange::Replace(i, e) => {
                if let Some(slot) = list.get_mut(*i) {
                    *slot = e.clone();
                }
            }
        }
    }

    /// The undo step's label.
    pub fn label(&self) -> &'static str {
        match self {
            FxChange::Add(_) => "Add Effect",
            FxChange::Replace(..) => "Edit Effect",
        }
    }

    /// The change the dialog's state asks for.
    pub fn of(st: &EffectState) -> FxChange {
        match st.edit {
            Some((_, i)) => FxChange::Replace(i, st.effect.clone()),
            None => FxChange::Add(st.effect.clone()),
        }
    }
}

/// The commands that give every target `change`, in order; `None` once
/// `cancel` is raised (the rest is not made).
pub fn fx_commands(
    targets: Vec<FxTarget>,
    change: &FxChange,
    symbols: &[Symbol],
    inflate: bool,
    dpi: f64,
    cancel: &AtomicBool,
) -> Option<Vec<Command>> {
    let mut cmds = Vec::new();
    for t in targets {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(c) = t.command(|l| change.apply(l), symbols, inflate, dpi, Some(cancel)) {
            cmds.push(c);
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    Some(cmds)
}

/// A preview being made.
struct Job {
    key: u64,
    label: &'static str,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Option<Vec<Command>>>,
}

/// A change waiting for its moment.
#[derive(Debug, Clone, Copy)]
struct Pending {
    key: u64,
    /// When the first value not yet shown changed.
    since: Instant,
    /// When the value last changed.
    changed: Instant,
}

/// The state of the live preview (see the module notes).
#[derive(Default)]
pub struct FxPreview {
    /// The values the drawing shows, as a key; `None` when it shows none.
    pub shown: Option<u64>,
    /// The engine holds steps of ours to withdraw.
    active: bool,
    pending: Option<Pending>,
    job: Option<Job>,
    /// When the last preview started being made, until the drawing shows
    /// the dialog's values.
    last_start: Option<Instant>,
    /// The objects as they are without the preview, gathered before it
    /// went on the drawing: what every preview starts from.
    base: Option<Vec<FxTarget>>,
    /// A dropped job whose thread may still run: no new one starts until
    /// it has ended, so dropped previews never pile up.
    stale: Option<Job>,
    /// The values whose making failed (its thread died): not made again
    /// until a value changes.
    failed: Option<u64>,
    /// How many previews were made, for tests.
    pub made: usize,
}

impl FxPreview {
    /// A preview is being made or waits to be.
    pub fn busy(&self) -> bool {
        self.job.is_some() || self.pending.is_some()
    }

    /// The drawing shows a preview (steps the engine holds).
    pub fn on_drawing(&self) -> bool {
        self.active
    }

    fn cancel_job(&mut self) {
        if let Some(j) = self.job.take() {
            j.cancel.store(true, Ordering::Relaxed);
            // Only one job can run at a time, so an older stale one has
            // ended already.
            self.stale = Some(j);
        }
        self.pending = None;
    }

    /// A dropped job's thread still runs.
    fn stale_running(&mut self) -> bool {
        let Some(j) = &self.stale else {
            return false;
        };
        if let Err(TryRecvError::Empty) = j.rx.try_recv() {
            return true;
        }
        self.stale = None;
        false
    }
}

/// The key of what a dialog's state would show on these objects.
pub fn preview_key(st: &EffectState, selection: &[ShapeId]) -> u64 {
    crate::ui::bitmap_preview::settings_key(&(&st.effect, st.edit, selection))
}

impl App {
    /// The target the effect dialogs work on for `id`, if it takes
    /// bitmap effects.
    pub fn fx_target(&self, id: ShapeId) -> Option<FxTarget> {
        let s = self.doc().find_shape(id)?;
        if super::takes_vector_fx(&s.kind) {
            return Some(FxTarget::Vector {
                shape: Box::new(s.clone()),
            });
        }
        let ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            png,
            fx,
        } = &s.kind
        else {
            return None;
        };
        let stack = match fx {
            Some(st) => (**st).clone(),
            None => BitmapFxStack {
                rect: *rect,
                width_px: *width_px,
                height_px: *height_px,
                png: png.clone(),
                effects: Vec::new(),
            },
        };
        Some(FxTarget::Bitmap { id, stack })
    }

    /// The objects an effect dialog changes: the edited object, or the
    /// selection.
    fn fx_targets(&self, st: &EffectState) -> Vec<FxTarget> {
        match st.edit {
            Some((id, _)) => self.fx_target(id).into_iter().collect(),
            None => self
                .selection
                .iter()
                .filter_map(|id| self.fx_target(*id))
                .collect(),
        }
    }

    /// The dialog's values differ from what the drawing shows without a
    /// preview: adding always does; editing, once the effect changed (a
    /// preview on the drawing stays, whatever it shows).
    fn fx_preview_wanted(&self, st: &EffectState) -> bool {
        match st.edit {
            None => true,
            Some((id, i)) => {
                self.fx_preview.active || self.bitmap_effects(id).get(i) != Some(&st.effect)
            }
        }
    }

    /// Each frame the dialog is open: show the dialog's values on the
    /// drawing, made again only when they change, off the interface
    /// thread once they have rested for [`DEBOUNCE`] (or [`MAX_WAIT`]
    /// while they keep changing); one preview is made at a time, a value
    /// changed meanwhile follows when it is done.
    pub fn preview_bitmap_effect(&mut self, ctx: &egui::Context, st: &EffectState) {
        if !st.preview || !self.fx_preview_wanted(st) {
            self.end_bitmap_effect_preview();
            return;
        }
        self.poll_fx_preview();
        let key = preview_key(st, &self.selection);
        if self.fx_preview.shown == Some(key) || self.fx_preview.failed == Some(key) {
            self.fx_preview.pending = None;
            self.fx_preview.last_start = None;
            return;
        }
        self.fx_preview.failed = None;
        let now = Instant::now();
        let p = match self.fx_preview.pending {
            Some(p) if p.key == key => p,
            Some(p) => Pending {
                key,
                since: p.since,
                changed: now,
            },
            // A value changed while the last preview was made counts
            // from that preview's start: it follows without a new wait.
            None => Pending {
                key,
                since: self.fx_preview.last_start.unwrap_or(now),
                changed: now,
            },
        };
        self.fx_preview.pending = Some(p);
        let due = (now - p.changed).as_secs_f64() >= DEBOUNCE
            || (now - p.since).as_secs_f64() >= MAX_WAIT;
        if self.fx_preview.job.is_some() {
            // Its result comes with a repaint; the value pending follows.
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        }
        if !due {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(DEBOUNCE));
            return;
        }
        if self.fx_preview.stale_running() {
            // A dropped preview still ends its current effect; its thread
            // repaints when done.
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        }
        self.start_fx_preview(ctx, st, key);
    }

    /// Make the preview for `key` on a thread of its own (in the browser,
    /// now), its result taken by [`Self::poll_fx_preview`].
    fn start_fx_preview(&mut self, ctx: &egui::Context, st: &EffectState, key: u64) {
        // With a preview on the drawing the objects show it: the next one
        // starts from the objects as gathered before it.
        let targets = match &self.fx_preview.base {
            Some(base) if self.fx_preview.active => base.clone(),
            _ => {
                let t = self.fx_targets(st);
                self.fx_preview.base = Some(t.clone());
                t
            }
        };
        let change = FxChange::of(st);
        let label = change.label();
        let symbols = self.doc().symbols.clone();
        let inflate = self.doc().metadata.auto_inflate_bitmaps();
        let dpi = super::vector_fx_dpi(&self.doc().metadata);
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let flag = cancel.clone();
        let wake = ctx.clone();
        let work = move || {
            let out = fx_commands(targets, &change, &symbols, inflate, dpi, &flag);
            let _ = tx.send(out);
            wake.request_repaint();
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(work);
        #[cfg(target_arch = "wasm32")]
        work();
        self.fx_preview.pending = None;
        self.fx_preview.last_start = Some(Instant::now());
        self.fx_preview.job = Some(Job {
            key,
            label,
            cancel,
            rx,
        });
    }

    /// A finished preview goes on the drawing, as steps the engine holds
    /// for withdrawal; the one shown before goes first.
    fn poll_fx_preview(&mut self) {
        let Some(job) = self.fx_preview.job.take() else {
            return;
        };
        let out = match job.rx.try_recv() {
            Ok(out) => out,
            Err(TryRecvError::Empty) => {
                self.fx_preview.job = Some(job);
                return;
            }
            Err(TryRecvError::Disconnected) => None,
        };
        let Some(cmds) = out else {
            // The thread died (an effect failed): these values are not
            // made again until one changes.
            self.fx_preview.failed = Some(job.key);
            self.fx_preview.last_start = None;
            return;
        };
        self.withdraw_fx_preview();
        if !cmds.is_empty() {
            self.engine.begin_preview();
            self.fx_preview.active = true;
            if let Err(e) = self.engine.run_batch(job.label, &cmds) {
                self.status = e.to_string();
            }
        }
        self.fx_preview.shown = Some(job.key);
        self.fx_preview.made += 1;
    }

    /// Take the preview off the drawing (the history as it was).
    fn withdraw_fx_preview(&mut self) {
        if std::mem::take(&mut self.fx_preview.active) {
            self.engine.withdraw_preview();
        }
        self.fx_preview.shown = None;
    }

    /// Cancel or the dialog closing some other way, or the Preview box
    /// cleared: the drawing goes back as it was, a preview being made is
    /// dropped.
    pub fn end_bitmap_effect_preview(&mut self) {
        self.fx_preview.cancel_job();
        self.withdraw_fx_preview();
        self.fx_preview.base = None;
        self.fx_preview.last_start = None;
        self.fx_preview.failed = None;
    }

    /// OK: the preview the drawing shows is kept as one undo step; with
    /// none shown yet (or an older one) the effect is made now.
    pub fn commit_bitmap_effect(&mut self, st: &EffectState) {
        self.poll_fx_preview();
        let key = preview_key(st, &self.selection);
        if self.fx_preview.shown == Some(key) {
            self.fx_preview.cancel_job();
            if std::mem::take(&mut self.fx_preview.active) {
                self.engine.keep_preview();
            }
            self.fx_preview.shown = None;
            self.fx_preview.base = None;
            self.fx_preview.last_start = None;
            return;
        }
        self.end_bitmap_effect_preview();
        let e = st.effect.clone();
        match st.edit {
            Some((id, i)) => self.edit_bitmap_effects(id, "Edit Effect", move |list| {
                if let Some(slot) = list.get_mut(i) {
                    *slot = e;
                }
            }),
            None => self.add_bitmap_effect(e),
        }
    }

    /// Wait for a preview being made (tests): frames until it is on the
    /// drawing or `seconds` pass.
    #[cfg(test)]
    pub fn settle_fx_preview(&mut self, ctx: &egui::Context, seconds: f64) -> bool {
        let t = Instant::now();
        while t.elapsed().as_secs_f64() < seconds {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 1000.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(self, ui));
            out.textures_delta.clear();
            if !self.fx_preview.busy() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;
    use image::{ImageBuffer, Rgba, RgbaImage};

    fn frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    fn enter() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// A drawing with a `w` x `h` bitmap of a sharp pattern, selected.
    fn bitmap_app(w: u32, h: u32) -> (egui::Context, App, ShapeId) {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        let img: RgbaImage = ImageBuffer::from_fn(w, h, |x, y| {
            if (x / 4 + y / 4) % 2 == 0 {
                Rgba([250, 30, 30, 255])
            } else {
                Rgba([30, 30, 250, 255])
            }
        });
        let png = crate::bitmap_fx::encode(&img).expect("png");
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: tracedraw_core::Rect::new(
                    10.0,
                    10.0,
                    10.0 + w as f64 / 4.0,
                    10.0 + h as f64 / 4.0,
                ),
                width_px: w,
                height_px: h,
                png: png.into(),
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id]);
        for _ in 0..2 {
            frame(&ctx, &mut app, Vec::new());
        }
        (ctx, app, id)
    }

    fn effects_of(app: &App, id: ShapeId) -> Vec<BitmapEffect> {
        app.bitmap_effects(id)
    }

    fn shown_png_len(app: &App, id: ShapeId) -> usize {
        match &app.doc().find_shape(id).expect("bitmap").kind {
            ShapeKind::Bitmap { png, .. } => png.len(),
            _ => 0,
        }
    }

    fn set_radius(app: &mut App, r: f64) {
        let Dialog::Effect(st) = &mut app.dialog else {
            panic!("the dialog: {:?}", app.dialog);
        };
        st.effect.params.insert("radius".into(), r);
    }

    /// The Blur dialog shows its effect on the drawing as soon as it
    /// opens, follows a changed value without any button pressed, makes
    /// nothing again while nothing changes, Esc (the dialog closed) puts
    /// the drawing back with no undo step, and OK keeps it as one step.
    #[test]
    fn the_effect_shows_on_the_drawing_while_its_dialog_is_open() {
        let (ctx, mut app, id) = bitmap_app(40, 32);
        let steps = app.engine.history_labels().0.len();
        let plain = shown_png_len(&app, id);
        crate::ui::effect_dialog::choose_effect(&mut app, "gaussian_blur");
        assert!(matches!(app.dialog, Dialog::Effect(_)));
        assert!(app.settle_fx_preview(&ctx, 10.0), "the preview is made");
        assert_eq!(effects_of(&app, id).len(), 1, "shown on the drawing");
        assert_eq!(effects_of(&app, id)[0].id, "gaussian_blur");
        assert!(app.engine.in_preview());
        assert!(app.fx_preview.on_drawing());
        assert_eq!(app.fx_preview.made, 1);
        // Nothing changes: nothing is made again.
        let rev = app.engine.revision();
        for _ in 0..5 {
            frame(&ctx, &mut app, Vec::new());
        }
        assert_eq!(app.engine.revision(), rev);
        assert_eq!(app.fx_preview.made, 1);
        // A value changes: the drawing follows, no button pressed.
        set_radius(&mut app, 9.0);
        assert!(app.settle_fx_preview(&ctx, 10.0));
        assert_eq!(app.fx_preview.made, 2);
        assert_eq!(effects_of(&app, id)[0].params.get("radius"), Some(&9.0));
        assert_ne!(shown_png_len(&app, id), plain);
        // The dialog closed some other way (Esc): back as it was, no step.
        app.dialog = Dialog::None;
        frame(&ctx, &mut app, Vec::new());
        assert!(effects_of(&app, id).is_empty());
        assert_eq!(shown_png_len(&app, id), plain);
        assert_eq!(app.engine.history_labels().0.len(), steps);
        assert!(!app.engine.in_preview());
        assert!(!app.fx_preview.on_drawing());
        // OK (Enter): the preview shown is kept as one step, not made again.
        crate::ui::effect_dialog::choose_effect(&mut app, "gaussian_blur");
        set_radius(&mut app, 5.0);
        assert!(app.settle_fx_preview(&ctx, 10.0));
        let made = app.fx_preview.made;
        frame(&ctx, &mut app, vec![enter()]);
        assert!(matches!(app.dialog, Dialog::None), "{:?}", app.dialog);
        assert_eq!(app.fx_preview.made, made);
        assert!(!app.engine.in_preview());
        let labels = app.engine.history_labels().0;
        assert_eq!(labels.len(), steps + 1);
        assert_eq!(labels.last(), Some(&"Add Effect"));
        assert_eq!(effects_of(&app, id)[0].params.get("radius"), Some(&5.0));
        app.undo();
        assert!(effects_of(&app, id).is_empty());
        assert_eq!(shown_png_len(&app, id), plain);
    }

    /// The Preview box cleared takes the effect off the drawing (no step
    /// left); ticked again, it comes back. OK with the box cleared makes
    /// the effect then, still one step.
    #[test]
    fn the_preview_box_puts_the_effect_on_the_drawing_and_takes_it_off() {
        let (ctx, mut app, id) = bitmap_app(40, 32);
        let steps = app.engine.history_labels().0.len();
        crate::ui::effect_dialog::choose_effect(&mut app, "gaussian_blur");
        assert!(app.settle_fx_preview(&ctx, 10.0));
        assert_eq!(effects_of(&app, id).len(), 1);
        let toggle = |app: &mut App, on: bool| {
            let Dialog::Effect(st) = &mut app.dialog else {
                panic!("the dialog");
            };
            st.preview = on;
        };
        toggle(&mut app, false);
        frame(&ctx, &mut app, Vec::new());
        assert!(effects_of(&app, id).is_empty());
        assert_eq!(app.engine.history_labels().0.len(), steps);
        assert!(!app.fx_preview.busy());
        toggle(&mut app, true);
        assert!(app.settle_fx_preview(&ctx, 10.0));
        assert_eq!(effects_of(&app, id).len(), 1);
        toggle(&mut app, false);
        frame(&ctx, &mut app, Vec::new());
        frame(&ctx, &mut app, vec![enter()]);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        assert_eq!(effects_of(&app, id).len(), 1);
        assert!(!app.engine.in_preview());
    }

    /// Editing an effect from the FX section: the drawing keeps showing
    /// the effect as it is until a value changes, then shows the change;
    /// OK changes the effect in place as one step.
    #[test]
    fn editing_an_effect_previews_the_change_in_its_place() {
        let (ctx, mut app, id) = bitmap_app(40, 32);
        let mut blur = super::super::new_effect("gaussian_blur");
        blur.params.insert("radius".into(), 2.0);
        app.add_bitmap_effect(blur);
        let steps = app.engine.history_labels().0.len();
        let shown = shown_png_len(&app, id);
        app.dialog = Dialog::Effect(crate::ui::effect_dialog::EffectState {
            effect: effects_of(&app, id)[0].clone(),
            edit: Some((id, 0)),
            channel: 0,
            lab: Default::default(),
            pick: None,
            preview: true,
        });
        for _ in 0..3 {
            frame(&ctx, &mut app, Vec::new());
        }
        assert_eq!(app.fx_preview.made, 0, "unchanged: nothing to make");
        assert_eq!(shown_png_len(&app, id), shown);
        set_radius(&mut app, 7.0);
        assert!(app.settle_fx_preview(&ctx, 10.0));
        assert_eq!(effects_of(&app, id).len(), 1);
        assert_eq!(effects_of(&app, id)[0].params.get("radius"), Some(&7.0));
        assert!(app.engine.in_preview());
        frame(&ctx, &mut app, vec![enter()]);
        let labels = app.engine.history_labels().0;
        assert_eq!(labels.len(), steps + 1);
        assert_eq!(labels.last(), Some(&"Edit Effect"));
        assert_eq!(effects_of(&app, id)[0].params.get("radius"), Some(&7.0));
    }

    /// A vector object shows the effect as its picture while the dialog
    /// is open, and is itself again on Cancel.
    #[test]
    fn a_vector_object_previews_its_picture() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(20.0, 20.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        let steps = app.engine.history_labels().0.len();
        crate::ui::effect_dialog::choose_effect(&mut app, "gaussian_blur");
        assert!(app.settle_fx_preview(&ctx, 10.0));
        let s = app.doc().find_shape(id).expect("rect").clone();
        let fx = s.raster_fx.as_ref().expect("the picture");
        assert_eq!(fx.effects.len(), 1);
        assert!(!fx.png.is_empty());
        app.dialog = Dialog::None;
        frame(&ctx, &mut app, Vec::new());
        assert!(app.doc().find_shape(id).expect("rect").raster_fx.is_none());
        assert_eq!(app.engine.history_labels().0.len(), steps);
    }

    /// A cancelled job makes nothing more.
    #[test]
    fn a_cancelled_job_gives_nothing() {
        let (_, app, id) = bitmap_app(16, 16);
        let t = app.fx_target(id).expect("target");
        let cancel = AtomicBool::new(true);
        let change = FxChange::Add(super::super::new_effect("gaussian_blur"));
        assert!(fx_commands(vec![t], &change, &[], true, 300.0, &cancel).is_none());
        let t = app.fx_target(id).expect("target");
        let cancel = AtomicBool::new(false);
        let cmds = fx_commands(vec![t], &change, &[], true, 300.0, &cancel).expect("made");
        assert_eq!(cmds.len(), 1);
    }

    /// The render itself stops at a raised flag (before its next effect),
    /// so a dropped preview ends soon instead of finishing its work.
    #[test]
    fn a_raised_flag_stops_the_render() {
        let (_, app, id) = bitmap_app(16, 16);
        let Some(FxTarget::Bitmap { mut stack, .. }) = app.fx_target(id) else {
            panic!("a bitmap target");
        };
        stack
            .effects
            .push(super::super::new_effect("gaussian_blur"));
        let raised = AtomicBool::new(true);
        assert!(super::super::render_stack(&stack, true, Some(&raised)).is_none());
        let lowered = AtomicBool::new(false);
        assert!(super::super::render_stack(&stack, true, Some(&lowered)).is_some());
    }

    /// Frames for a while, `n` of them `ms` apart.
    fn frames(ctx: &egui::Context, app: &mut App, n: usize, ms: u64) {
        for _ in 0..n {
            frame(ctx, app, Vec::new());
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }

    /// A job whose thread has not answered yet, and its sender.
    fn job(key: u64) -> (Job, std::sync::mpsc::Sender<Option<Vec<Command>>>) {
        let (tx, rx) = channel();
        let j = Job {
            key,
            label: "Add Effect",
            cancel: Arc::new(AtomicBool::new(false)),
            rx,
        };
        (j, tx)
    }

    /// At most one preview is made at a time: while a dropped one still
    /// runs, the next one waits (no second full render beside it), and
    /// starts once the dropped one has ended.
    #[test]
    fn a_dropped_preview_still_running_holds_the_next_one_back() {
        let (ctx, mut app, id) = bitmap_app(40, 32);
        crate::ui::effect_dialog::choose_effect(&mut app, "gaussian_blur");
        let (stale, tx) = job(0);
        app.fx_preview.stale = Some(stale);
        frames(&ctx, &mut app, 8, 60);
        assert!(app.fx_preview.job.is_none(), "no second job beside it");
        assert_eq!(app.fx_preview.made, 0);
        assert!(app.fx_preview.busy(), "the preview waits its turn");
        drop(tx);
        assert!(app.settle_fx_preview(&ctx, 10.0));
        assert_eq!(app.fx_preview.made, 1);
        assert_eq!(effects_of(&app, id).len(), 1);
        // Clearing the box drops the job being made: it is kept as the
        // stale one, so the box ticked again waits for it.
        let toggle = |app: &mut App, on: bool| {
            let Dialog::Effect(st) = &mut app.dialog else {
                panic!("the dialog");
            };
            st.preview = on;
        };
        let (running, tx) = job(1);
        app.fx_preview.job = Some(running);
        toggle(&mut app, false);
        frame(&ctx, &mut app, Vec::new());
        assert!(app.fx_preview.job.is_none());
        assert!(app.fx_preview.stale.is_some());
        toggle(&mut app, true);
        frames(&ctx, &mut app, 8, 60);
        assert!(app.fx_preview.job.is_none());
        drop(tx);
        assert!(app.settle_fx_preview(&ctx, 10.0));
        assert_eq!(app.fx_preview.made, 2);
        assert_eq!(effects_of(&app, id).len(), 1);
    }

    /// A preview whose thread died (an effect failed) is not made again
    /// for the same values, frame after frame; a changed value is made.
    #[test]
    fn a_failed_preview_is_not_made_again_until_a_value_changes() {
        let (ctx, mut app, id) = bitmap_app(40, 32);
        crate::ui::effect_dialog::choose_effect(&mut app, "gaussian_blur");
        let key = {
            let Dialog::Effect(st) = &app.dialog else {
                panic!("the dialog");
            };
            preview_key(st, &app.selection)
        };
        let (dead, tx) = job(key);
        drop(tx);
        app.fx_preview.job = Some(dead);
        frames(&ctx, &mut app, 10, 50);
        assert_eq!(app.fx_preview.failed, Some(key));
        assert!(app.fx_preview.job.is_none(), "not started again");
        assert!(!app.fx_preview.busy());
        assert_eq!(app.fx_preview.made, 0);
        assert!(effects_of(&app, id).is_empty());
        set_radius(&mut app, 4.0);
        assert!(app.settle_fx_preview(&ctx, 10.0));
        assert_eq!(app.fx_preview.made, 1);
        assert_eq!(app.fx_preview.failed, None);
        assert_eq!(effects_of(&app, id)[0].params.get("radius"), Some(&4.0));
    }

    /// In the browser the box starts cleared (the work would run on the
    /// interface thread); on the desktop, ticked.
    #[test]
    fn the_preview_box_starts_cleared_in_the_browser() {
        let st = crate::ui::effect_dialog::EffectState::new("gaussian_blur");
        assert_eq!(st.preview, !crate::files::WEB);
    }

    /// `cargo test --release -p tracedraw -- --ignored frame_times_of_a_bitmap_effect_preview --nocapture`:
    /// a 2400 x 1800 bitmap, the Blur dialog open, the radius changed
    /// every frame for a while: the interface frames, with the live
    /// preview on and off, and how long one preview takes on its thread.
    #[test]
    #[ignore]
    fn frame_times_of_a_bitmap_effect_preview() {
        let (ctx, mut app, id) = bitmap_app(2400, 1800);
        let n = 30;
        let mut report = String::new();
        for on in [false, true] {
            crate::ui::effect_dialog::choose_effect(&mut app, "gaussian_blur");
            if let Dialog::Effect(st) = &mut app.dialog {
                st.preview = on;
            }
            frame(&ctx, &mut app, Vec::new());
            let mut worst = std::time::Duration::ZERO;
            let mut total = std::time::Duration::ZERO;
            for i in 0..n {
                set_radius(&mut app, 2.0 + i as f64 * 0.5);
                let t = std::time::Instant::now();
                frame(&ctx, &mut app, Vec::new());
                let d = t.elapsed();
                total += d;
                worst = worst.max(d);
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
            let t = std::time::Instant::now();
            let settled = app.settle_fx_preview(&ctx, 60.0);
            let wait = t.elapsed();
            report.push_str(&format!(
                "live preview {}: {n} frames with the radius changing, {:?} per frame, worst {:?}; previews made {}, last one shown {:?} after the last change (settled {settled})\n",
                if on { "on" } else { "off" },
                total / n,
                worst,
                app.fx_preview.made,
                wait,
            ));
            app.dialog = Dialog::None;
            frame(&ctx, &mut app, Vec::new());
            app.fx_preview.made = 0;
        }
        // One heavy effect on its own, as the thread runs it.
        let t = app.fx_target(id).expect("target");
        let mut blur = super::super::new_effect("gaussian_blur");
        blur.params.insert("radius".into(), 16.0);
        let change = FxChange::Add(blur);
        let cancel = AtomicBool::new(false);
        let t0 = std::time::Instant::now();
        let cmds = fx_commands(vec![t], &change, &[], true, 300.0, &cancel).expect("made");
        report.push_str(&format!(
            "gaussian blur, radius 16, on 2400 x 1800 (render and PNG): {:?} for {} command(s)\n",
            t0.elapsed(),
            cmds.len()
        ));
        eprintln!("{report}");
    }
}
