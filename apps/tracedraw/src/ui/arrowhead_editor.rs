//! The Arrowhead editor (#1210): the "..." beside an end's arrowhead in
//! the Properties docker's outline section and in the Outline Pen dialog
//! opens it on that end. It offers the head picker, Size (a percentage
//! of the preset head size), Offset along the line, Mirror horizontally
//! and vertically, Rotation, and a preview of the line's end with the
//! head. OK sends the head and its placement to the selection as one
//! undo step (through `patch_selection_stroke`), to the default outline
//! with nothing selected, or back into the Outline Pen dialog it was
//! opened from. See docs/behavior/outline-pen.md.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::Dialog;
use crate::ui::tips::RichTip;
use egui::{Context, Sense, Ui, Vec2};
use tracedraw_core::patch::StrokeField;
use tracedraw_core::{Arrowhead, ArrowheadPlacement, Stroke};

/// Where the editor's result goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The selected objects, one undo step.
    Selection,
    /// Characters selected in the text being edited.
    TextSelection,
    /// Nothing selected: the outline new objects get.
    Defaults,
    /// The Outline Pen dialog the editor was opened from (kept in
    /// `App::dialog_return`); the result goes into its outline.
    OutlinePen,
}

/// What the editor keeps while it is open.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowheadEditorState {
    /// The end being edited: the end of the line, or its start.
    pub end: bool,
    /// The outline the head sits on, as it was where the editor opened
    /// (the first object's, or the default outline): the preview draws
    /// it, OK changes only the head and its placement.
    pub old: Option<Stroke>,
    /// The head being edited.
    pub head: Arrowhead,
    /// Its placement.
    pub at: ArrowheadPlacement,
    /// The head, or the placement, differs across the selection.
    pub head_mixed: bool,
    pub at_mixed: bool,
    /// The head, or the placement, was set in the editor (it applies
    /// even when it equals the value shown).
    pub head_set: bool,
    pub at_set: bool,
    pub target: Target,
    /// What the preview picture shows, so it is drawn again only when a
    /// value changes (never every frame).
    pub shown: Option<(Arrowhead, ArrowheadPlacement, f64)>,
}

fn request_id() -> egui::Id {
    egui::Id::new("arrowhead_editor_request")
}

/// Ask for the editor on an end (`end`: the line's end, else its start)
/// from inside a docker or the Outline Pen dialog; opened on the next
/// pass through `dialogs::show`.
pub fn request(ui: &Ui, end: bool) {
    ui.data_mut(|d| d.insert_temp(request_id(), end));
}

/// The end the editor was asked for since the last call, if any.
pub fn take_request(ctx: &Context) -> Option<bool> {
    ctx.data_mut(|d| d.remove_temp::<bool>(request_id()))
}

/// The head and placement of one end of `s`.
fn head_of(s: &Stroke, end: bool) -> (Arrowhead, ArrowheadPlacement) {
    if end {
        (s.end_arrow.clone(), s.end_head)
    } else {
        (s.start_arrow.clone(), s.start_head)
    }
}

/// Give one end of `s` the head and placement; with Share attributes on,
/// the start reaches the end too.
pub(crate) fn set_head(s: &mut Stroke, end: bool, head: Arrowhead, at: ArrowheadPlacement) {
    if end {
        s.end_arrow = head;
        s.end_head = at;
    } else {
        s.start_arrow = head;
        s.start_head = at;
        if s.share_arrowheads {
            s.share_start_head();
        }
    }
}

/// The fields an edit of one end touches.
pub(crate) fn fields_of(end: bool) -> [StrokeField; 2] {
    if end {
        [StrokeField::EndArrow, StrokeField::EndHead]
    } else {
        [StrokeField::StartArrow, StrokeField::StartHead]
    }
}

impl App {
    /// Open the Arrowhead editor on one end of the outline the request
    /// came from: the Outline Pen dialog's outline when that dialog is
    /// open (it comes back after), else the selection's first outline (or
    /// the default outline with nothing selected). Another dialog open:
    /// nothing happens.
    pub fn open_arrowhead_editor(&mut self, end: bool) {
        let (old, target, mixed) = match &self.dialog {
            Dialog::OutlinePen(st) => (
                st.has.then(|| st.stroke.clone()),
                Target::OutlinePen,
                (
                    st.mix.has(fields_of(end)[0]) && !st.touched.contains(&fields_of(end)[0]),
                    st.mix.has(fields_of(end)[1]) && !st.touched.contains(&fields_of(end)[1]),
                ),
            ),
            Dialog::None if self.text_selection_active() => (
                self.text_selection_stroke().unwrap_or(None),
                Target::TextSelection,
                (false, false),
            ),
            Dialog::None => {
                let targets = self.paint_targets();
                match targets.first() {
                    None => (self.default_stroke(), Target::Defaults, (false, false)),
                    Some(first) => {
                        let mix = self.selection_stroke_mix();
                        (
                            first.stroke.clone(),
                            Target::Selection,
                            (mix.has(fields_of(end)[0]), mix.has(fields_of(end)[1])),
                        )
                    }
                }
            }
            _ => return,
        };
        let (head, at) = old
            .as_ref()
            .map_or_else(Default::default, |s| head_of(s, end));
        let st = ArrowheadEditorState {
            end,
            old,
            head,
            at,
            head_mixed: mixed.0,
            at_mixed: mixed.1,
            head_set: false,
            at_set: false,
            target,
            shown: None,
        };
        let editor = Dialog::ArrowheadEditor(Box::new(st));
        if target == Target::OutlinePen {
            let back = std::mem::replace(&mut self.dialog, editor);
            self.dialog_return = Some(Box::new(back));
        } else {
            self.dialog = editor;
        }
    }

    /// OK in the Arrowhead editor: the head and its placement go where
    /// the editor was opened from, one undo step.
    pub fn apply_arrowhead_editor(&mut self, st: &ArrowheadEditorState) {
        let mut touched: Vec<StrokeField> = Vec::new();
        let fields = fields_of(st.end);
        if st.head_set || st.head_mixed {
            touched.push(fields[0]);
        }
        if st.at_set || st.at_mixed {
            touched.push(fields[1]);
        }
        let mut new = st.old.clone().unwrap_or_default();
        set_head(&mut new, st.end, st.head.clone(), st.at);
        match st.target {
            Target::OutlinePen => {
                if let Some(Dialog::OutlinePen(pen)) = self.dialog_return.as_deref_mut() {
                    pen.set_head(st.end, st.head.clone(), st.at);
                }
            }
            Target::Selection => {
                if st.old.is_none() {
                    // The objects had no outline: they get the default one
                    // with the head, in the drawing's black.
                    new.color = self.doc().metadata.primary_color_mode.black();
                }
                self.patch_selection_stroke(&st.old, &Some(new), &touched);
            }
            Target::TextSelection => {
                if Some(&new) != st.old.as_ref() {
                    self.text_apply_outline(Some(new));
                }
            }
            Target::Defaults => {
                if Some(&new) != st.old.as_ref() {
                    self.request_default_change(crate::defaults::DefaultChange::Outline(Some(new)));
                }
            }
        }
    }
}

/// The editor's window.
pub fn dialog(app: &mut App, ctx: &Context, st: &mut ArrowheadEditorState, close: &mut bool) {
    let lib = crate::ui::line_style::OutlineLib::of(app);
    let title = if st.end {
        tr("arrowhead_editor.title_end")
    } else {
        tr("arrowhead_editor.title_start")
    };
    let mut ok = false;
    crate::ui::dialogs::window(ctx, title).show(ctx, |ui| {
        ui.set_min_width(330.0);
        preview(ui, st, &lib);
        ui.add_space(6.0);
        let id = if st.end {
            "arrowhead_editor_end"
        } else {
            "arrowhead_editor_start"
        };
        ui.horizontal(|ui| {
            row_label(ui, "arrowhead_editor.head");
            if crate::ui::dockers::arrow_combo(
                ui,
                id,
                &tr("arrowhead_editor.head"),
                &mut st.head,
                &lib.arrows,
                st.head_mixed && !st.head_set,
            ) {
                st.head_set = true;
            }
        });
        let at_mixed = st.at_mixed && !st.at_set;
        ui.horizontal(|ui| {
            row_label(ui, "arrowhead_editor.size");
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut st.at.size)
                        .range(ArrowheadPlacement::SIZE_RANGE)
                        .speed(1.0)
                        .max_decimals(1)
                        .suffix(" %")
                        .width(70.0)
                        .mixed(at_mixed),
                )
                .rich_tip(tr("arrowhead_editor.size_tip"))
                .changed()
            {
                st.at_set = true;
            }
        });
        ui.horizontal(|ui| {
            row_label(ui, "arrowhead_editor.offset");
            let u = lib.units;
            let mut v = u.from_mm(st.at.offset);
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut v)
                        .range(u.from_mm(-1000.0)..=u.from_mm(1000.0))
                        .unit_mm(u.mm())
                        .max_decimals(3)
                        .width(70.0)
                        .mixed(at_mixed),
                )
                .rich_tip(tr("arrowhead_editor.offset_tip"))
                .changed()
            {
                st.at.offset = u.to_mm(v).clamp(-1000.0, 1000.0);
                st.at_set = true;
            }
            if let Some(nu) =
                crate::ui::propbar::outline_unit_combo(ui, "arrowhead_editor_units", u)
            {
                crate::ui::line_style::request_units(ui, nu);
            }
        });
        ui.horizontal(|ui| {
            for (v, key) in [
                (&mut st.at.mirror_h, "arrowhead_editor.mirror_h"),
                (&mut st.at.mirror_v, "arrowhead_editor.mirror_v"),
            ] {
                if ui
                    .add(egui::Checkbox::new(v, tr(key)).indeterminate(at_mixed))
                    .rich_tip(tr(&format!("{key}_tip")))
                    .changed()
                {
                    st.at_set = true;
                }
            }
        });
        ui.horizontal(|ui| {
            row_label(ui, "arrowhead_editor.rotation");
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut st.at.rotation)
                        .range(-360.0..=360.0)
                        .speed(1.0)
                        .max_decimals(1)
                        .suffix("°")
                        .width(70.0)
                        .mixed(at_mixed),
                )
                .rich_tip(tr("arrowhead_editor.rotation_tip"))
                .changed()
            {
                st.at_set = true;
            }
            if ui
                .button(tr("outline_pen.default"))
                .rich_tip(tr("arrowhead_editor.default_tip"))
                .clicked()
            {
                st.at = ArrowheadPlacement::default();
                st.at_set = true;
            }
        });
        ok = crate::ui::dialogs::ok_cancel(ui, close);
    });
    if ok {
        app.apply_arrowhead_editor(st);
    }
}

fn row_label(ui: &mut Ui, key: &str) {
    ui.add_sized(
        [70.0, 20.0],
        egui::Label::new(egui::RichText::new(tr(key)).color(Tokens::text())),
    );
}

fn preview_id() -> egui::Id {
    egui::Id::new("arrowhead_editor_preview")
}

/// The preview's size in points.
const PREVIEW: Vec2 = Vec2::new(300.0, 90.0);

/// The outline the preview draws: the one the editor opened on (or a
/// 1 mm black line) with the edited head on its end and nothing on the
/// other.
fn preview_stroke(st: &ArrowheadEditorState) -> Stroke {
    let mut s = st
        .old
        .clone()
        .unwrap_or_else(|| Stroke::new(tracedraw_core::Color::BLACK, 1.0));
    s.start_arrow = Arrowhead::None;
    s.end_arrow = Arrowhead::None;
    s.start_head = ArrowheadPlacement::default();
    s.end_head = ArrowheadPlacement::default();
    s.share_arrowheads = false;
    s.widths.clear();
    s.painterly = None;
    set_head(&mut s, st.end, st.head.clone(), st.at);
    s
}

/// A picture of the line's end with the head, drawn by the renderer so it
/// shows exactly what the page will; made again only when the head, its
/// placement or the outline changes.
fn preview(ui: &mut Ui, st: &mut ArrowheadEditorState, lib: &crate::ui::line_style::OutlineLib) {
    let (rect, _) = ui.allocate_exact_size(PREVIEW, Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 2.0, egui::Color32::WHITE);
    p.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let stroke = preview_stroke(st);
    let now = (st.head.clone(), st.at, stroke.width);
    let cached = ui
        .ctx()
        .data(|d| d.get_temp::<egui::TextureHandle>(preview_id()));
    let tex = match cached {
        Some(t) if st.shown.as_ref() == Some(&now) => Some(t),
        _ => {
            let ppp = ui.ctx().pixels_per_point().clamp(1.0, 4.0);
            let size = [(PREVIEW.x * ppp) as u32, (PREVIEW.y * ppp) as u32];
            let tex = render_preview(ui.ctx(), &stroke, st.end, size, lib.black);
            st.shown = Some(now);
            if let Some(t) = &tex {
                let t = t.clone();
                ui.ctx().data_mut(|d| d.insert_temp(preview_id(), t));
            }
            tex
        }
    };
    if let Some(t) = tex {
        ui.painter().image(
            t.id(),
            rect.shrink(1.0),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
}

/// The preview's drawing: a horizontal line with `stroke`, its edited end
/// (and the head there) fitted into `size` pixels with the line running
/// in from the other side.
fn render_preview(
    ctx: &Context,
    stroke: &Stroke,
    end: bool,
    size: [u32; 2],
    black: tracedraw_core::Color,
) -> Option<egui::TextureHandle> {
    use tracedraw_core::geometry::{BezPath, Rect, Shape as _};
    use tracedraw_core::{Document, Shape, ShapeKind};
    let mut doc = Document::default();
    let layer = doc.pages.first()?.layers.first()?.id;
    // The line sits inside the page; the head's size decides how much of
    // it the picture shows.
    let (head_mm, _) = tracedraw_core::style::head_size_for_width(stroke.width);
    let reach = head_mm * 3.0 + stroke.width;
    let y = 100.0;
    let (x0, x1) = (20.0, 20.0 + reach.max(10.0) * 4.0);
    let mut path = BezPath::new();
    path.move_to((x0, y));
    path.line_to((x1, y));
    let id = doc.ids_mut().shape();
    let mut line = Shape::new(
        id,
        ShapeKind::Path {
            path: path.clone(),
            closed: false,
        },
    );
    line.fill = tracedraw_core::Fill::None;
    let mut s = stroke.clone();
    if s.color.to_rgb8() == [255, 255, 255] {
        s.color = black;
    }
    line.stroke = Some(s.clone());
    doc.layer_mut(layer).ok()?.shapes.push(line);
    // What to show: the edited end with the head and some line before it.
    let mut bounds = Rect::new(
        if end { x1 - reach } else { x0 },
        y - head_mm,
        if end { x1 } else { x0 + reach },
        y + head_mm,
    );
    for head in tracedraw_core::arrowhead_paths(&path, &s) {
        bounds = bounds.union(head.bounding_box());
    }
    let margin = 6.0;
    let w = size[0].max(1) as f64;
    let h = size[1].max(1) as f64;
    let zoom = ((w - 2.0 * margin) / bounds.width().max(1e-6))
        .min((h - 2.0 * margin) / bounds.height().max(1e-6));
    if !(zoom.is_finite() && zoom > 0.0) {
        return None;
    }
    // Centred in the picture.
    let origin_x = (w - bounds.width() * zoom) / 2.0 - bounds.x0 * zoom;
    let origin_y = (h - bounds.height() * zoom) / 2.0 + bounds.y1 * zoom;
    let opts = tracedraw_render::RenderOptions {
        width: size[0].max(1),
        height: size[1].max(1),
        view: tracedraw_render::ViewTransform {
            zoom,
            origin_x,
            origin_y,
        },
        ..tracedraw_render::RenderOptions::default()
    };
    let page = doc.pages.first()?.id;
    let pm = tracedraw_render::render_page(&doc, page, &opts)?;
    let image = egui::ColorImage::from_rgba_premultiplied(
        [pm.width() as usize, pm.height() as usize],
        pm.data(),
    );
    Some(ctx.load_texture(
        "arrowhead_editor_preview",
        image,
        egui::TextureOptions::LINEAR,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::BezPath;
    use tracedraw_core::{Color, ShapeKind};

    fn line(app: &mut App) -> tracedraw_core::ShapeId {
        let mut p = BezPath::new();
        p.move_to((10.0, 50.0));
        p.line_to((60.0, 50.0));
        let id = app
            .new_shape(ShapeKind::Path {
                path: p,
                closed: false,
            })
            .expect("line");
        app.select(vec![id]);
        app.apply_outline_color(Some(Color::rgb8(0, 0, 255)));
        id
    }

    fn stroke_of(app: &App, id: tracedraw_core::ShapeId) -> Stroke {
        app.doc()
            .find_shape(id)
            .and_then(|s| s.stroke.clone())
            .expect("outline")
    }

    fn take_editor(app: &mut App) -> Box<ArrowheadEditorState> {
        let Dialog::ArrowheadEditor(st) = std::mem::replace(&mut app.dialog, Dialog::None) else {
            panic!("the editor: {:?}", app.dialog);
        };
        st
    }

    /// Every text the dialog painted in one frame.
    fn painted(ctx: &egui::Context, app: &mut App, st: &mut ArrowheadEditorState) -> Vec<String> {
        fn walk(s: &egui::Shape, out: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => out.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1200.0),
            )),
            ..Default::default()
        };
        let mut close = false;
        let mut out = ctx.run_ui(input, |ui| {
            let c = ui.ctx().clone();
            dialog(app, &c, st, &mut close);
        });
        out.textures_delta.clear();
        let mut texts = Vec::new();
        for c in &out.shapes {
            walk(&c.shape, &mut texts);
        }
        texts
    }

    /// The editor opens on the selection's end head; OK sends the head
    /// and its placement to the selection as one undo step and leaves
    /// the rest of the outline alone (#1210).
    #[test]
    fn the_editor_sets_one_end_of_the_selection_as_one_step() {
        let mut app = App::headless();
        let a = line(&mut app);
        let b = line(&mut app);
        app.select(vec![a, b]);
        app.open_arrowhead_editor(true);
        let mut st = take_editor(&mut app);
        assert_eq!((st.end, st.target), (true, Target::Selection));
        assert_eq!(st.head, Arrowhead::None);
        assert!(st.at.is_default());
        st.head = Arrowhead::Arrow;
        st.head_set = true;
        st.at.size = 200.0;
        st.at.offset = 2.0;
        st.at.mirror_v = true;
        st.at.rotation = 15.0;
        st.at_set = true;
        let steps = app.engine.history_labels().0.len();
        app.apply_arrowhead_editor(&st);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        for id in [a, b] {
            let s = stroke_of(&app, id);
            assert_eq!(s.end_arrow, Arrowhead::Arrow);
            assert_eq!(s.end_head.size, 200.0);
            assert_eq!(s.end_head.offset, 2.0);
            assert!(s.end_head.mirror_v && !s.end_head.mirror_h);
            assert_eq!(s.end_head.rotation, 15.0);
            assert_eq!(s.start_arrow, Arrowhead::None, "the start stays");
            assert_eq!(s.color, Color::rgb8(0, 0, 255), "the colour stays");
        }
        // The start, with Share attributes on: the end follows it.
        let mut shared = stroke_of(&app, a);
        shared.share_arrowheads = true;
        app.run(tracedraw_core::Command::SetStroke {
            shapes: vec![a],
            stroke: Some(shared),
        });
        app.select(vec![a]);
        app.open_arrowhead_editor(false);
        let mut st = take_editor(&mut app);
        st.head = Arrowhead::Diamond;
        st.head_set = true;
        st.at.size = 50.0;
        st.at_set = true;
        app.apply_arrowhead_editor(&st);
        let s = stroke_of(&app, a);
        assert_eq!(s.start_arrow, Arrowhead::Diamond);
        assert_eq!(s.end_arrow, Arrowhead::Diamond);
        assert_eq!(s.end_head, s.start_head);
        assert_eq!(s.start_head.size, 50.0);
        // Nothing changed: no step.
        let steps = app.engine.history_labels().0.len();
        app.open_arrowhead_editor(false);
        let st = take_editor(&mut app);
        app.apply_arrowhead_editor(&st);
        assert_eq!(app.engine.history_labels().0.len(), steps);
    }

    /// Opened from the Outline Pen dialog, the editor's OK goes back into
    /// that dialog's outline (marked as set), and the dialog returns.
    /// With nothing selected it edits the default outline.
    #[test]
    fn the_editor_returns_its_result_to_the_outline_pen_and_the_defaults() {
        let mut app = App::headless();
        let a = line(&mut app);
        app.select(vec![a]);
        app.open_outline_pen();
        app.open_arrowhead_editor(false);
        let mut st = take_editor(&mut app);
        assert_eq!(st.target, Target::OutlinePen);
        assert!(matches!(
            app.dialog_return.as_deref(),
            Some(Dialog::OutlinePen(_))
        ));
        st.head = Arrowhead::Circle;
        st.head_set = true;
        st.at.offset = -1.5;
        st.at_set = true;
        app.apply_arrowhead_editor(&st);
        let Some(Dialog::OutlinePen(pen)) = app.dialog_return.as_deref() else {
            panic!("the Outline Pen dialog");
        };
        assert_eq!(pen.stroke.start_arrow, Arrowhead::Circle);
        assert_eq!(pen.stroke.start_head.offset, -1.5);
        assert!(pen.touched.contains(&StrokeField::StartArrow));
        assert!(pen.touched.contains(&StrokeField::StartHead));
        assert_eq!(
            stroke_of(&app, a).start_arrow,
            Arrowhead::None,
            "the drawing waits for the Outline Pen's OK"
        );
        app.dialog_return = None;
        // Nothing selected: the default outline.
        app.select(Vec::new());
        app.settings.defaults.ask = false;
        app.open_arrowhead_editor(true);
        let mut st = take_editor(&mut app);
        assert_eq!(st.target, Target::Defaults);
        st.head = Arrowhead::Bar;
        st.head_set = true;
        app.apply_arrowhead_editor(&st);
        assert_eq!(
            app.default_stroke().map(|s| s.end_arrow),
            Some(Arrowhead::Bar)
        );
        // A request from a docker is taken once.
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput::default();
        let mut out = ctx.run_ui(input, |ui| request(ui, true));
        out.textures_delta.clear();
        assert_eq!(take_request(&ctx), Some(true));
        assert_eq!(take_request(&ctx), None);
    }

    /// The dialog shows its fields and the preview is drawn again only
    /// when a value changes.
    #[test]
    fn the_dialog_shows_its_fields_and_draws_the_preview_once() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let a = line(&mut app);
        app.select(vec![a]);
        app.open_arrowhead_editor(true);
        let mut st = take_editor(&mut app);
        st.head = Arrowhead::Arrow;
        // The window settles over the first frames.
        for _ in 0..3 {
            painted(&ctx, &mut app, &mut st);
        }
        let texts = painted(&ctx, &mut app, &mut st);
        for key in [
            "arrowhead_editor.size",
            "arrowhead_editor.offset",
            "arrowhead_editor.rotation",
            "arrowhead_editor.mirror_h",
            "arrowhead_editor.mirror_v",
            "arrowhead_editor.title_end",
        ] {
            let want = tr(key);
            assert!(texts.iter().any(|t| *t == want), "{want}: {texts:?}");
        }
        let shown = st.shown.clone();
        assert!(shown.is_some(), "the preview was drawn");
        let tex = ctx.data(|d| d.get_temp::<egui::TextureHandle>(preview_id()));
        assert!(tex.is_some());
        painted(&ctx, &mut app, &mut st);
        assert_eq!(st.shown, shown, "unchanged values: nothing drawn again");
        st.at.size = 300.0;
        painted(&ctx, &mut app, &mut st);
        assert_ne!(st.shown, shown);
        // The preview's outline: the selection's, with the edited head on
        // the edited end only.
        let s = preview_stroke(&st);
        assert_eq!(s.end_arrow, Arrowhead::Arrow);
        assert_eq!(s.end_head.size, 300.0);
        assert_eq!(s.start_arrow, Arrowhead::None);
        assert_eq!(s.color, Color::rgb8(0, 0, 255));
    }
}
