//! The colour picker of the Properties docker's colour rows: a swatch
//! that opens a popup beside it with the colour viewer (saturation and
//! brightness square, hue strip) and the sliders of the colour's own
//! model. The popup opens next to the swatch, on its right when there is
//! room and else on its left, and always stays inside the window. The
//! colour keeps its model: a CMYK colour is edited in CMYK, an RGB one in
//! RGB, and a Model list switches it on demand (the colour converts and
//! the object holds a colour of the new model). A click outside the
//! popup or Esc closes it. The dotted strip at the popup's top drags it
//! elsewhere (off the object it colours), and it stays there while open.

use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use tracedraw_core::Color;

/// Gap between the swatch and the popup, and the popup's margin from the
/// window edges (points).
const GAP: f32 = 4.0;
const MARGIN: f32 = 4.0;
/// The popup's size before it has been measured once.
const FIRST_SIZE: Vec2 = Vec2::new(250.0, 330.0);
/// The width the popup's contents get.
const CONTENT_WIDTH: f32 = 232.0;

/// What a picker keeps while it is open.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Open {
    /// The hue the viewer shows, kept while the colour has none (greys).
    hue: f32,
    /// The popup's size, as measured on the frame before.
    size: Vec2,
    /// How far the popup was dragged by its strip from where it opened.
    offset: Vec2,
}

/// A dotted strip `width` points wide that drags what it sits on: the
/// distance the pointer moved over it this frame.
pub fn drag_strip(ui: &mut Ui, size: Vec2) -> Vec2 {
    let (r, resp) = ui.allocate_exact_size(size, Sense::drag());
    let resp = resp.on_hover_cursor(egui::CursorIcon::Grab);
    let c = if resp.hovered() || resp.dragged() {
        Tokens::text()
    } else {
        Tokens::text_dim()
    };
    let p = ui.painter();
    let (cols, rows) = if size.x >= size.y {
        (((size.x - 4.0) / 5.0).floor().clamp(1.0, 12.0) as i32, 2)
    } else {
        (2, ((size.y - 4.0) / 5.0).floor().clamp(1.0, 12.0) as i32)
    };
    let step = 4.0;
    let start = r.center() - Vec2::new((cols - 1) as f32, (rows - 1) as f32) * step / 2.0;
    for i in 0..cols {
        for j in 0..rows {
            p.circle_filled(start + Vec2::new(i as f32, j as f32) * step, 1.0, c);
        }
    }
    if resp.dragged() {
        resp.drag_delta()
    } else {
        Vec2::ZERO
    }
}

/// The swatch and popup of the picker shown last, for checks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shown {
    pub swatch: Rect,
    pub popup: Rect,
}

fn last_id() -> Id {
    Id::new("tracedraw.color_picker.last")
}

/// The picker that was open on the last frame (its swatch and popup).
#[cfg(test)]
pub fn last_shown(ctx: &egui::Context) -> Option<Shown> {
    ctx.data(|d| d.get_temp::<Shown>(last_id()))
}

/// Where a popup of `size` opens beside the swatch `anchor` inside the
/// window `screen`: to the right of the swatch when it fits there, else
/// to its left, else as close as the window allows; its top level with
/// the swatch's, moved up or down to stay inside the window.
pub fn place(anchor: Rect, size: Vec2, screen: Rect) -> Rect {
    let lo_x = screen.left() + MARGIN;
    let hi_x = (screen.right() - MARGIN - size.x).max(lo_x);
    let right = anchor.right() + GAP;
    let left = anchor.left() - GAP - size.x;
    let x = if right <= hi_x {
        right
    } else if left >= lo_x {
        left
    } else {
        right.clamp(lo_x, hi_x)
    };
    let lo_y = screen.top() + MARGIN;
    let hi_y = (screen.bottom() - MARGIN - size.y).max(lo_y);
    let y = anchor.top().clamp(lo_y, hi_y);
    Rect::from_min_size(Pos2::new(x, y), size)
}

/// The model a colour is edited in: its own, or CMYK for a spot ink or
/// the registration colour (which have no sliders of their own).
pub fn edit_model(c: Color) -> &'static str {
    match c {
        Color::Spot { .. } | Color::Registration => "CMYK",
        other => other.model_name(),
    }
}

/// The Model list's entry that makes the colour the registration colour
/// (it prints on every separation), after the models.
pub const REGISTRATION: &str = "Registration";

/// The entry the Model list shows as chosen for `c`.
pub fn list_model(c: Color) -> &'static str {
    match c {
        Color::Registration => REGISTRATION,
        other => edit_model(other),
    }
}

/// `c` after `choice` is picked in the Model list: the colour converted
/// to that model, or the registration colour.
pub fn choose_model(c: Color, choice: &str) -> Color {
    if choice == REGISTRATION {
        Color::Registration
    } else {
        c.convert_to(choice)
    }
}

/// The Model list (the docker's models, then the registration colour).
/// True when a choice changed the colour.
pub fn model_list(ui: &mut Ui, id: Id, c: &mut Color) -> bool {
    let shown = list_model(*c);
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(if shown == REGISTRATION {
            tr("docker.registration")
        } else {
            shown.to_string()
        })
        .width(96.0)
        .show_ui(ui, |ui| {
            for m in crate::ui::color_docker::MODELS {
                if ui.selectable_label(shown == m, m).clicked() && shown != m {
                    *c = choose_model(*c, m);
                    changed = true;
                }
            }
            ui.separator();
            let reg = shown == REGISTRATION;
            if ui
                .selectable_label(reg, tr("docker.registration"))
                .rich_tip(tr("docker.registration_tip"))
                .clicked()
                && !reg
            {
                *c = choose_model(*c, REGISTRATION);
                changed = true;
            }
        });
    changed
}

/// A colour chosen in the viewer (hue, saturation, brightness) as a
/// colour of `model`. In CMYK the square spans the ink space itself
/// (#907): left to right the hue's inks go from none to full, top to
/// bottom black ink goes from 0 to 100%, so every corner is a CMYK
/// extreme (paper, the hue's solid inks, K 100, the hue's inks over
/// K 100) instead of a screen colour the press cannot print.
pub fn from_viewer(h: f32, s: f32, b: f32, model: &str) -> Color {
    if model == "CMYK" {
        let [c, m, y] = hue_inks(h);
        let s = unit(s);
        Color::Cmyk {
            c: c * s,
            m: m * s,
            y: y * s,
            k: 1.0 - unit(b),
        }
    } else {
        Color::Hsb { h, s, b }.convert_to(model)
    }
}

/// `v` within 0..1 (0 for NaN).
fn unit(v: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

/// The C, M and Y inks of hue `h` at full strength: the complement of
/// the hue's pure screen colour, so one ink is solid, one is off and the
/// third blends between hues.
fn hue_inks(h: f32) -> [f32; 3] {
    let h = if h.is_finite() { h } else { 0.0 };
    tracedraw_core::color::hsb_to_rgb(h.rem_euclid(360.0), 1.0, 1.0).map(|v| 1.0 - unit(v))
}

/// Where the viewer shows `c` (hue, saturation, brightness), the way
/// back of [`from_viewer`] for the colour's edit model: in CMYK its inks
/// (black ink sets the height, the strongest of C, M and Y the width,
/// their balance the hue), else its screen look.
pub fn viewer_point(c: Color) -> (f32, f32, f32) {
    if edit_model(c) == "CMYK" {
        if let Color::Cmyk { c, m, y, k } = c.process().convert_to("CMYK") {
            let (c, m, y) = (unit(c), unit(m), unit(y));
            let s = c.max(m).max(y);
            let (h, _, _) = tracedraw_core::color::rgb_to_hsb(1.0 - c, 1.0 - m, 1.0 - y);
            return (h, s, 1.0 - unit(k));
        }
    }
    let (h, s, b) = c.to_hsb();
    (h as f32, s as f32, b as f32)
}

/// Whether the popup closes: on Esc, or on a press outside it and its
/// swatch. While its Model list is open a press is not outside: the list
/// opens below the popup and reaches past it, and a press on its lower
/// entries (Lab, Grayscale, YIQ) is a choice (#440).
fn closes(pressed_at: Option<Pos2>, esc: bool, popup: Rect, swatch: Rect, list_open: bool) -> bool {
    esc || (!list_open && pressed_at.is_some_and(|p| !popup.contains(p) && !swatch.contains(p)))
}

/// The picker waiting for a click on the drawing to give it a colour
/// (its eyedropper was pressed, #814).
fn pick_request_id() -> Id {
    Id::new("color_pick_request")
}

/// The colour a click on the drawing gave, with the picker that asked.
fn pick_result_id() -> Id {
    Id::new("color_pick_result")
}

/// A picker's eyedropper is waiting for a click on the drawing.
pub fn picking(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Id>(pick_request_id()).is_some())
}

/// Picker `id` waits for a click on the drawing to give it a colour.
pub fn start_pick(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|d| d.insert_temp(pick_request_id(), id));
}

/// The click on the drawing gave `c` (none: the pick was cancelled); the
/// picker that asked takes it when it is next drawn.
pub fn finish_pick(ctx: &egui::Context, c: Option<Color>) {
    ctx.data_mut(|d| {
        let asked = d.get_temp::<Id>(pick_request_id());
        d.remove::<Id>(pick_request_id());
        if let (Some(id), Some(c)) = (asked, c) {
            d.insert_temp(pick_result_id(), (id, c));
        }
    });
}

/// The colour picked from the drawing for the picker `id`, once.
fn take_pick(ui: &Ui, id: Id) -> Option<Color> {
    ui.data_mut(|d| match d.get_temp::<(Id, Color)>(pick_result_id()) {
        Some((asked, c)) if asked == id => {
            d.remove::<(Id, Color)>(pick_result_id());
            Some(c)
        }
        _ => None,
    })
}

/// The eyedropper of picker `id`: pressed, the next click on the drawing
/// gives `c` the colour there (Esc cancels); pressed again, it stops
/// waiting (#814). True when `c` changed.
pub fn eyedropper(ui: &mut Ui, id: Id, c: &mut Color) -> bool {
    let mut changed = false;
    if let Some(p) = take_pick(ui, id) {
        *c = p;
        changed = true;
    }
    let waiting = ui.data(|d| d.get_temp::<Id>(pick_request_id())) == Some(id);
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
    if waiting {
        ui.painter().rect_filled(r, 2.0, Tokens::tool_active());
    } else if resp.hovered() {
        ui.painter().rect_filled(r, 2.0, Tokens::tool_hover());
    }
    crate::ui::icons::draw(
        ui.painter(),
        r.shrink(3.0),
        crate::tools::Tool::ColorEyedropper,
        Tokens::icon(),
    );
    if resp
        .rich_tip(crate::tools::Tool::ColorEyedropper.name())
        .clicked()
    {
        if waiting {
            ui.data_mut(|d| d.remove::<Id>(pick_request_id()));
        } else {
            start_pick(ui.ctx(), id);
        }
    }
    changed
}

/// A colour swatch that opens the picker. True when the colour changed.
pub fn color_button(ui: &mut Ui, c: &mut Color) -> bool {
    let (swatch, resp) = ui.allocate_exact_size(Vec2::new(40.0, 18.0), Sense::click());
    let resp = resp.rich_tip(crate::ui::tips::keyed("color_picker.button"));
    let id = resp.id.with("color_picker");
    let [r, g, b] = c.to_rgb8();
    ui.painter()
        .rect_filled(swatch.shrink(1.0), 2.0, Color32::from_rgb(r, g, b));
    ui.painter().rect_stroke(
        swatch,
        2.0,
        Stroke::new(1.0, Tokens::control_border()),
        egui::StrokeKind::Inside,
    );
    let mut open = ui.data(|d| d.get_temp::<Open>(id));
    // A colour its eyedropper asked for, picked from the drawing.
    let mut changed = false;
    if let Some(p) = take_pick(ui, id) {
        *c = p;
        changed = true;
    }
    let was_picking = picking(ui.ctx());
    if resp.clicked() {
        open = match open {
            Some(_) => None,
            None => Some(Open {
                hue: viewer_point(*c).0,
                size: FIRST_SIZE,
                offset: Vec2::ZERO,
            }),
        };
    }
    if let Some(mut st) = open {
        let screen = ui.ctx().content_rect();
        let placed = place(swatch, st.size, screen);
        // Dragged by its strip, kept inside the window.
        let lo = screen.min + Vec2::splat(MARGIN);
        let hi = (screen.max - st.size - Vec2::splat(MARGIN)).max(lo);
        let rect = Rect::from_min_size((placed.min + st.offset).clamp(lo, hi), st.size);
        st.offset = rect.min - placed.min;
        let model = edit_model(*c);
        let area = egui::Area::new(id.with("area"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(CONTENT_WIDTH);
                    st.offset += drag_strip(ui, Vec2::new(CONTENT_WIDTH, 8.0));
                    if let Some((h, s, b)) = crate::ui::color_docker::viewer(ui, st.hue, *c) {
                        st.hue = h;
                        *c = from_viewer(h, s, b, model);
                        changed = true;
                    }
                    ui.add_space(6.0);
                    // The model the colour is kept in, changed on demand:
                    // the colour converts, and from then on the object
                    // holds a colour of that model.
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(tr("docker.model"))
                                .color(Tokens::text_dim())
                                .size(11.0),
                        );
                        changed |= model_list(ui, id.with("model"), c);
                        changed |= eyedropper(ui, id, c);
                    });
                    let model = edit_model(*c);
                    let mut m = c.convert_to(model);
                    if crate::ui::color_docker::sliders(ui, &mut m) {
                        *c = m;
                        changed = true;
                    }
                    ui.label(
                        egui::RichText::new(crate::app::color_description(*c))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                });
            });
        let popup = area.response.rect;
        st.size = popup.size();
        // A press outside the popup and its swatch, or Esc, closes it.
        let (pressed_at, esc) = ui.input(|i| {
            (
                i.pointer
                    .any_pressed()
                    .then(|| i.pointer.interact_pos())
                    .flatten(),
                i.key_pressed(egui::Key::Escape),
            )
        });
        let list_open = egui::Popup::is_any_open(ui.ctx());
        // The click that picks a colour from the drawing keeps it open.
        let pressed_at = pressed_at.filter(|_| !was_picking);
        open = (!closes(pressed_at, esc && !was_picking, popup, swatch, list_open)).then_some(st);
        ui.data_mut(|d| {
            d.insert_temp(last_id(), Shown { swatch, popup });
        });
    }
    ui.data_mut(|d| match open {
        Some(st) => {
            d.insert_temp(id, st);
        }
        None => d.remove::<Open>(id),
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 900.0))
    }

    #[test]
    fn the_popup_opens_beside_its_swatch_inside_the_window() {
        let size = Vec2::new(250.0, 330.0);
        // Room on the right: it opens there, level with the swatch.
        let swatch = Rect::from_min_size(Pos2::new(300.0, 200.0), Vec2::new(40.0, 18.0));
        let r = place(swatch, size, screen());
        assert_eq!(r.min, Pos2::new(344.0, 200.0));
        // A swatch near the right edge (a docker there): on its left,
        // never past the window's edge.
        let swatch = Rect::from_min_size(Pos2::new(1300.0, 200.0), Vec2::new(40.0, 18.0));
        let r = place(swatch, size, screen());
        assert_eq!(r.max.x, 1296.0);
        assert!(r.right() <= swatch.left());
        assert!(screen().contains_rect(r));
        // Near the bottom: moved up into the window, still beside it (it
        // covers nothing below the swatch's own row on its column).
        let swatch = Rect::from_min_size(Pos2::new(1300.0, 800.0), Vec2::new(40.0, 18.0));
        let r = place(swatch, size, screen());
        assert!(screen().contains_rect(r));
        assert!(r.right() <= swatch.left());
        assert_eq!(r.bottom(), 896.0);
        // A window too narrow for either side: kept inside it.
        let small = Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 900.0));
        let swatch = Rect::from_min_size(Pos2::new(150.0, 100.0), Vec2::new(40.0, 18.0));
        let r = place(swatch, size, small);
        assert!(r.left() >= 4.0 && r.right() <= 296.0, "{r:?}");
    }

    /// The popup's model list offers every model the Colour docker does,
    /// and a colour switched to one of them is kept in it from then on,
    /// edited there (#286).
    #[test]
    fn the_model_can_be_changed_on_demand() {
        let red = Color::rgb8(220, 30, 30);
        for m in crate::ui::color_docker::MODELS {
            let c = red.convert_to(m);
            assert_eq!(c.model_name(), m);
            assert_eq!(edit_model(c), m, "edited in the model chosen");
            // Still the same colour to the eye (Grayscale keeps the
            // lightness only, by its nature).
            let [r, g, b] = c.to_rgb8();
            if m == "Grayscale" {
                assert!(r == g && g == b && r > 60 && r < 160, "{m}: {r} {g} {b}");
            } else {
                assert!(
                    (r as i32 - 220).abs() < 8
                        && (g as i32 - 30).abs() < 8
                        && (b as i32 - 30).abs() < 8,
                    "{m}: {r} {g} {b}"
                );
            }
        }
        assert!(crate::ui::color_docker::MODELS.contains(&"CMYK"));
        assert!(crate::ui::color_docker::MODELS.contains(&"RGB"));
    }

    #[test]
    fn a_colour_is_edited_in_its_own_model() {
        let red_cmyk = Color::cmyk_pct(0.0, 100.0, 100.0, 0.0);
        assert_eq!(edit_model(red_cmyk), "CMYK");
        let (h, s, b) = red_cmyk.to_hsb();
        let c = from_viewer(h as f32, s as f32 * 0.5, b as f32, edit_model(red_cmyk));
        assert!(matches!(c, Color::Cmyk { .. }), "{c:?}");
        let rgb = Color::rgb8(10, 20, 30);
        assert!(matches!(
            from_viewer(10.0, 0.5, 0.5, edit_model(rgb)),
            Color::Rgb { .. }
        ));
        assert_eq!(edit_model(Color::Registration), "CMYK");
    }

    /// In CMYK the viewer spans the inks (#907): the way back of every
    /// point gives the point again, so the ring sits where it was put
    /// and can reach the edges and corners.
    #[test]
    fn the_cmyk_viewer_round_trips_its_points() {
        for h in [0.0f32, 30.0, 90.0, 180.0, 250.0, 330.0] {
            for s in [0.0f32, 0.25, 1.0] {
                for b in [0.0f32, 0.4, 1.0] {
                    let c = from_viewer(h, s, b, "CMYK");
                    let Color::Cmyk { c: cc, m, y, k } = c else {
                        panic!("{c:?}");
                    };
                    assert!(cc + m + y + k <= 3.0 + 1e-5, "ink limit {c:?}");
                    let (h2, s2, b2) = viewer_point(c);
                    assert!(
                        (s2 - s).abs() < 1e-4 && (b2 - b).abs() < 1e-4,
                        "{h} {s} {b}: {s2} {b2}"
                    );
                    if s > 0.0 {
                        let dh = (h2 - h).rem_euclid(360.0);
                        assert!(dh.min(360.0 - dh) < 0.1, "{h} {s} {b}: {h2}");
                    }
                }
            }
        }
        // A spot ink is shown by its process inks.
        let spot = Color::Spot {
            ink: 1,
            tint: 1.0,
            c: 0.0,
            m: 1.0,
            y: 1.0,
            k: 0.0,
        };
        let (_, s, b) = viewer_point(spot);
        assert!((s - 1.0).abs() < 1e-4 && (b - 1.0).abs() < 1e-4, "{s} {b}");
        // Nonsense from a file does not panic.
        let odd = from_viewer(f32::NAN, f32::NAN, f32::INFINITY, "CMYK");
        assert!(matches!(odd, Color::Cmyk { .. }));
    }

    /// The Model list offers the registration colour after the models, as
    /// the reference program's list does; it shows as chosen for that
    /// colour, and a model chosen from it converts it like any colour
    /// (#286).
    #[test]
    fn the_model_list_offers_the_registration_colour() {
        let red = Color::rgb8(220, 30, 30);
        let reg = choose_model(red, REGISTRATION);
        assert_eq!(reg, Color::Registration);
        assert_eq!(list_model(reg), REGISTRATION);
        assert_eq!(list_model(red), "RGB");
        let back = choose_model(reg, "CMYK");
        assert!(matches!(back, Color::Cmyk { .. }), "{back:?}");
        assert_eq!(list_model(back), "CMYK");
        assert!(!crate::ui::color_docker::MODELS.contains(&REGISTRATION));
    }

    /// A press below the popup closes it, unless the Model list is open
    /// there: its lower entries reach past the popup (#440).
    #[test]
    fn a_choice_in_the_model_list_keeps_the_popup_open() {
        let popup = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(250.0, 330.0));
        let swatch = Rect::from_min_size(Pos2::new(50.0, 100.0), Vec2::new(40.0, 18.0));
        let below = Some(Pos2::new(150.0, 450.0));
        assert!(closes(below, false, popup, swatch, false));
        assert!(!closes(below, false, popup, swatch, true));
        assert!(!closes(
            Some(Pos2::new(150.0, 200.0)),
            false,
            popup,
            swatch,
            false
        ));
        assert!(closes(None, true, popup, swatch, true), "Esc always closes");
    }

    /// A click on a swatch near the window's right edge opens the popup
    /// on its left, inside the window, and the swatch's colour stays CMYK.
    #[test]
    fn a_swatch_click_opens_the_popup_inside_the_window() {
        let ctx = crate::theme::ui_context();
        let mut c = Color::cmyk_pct(0.0, 100.0, 100.0, 0.0);
        let at = Pos2::new(1340.0, 209.0);
        let run = |ctx: &egui::Context, c: &mut Color, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen()),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                egui::Area::new(Id::new("swatch_area"))
                    .fixed_pos(Pos2::new(1320.0, 200.0))
                    .show(ui.ctx(), |ui| color_button(ui, c));
            });
            out.textures_delta.clear();
        };
        run(&ctx, &mut c, Vec::new());
        run(&ctx, &mut c, vec![egui::Event::PointerMoved(at)]);
        run(&ctx, &mut c, vec![egui::Event::PointerMoved(at)]);
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(&ctx, &mut c, vec![press(true)]);
        run(&ctx, &mut c, vec![press(false)]);
        run(&ctx, &mut c, Vec::new());
        let shown = last_shown(&ctx).expect("the popup is open");
        assert!(screen().contains_rect(shown.popup), "{shown:?}");
        assert!(shown.popup.right() <= shown.swatch.left(), "{shown:?}");
        assert!(matches!(c, Color::Cmyk { .. }));
        // Its dotted strip drags it off what it covers; it stays open
        // there, inside the window.
        let grip = Pos2::new(shown.popup.center().x, shown.popup.top() + 10.0);
        let to = grip + Vec2::new(-300.0, 150.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(&ctx, &mut c, vec![egui::Event::PointerMoved(grip)]);
        run(&ctx, &mut c, vec![button(grip, true)]);
        for k in 1..=5 {
            let p = grip + (to - grip) * (k as f32 / 5.0);
            run(&ctx, &mut c, vec![egui::Event::PointerMoved(p)]);
        }
        run(&ctx, &mut c, vec![button(to, false)]);
        run(&ctx, &mut c, Vec::new());
        let moved = last_shown(&ctx).expect("still open");
        let d = moved.popup.min - shown.popup.min;
        assert!(
            (d.x + 300.0).abs() < 2.0 && (d.y - 150.0).abs() < 2.0,
            "{d:?}"
        );
        assert!(screen().contains_rect(moved.popup));
    }
}
