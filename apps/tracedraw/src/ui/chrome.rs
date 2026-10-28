//! Dialog windows with a one-pixel grey
//! border, a white 31 px title bar with the title on the left and a close
//! button on the right, a white 5 px frame round a light grey body, centred
//! over the main window. Also the form controls its dialogs use: labels
//! right-aligned before 28 px fields, white text fields and grey lists
//! with a grey border (blue while typing), spin fields, push buttons.

use crate::theme::Tokens;
use egui::{
    Align2, Color32, Context, CornerRadius, FontId, Frame, Margin, Pos2, Rect, Sense, Stroke, Ui,
    Vec2,
};

/// Height of the title bar (under the top border).
pub const TITLE_BAR: f32 = 31.0;
/// The white frame round the body.
pub const EDGE: f32 = 5.0;
/// Height of fields, lists and buttons.
pub const FIELD_H: f32 = 28.0;
/// Rows of a form, one field tall plus the gap.
pub const ROW_PITCH: f32 = 33.0;
/// The close button's width.
const CLOSE_W: f32 = 46.0;
pub const FIELD_FOCUS: Color32 = Color32::from_rgb(0x00, 0xAD, 0xFE);
/// The close button turns red under the pointer.
const CLOSE_HOVER: Color32 = Color32::from_rgb(0xE8, 0x11, 0x23);
const TITLE_FONT: f32 = 13.0;

/// A modal-looking window of `size` (borders and title bar included)
/// centred on the screen. `add` draws the body, whose rect starts under
/// the title bar at the window's left edge. Returns true when the close
/// button was clicked or Escape pressed.
pub fn dialog(ctx: &Context, id: &str, title: &str, size: Vec2, add: impl FnOnce(&mut Ui)) -> bool {
    let mut closed = false;
    // On a screen smaller than the dialog, the window takes what there
    // is and its body scrolls (both ways) over the dialog's full layout.
    let room = (ctx.content_rect().size() - Vec2::splat(16.0)).max(Vec2::splat(200.0));
    let shown = size.min(room);
    if shown != size {
        let w = egui::Window::new(id)
            .id(egui::Id::new(id))
            .order(egui::Order::Foreground)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .fixed_size(shown);
        placed(w, ctx, egui::Id::new(id))
            .frame(
                Frame::new()
                    .fill(crate::theme::Tokens::title_bar())
                    .stroke(Stroke::new(1.0, crate::theme::Tokens::window_border()))
                    .inner_margin(Margin::ZERO),
            )
            .show(ctx, |ui| {
                ui.set_min_size(shown);
                let (bar, _) =
                    ui.allocate_exact_size(Vec2::new(shown.x, TITLE_BAR), Sense::hover());
                if title_bar(ui, bar, title, id) {
                    closed = true;
                }
                egui::ScrollArea::both()
                    .id_salt((id, "scroll"))
                    .max_width(shown.x)
                    .max_height(shown.y - TITLE_BAR)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (r, _) = ui.allocate_exact_size(
                            Vec2::new(size.x, size.y - TITLE_BAR),
                            Sense::hover(),
                        );
                        let body = Rect::from_min_max(Pos2::new(r.left() - 1.0, r.top()), r.max);
                        ui.painter().rect_filled(
                            Rect::from_min_max(
                                body.min + Vec2::new(EDGE, 0.0),
                                body.max - Vec2::new(EDGE, EDGE),
                            ),
                            0.0,
                            Tokens::panel(),
                        );
                        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                            form_style(ui);
                            add(ui)
                        });
                    });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            closed = true;
        }
        return closed;
    }
    let w = egui::Window::new(id)
        .id(egui::Id::new(id))
        .order(egui::Order::Foreground)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(size);
    placed(w, ctx, egui::Id::new(id))
        .frame(
            Frame::new()
                .fill(crate::theme::Tokens::title_bar())
                .stroke(Stroke::new(1.0, crate::theme::Tokens::window_border()))
                .shadow(crate::theme::shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: Color32::from_black_alpha(60),
                }))
                .inner_margin(Margin::ZERO),
        )
        .show(ctx, |ui| {
            ui.set_min_size(size);
            let (bar, _) = ui.allocate_exact_size(Vec2::new(size.x, TITLE_BAR), Sense::hover());
            if title_bar(ui, bar, title, id) {
                closed = true;
            }
            // Body coordinates: x from the window's outer edge, y from
            // under the title bar.
            let body = Rect::from_min_max(
                Pos2::new(bar.left() - 1.0, bar.bottom()),
                Pos2::new(bar.right(), bar.top() + size.y),
            );
            ui.painter().rect_filled(
                Rect::from_min_max(
                    body.min + Vec2::new(EDGE, 0.0),
                    body.max - Vec2::new(EDGE, EDGE),
                ),
                0.0,
                Tokens::panel(),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
                form_style(ui);
                add(ui)
            });
        });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        closed = true;
    }
    closed
}

/// How much of a dialog stays inside the main window, at least, when it
/// is dragged towards a side: enough of its title bar to take it back.
pub const KEPT_IN_VIEW: f32 = 100.0;

/// Where a dialog of `size` may be dragged within `screen`: anywhere its
/// title bar stays below the top edge and at least [`KEPT_IN_VIEW`] of
/// it stays inside the left, right and bottom edges. A dialog about as
/// large as the window can so still be pushed aside to show the drawing
/// beneath; one held wholly inside the window could not move at all
/// (#908).
pub fn drag_reach(screen: Rect, size: Vec2) -> Rect {
    let side = (size.x - KEPT_IN_VIEW).max(0.0);
    let below = (size.y - TITLE_BAR).max(0.0);
    Rect::from_min_max(
        Pos2::new(screen.left() - side, screen.top()),
        Pos2::new(screen.right() + side, screen.bottom() + below),
    )
}

/// Place a dialog window `id`: it can always be dragged by its title bar,
/// partly out of the main window (see [`drag_reach`]). When it opens (it
/// was not shown on the frame before) it starts at the centre of the
/// main window, or, with Center dialog boxes off, where it was left;
/// while open it stays where the user put it.
fn placed<'a>(window: egui::Window<'a>, ctx: &Context, id: egui::Id) -> egui::Window<'a> {
    let frame = ctx.cumulative_frame_nr();
    let key = id.with("chrome_last_frame");
    let last: Option<u64> = ctx.data(|d| d.get_temp(key));
    ctx.data_mut(|d| d.insert_temp(key, frame));
    let opened = last.is_none_or(|l| l + 1 < frame);
    let screen = ctx.content_rect();
    // The size it had last frame; on its first frame the window itself.
    let reach = match ctx.memory(|m| m.area_rect(id)) {
        Some(r) if !opened && r.is_finite() => drag_reach(screen, r.size()),
        _ => screen,
    };
    let window = window
        .movable(true)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(screen.center())
        .constrain_to(reach);
    if opened && center_dialogs(ctx) {
        window.current_pos(ctx.content_rect().center())
    } else {
        window
    }
}

/// An auto-sized dialog window in the same chrome as [`dialog`]: the white
/// title bar with the close button, a white edge and the light grey body,
/// centred over the main window. The close button asks the dialog system
/// to close the dialog (see [`take_close_request`]).
pub struct Window {
    title: String,
    id: egui::Id,
}

/// The chrome window for a dialog titled `title`.
pub fn window(title: impl Into<String>) -> Window {
    let title = title.into();
    let id = egui::Id::new(("chrome_dialog", title.clone()));
    Window { title, id }
}

/// The widest a dialog's title bar must be for its title and close button.
fn title_width(ui: &Ui, title: &str) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        title.to_string(),
        FontId::proportional(TITLE_FONT),
        crate::theme::Tokens::text(),
    );
    galley.size().x + 12.0 + 24.0 + CLOSE_W
}

impl Window {
    /// Draw the window with `add` as its body. Returns what `add` returned.
    pub fn show<R>(self, ctx: &Context, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let mut out = None;
        let mut closed = false;
        let window = egui::Window::new(&self.title)
            .id(self.id)
            .order(egui::Order::Foreground)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .frame(window_frame());
        // Dragged by the title bar; opens centred (or where it was left).
        let window = placed(window, ctx, self.id);
        window.show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let min_w = title_width(ui, &self.title).max(280.0);
            ui.set_min_width(min_w);
            let top = ui.cursor().min;
            ui.add_space(TITLE_BAR);
            let inner = Frame::new()
                .fill(Tokens::panel())
                .inner_margin(Margin {
                    left: 14,
                    right: 14,
                    top: 12,
                    bottom: 12,
                })
                .outer_margin(Margin {
                    left: EDGE as i8,
                    right: EDGE as i8,
                    top: 0,
                    bottom: EDGE as i8,
                })
                .show(ui, |ui| {
                    form_style(ui);
                    // A dialog taller than the window scrolls.
                    let max_h = (ui.ctx().content_rect().height() - TITLE_BAR - 60.0).max(150.0);
                    // The window starts from the size it had last frame, so
                    // the body may grow up to `max_h` before it scrolls.
                    egui::ScrollArea::vertical()
                        .id_salt("dialog_body")
                        .max_height(max_h)
                        .min_scrolled_height(max_h)
                        .show(ui, |ui| add(ui))
                        .inner
                });
            out = Some(inner.inner);
            let width = ui.min_rect().width().max(min_w);
            let bar = Rect::from_min_size(top, Vec2::new(width, TITLE_BAR));
            if title_bar(ui, bar, &self.title, &format!("{:?}", self.id)) {
                closed = true;
            }
        });
        if closed {
            request_close(ctx);
        }
        out
    }
}

/// Dialogs are modal: while one is open, a transparent layer over the
/// whole window, under the dialogs (which sit in the foreground order),
/// takes every click, drag and scroll meant for the menus, toolbars,
/// dockers and the drawing.
pub fn modal_blocker(ctx: &Context) {
    modal_blocker_except(ctx, &[]);
}

/// [`modal_blocker`] leaving `holes` open: the toolbars whose buttons
/// the Customization options let the user move (#563).
pub fn modal_blocker_except(ctx: &Context, holes: &[Rect]) {
    // A colour picker's eyedropper waits for a click on the drawing.
    if crate::ui::color_picker::picking(ctx) {
        return;
    }
    let screen = ctx.content_rect();
    let pieces = if holes.is_empty() {
        vec![screen]
    } else {
        uncovered(screen, holes)
    };
    for (i, r) in pieces.into_iter().enumerate() {
        let id = if i == 0 {
            egui::Id::new("modal_blocker")
        } else {
            egui::Id::new(("modal_blocker", i))
        };
        egui::Area::new(id)
            .order(egui::Order::Middle)
            .fixed_pos(r.min)
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_min_size(r.size());
                ui.interact(r, id.with("cover"), Sense::click_and_drag());
            });
        ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, id));
    }
}

/// `screen` without `holes`, as rectangles: bands between the holes'
/// top and bottom edges, each cut where holes cross it.
pub fn uncovered(screen: Rect, holes: &[Rect]) -> Vec<Rect> {
    let holes: Vec<Rect> = holes
        .iter()
        .map(|h| h.intersect(screen))
        .filter(|h| h.is_positive())
        .collect();
    let mut ys = vec![screen.top(), screen.bottom()];
    for h in &holes {
        ys.push(h.top());
        ys.push(h.bottom());
    }
    ys.sort_by(f32::total_cmp);
    ys.dedup();
    let mut out = Vec::new();
    for w in ys.windows(2) {
        let (y0, y1) = (w[0], w[1]);
        if y1 - y0 < 0.5 {
            continue;
        }
        let mut xs: Vec<(f32, f32)> = holes
            .iter()
            .filter(|h| h.top() <= y0 && h.bottom() >= y1)
            .map(|h| (h.left(), h.right()))
            .collect();
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut x = screen.left();
        for (a, b) in xs {
            if a > x {
                out.push(Rect::from_min_max(Pos2::new(x, y0), Pos2::new(a, y1)));
            }
            x = x.max(b);
        }
        if x < screen.right() {
            out.push(Rect::from_min_max(
                Pos2::new(x, y0),
                Pos2::new(screen.right(), y1),
            ));
        }
    }
    out
}

/// The frame of every dialog window: white, a grey border and a shadow.
fn window_frame() -> Frame {
    Frame::new()
        .fill(crate::theme::Tokens::title_bar())
        .stroke(Stroke::new(1.0, crate::theme::Tokens::window_border()))
        .shadow(crate::theme::shadow(egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(60),
        }))
        .inner_margin(Margin::ZERO)
}

fn center_id() -> egui::Id {
    egui::Id::new("chrome_center_dialogs")
}

/// Customization > Appearance > Center Dialog Boxes, for the dialogs
/// drawn this frame.
pub fn set_center_dialogs(ctx: &Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(center_id(), on));
}

/// Dialogs open in the middle and stay there (the default).
pub fn center_dialogs(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp(center_id())).unwrap_or(true)
}

fn close_request_id() -> egui::Id {
    egui::Id::new("chrome_dialog_close_request")
}

/// Ask the dialog system to close the open dialog (its close button).
pub fn request_close(ctx: &Context) {
    ctx.data_mut(|d| d.insert_temp(close_request_id(), true));
}

/// True once after a dialog's close button was clicked.
pub fn take_close_request(ctx: &Context) -> bool {
    ctx.data_mut(|d| d.remove_temp::<bool>(close_request_id()))
        .unwrap_or(false)
}

/// The icon a message box shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageIcon {
    Question,
    /// Something to know: an "i".
    Info,
    /// Work going on (checking for updates): a turning ring of dashes,
    /// the text in bold.
    Busy,
}

/// Paint a 32 px message icon at `rect`: a blue disc with a white
/// question mark or "i".
fn paint_message_icon(ui: &Ui, rect: Rect, icon: MessageIcon) {
    let p = ui.painter();
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0;
    let mark = match icon {
        MessageIcon::Question => "?",
        MessageIcon::Info => "i",
        MessageIcon::Busy => {
            // Eight dashes; the brightest one goes round once a second.
            let t = ui.input(|i| i.time);
            let lead = (t * 8.0) as usize % 8;
            for k in 0..8usize {
                let a = k as f32 * std::f32::consts::TAU / 8.0;
                let d = Vec2::new(a.cos(), a.sin());
                let age = (lead + 8 - k) % 8;
                let grey = (40 + age * 24).min(220) as u8;
                p.line_segment(
                    [c + d * r * 0.45, c + d * r * 0.8],
                    Stroke::new(2.0, Color32::from_gray(grey)),
                );
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(60));
            return;
        }
    };
    p.circle_filled(c, r, Color32::from_rgb(0x1F, 0x6F, 0xC5));
    p.circle_stroke(
        c,
        r - 0.5,
        Stroke::new(1.0, Color32::from_rgb(0x14, 0x52, 0x96)),
    );
    p.text(
        c + Vec2::new(0.0, 0.5),
        Align2::CENTER_CENTER,
        mark,
        crate::theme::bold(r * 1.25),
        Color32::WHITE,
    );
}

/// A button label with its access key (the first letter) underlined.
fn access_label(text: &str) -> egui::WidgetText {
    let mut job = egui::text::LayoutJob::default();
    let font = FontId::proportional(13.0);
    let mut chars = text.chars();
    if let Some(first) = chars.next() {
        job.append(
            &first.to_string(),
            0.0,
            egui::TextFormat {
                font_id: font.clone(),
                color: Tokens::text(),
                underline: Stroke::new(1.0, Tokens::text()),
                ..Default::default()
            },
        );
        job.append(
            chars.as_str(),
            0.0,
            egui::TextFormat {
                font_id: font,
                color: Tokens::text(),
                ..Default::default()
            },
        );
    }
    job.into()
}

/// A row of 100 x 27 push buttons aligned to the right of what was drawn
/// above them. The first is the default button (a blue border, Enter);
/// the last answers Escape. With `access_keys`, each button's first letter
/// is underlined and pressing it (with no field focused) picks it. Returns
/// the index of the button picked this frame.
pub fn button_row(ui: &mut Ui, labels: &[&str], access_keys: bool) -> Option<usize> {
    let mut picked = None;
    let spacing = 8.0;
    let total = labels.len() as f32 * 100.0 + (labels.len().saturating_sub(1)) as f32 * spacing;
    let left = (ui.min_rect().width() - total).max(0.0);
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;
        ui.add_space(left);
        for (i, label) in labels.iter().enumerate() {
            let text: egui::WidgetText = if access_keys {
                access_label(label)
            } else {
                (*label).into()
            };
            let resp = ui
                .scope(|ui| {
                    if i == 0 {
                        let w = &mut ui.visuals_mut().widgets;
                        w.inactive.bg_stroke = Stroke::new(1.0, Tokens::ACCENT);
                        w.inactive.weak_bg_fill = crate::theme::Tokens::tool_hover();
                    }
                    ui.add_sized([100.0, 27.0], egui::Button::new(text))
                })
                .inner;
            let resp = crate::ui::tips::named(resp, label);
            if resp.clicked() {
                picked = Some(i);
            }
        }
    });
    let typing = ui.memory(|m| m.focused().is_some());
    ui.input(|inp| {
        if picked.is_some() {
            return;
        }
        if inp.key_pressed(egui::Key::Enter) && !typing {
            picked = Some(0);
        } else if inp.key_pressed(egui::Key::Escape) && labels.len() > 1 {
            picked = Some(labels.len() - 1);
        } else if access_keys && !typing && inp.modifiers.is_none() {
            for ev in &inp.events {
                if let egui::Event::Text(t) = ev {
                    let typed = t.to_lowercase();
                    if let Some(i) = labels.iter().position(|l| {
                        l.chars()
                            .next()
                            .map(|c| c.to_lowercase().to_string() == typed)
                            .unwrap_or(false)
                    }) {
                        picked = Some(i);
                    }
                }
            }
        }
    });
    picked
}

/// A message box: the application name as title, an icon, the message and
/// a row of buttons (see [`button_row`]). Returns the index of the button
/// picked, the last one when the close button was clicked.
pub fn message_box(
    ctx: &Context,
    icon: MessageIcon,
    text: &str,
    buttons: &[&str],
) -> Option<usize> {
    message_box_with(ctx, icon, text, buttons, None)
}

/// A [`message_box`] with a check box under the message ("Do not show
/// this message again").
pub fn message_box_check(
    ctx: &Context,
    icon: MessageIcon,
    text: &str,
    buttons: &[&str],
    check: &str,
    checked: &mut bool,
) -> Option<usize> {
    message_box_with(ctx, icon, text, buttons, Some((check, checked)))
}

fn message_box_with(
    ctx: &Context,
    icon: MessageIcon,
    text: &str,
    buttons: &[&str],
    check: Option<(&str, &mut bool)>,
) -> Option<usize> {
    let mut picked = None;
    let before = take_close_request(ctx);
    window("TraceDraw").show(ctx, |ui| {
        ui.set_min_width(360.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
            paint_message_icon(ui, rect, icon);
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.add_space(6.0);
                ui.set_max_width(380.0);
                let mut rich = egui::RichText::new(text).size(13.5);
                if icon == MessageIcon::Busy {
                    rich = rich.strong();
                }
                ui.add(egui::Label::new(rich).wrap());
            });
        });
        if let Some((label, checked)) = check {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(48.0);
                ui.checkbox(checked, label);
            });
        }
        ui.add_space(14.0);
        picked = button_row(ui, buttons, true);
    });
    if before || take_close_request(ctx) {
        picked = Some(buttons.len().saturating_sub(1));
    }
    picked
}

/// The white title bar: title at the left, the close button at the right.
/// Returns true when the close button was clicked.
fn title_bar(ui: &mut Ui, bar: Rect, title: &str, id: &str) -> bool {
    let painter = ui.painter();
    painter.rect_filled(bar, 0.0, crate::theme::Tokens::title_bar());
    painter.text(
        Pos2::new(bar.left() + 12.0, bar.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(TITLE_FONT),
        crate::theme::Tokens::text(),
    );
    let close = Rect::from_min_max(
        Pos2::new(bar.right() - CLOSE_W, bar.top()),
        bar.right_bottom(),
    );
    let resp = ui.interact(close, egui::Id::new((id, "close")), Sense::click());
    let resp =
        crate::ui::tips::RichTip::rich_tip(resp, crate::ui::tips::keyed("dialog.close_window"));
    let color = if resp.hovered() {
        ui.painter().rect_filled(close, 0.0, CLOSE_HOVER);
        Color32::WHITE
    } else {
        crate::theme::Tokens::text()
    };
    let c = close.center();
    let s = Stroke::new(1.3, color);
    let d = 6.0;
    ui.painter()
        .line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], s);
    ui.painter()
        .line_segment([c + Vec2::new(d, -d), c + Vec2::new(-d, d)], s);
    resp.clicked()
}

/// Controls inside a dialog body: square corners, grey borders, the blue
/// border while typing, grey lists and buttons.
pub fn form_style(ui: &mut Ui) {
    let v = ui.visuals_mut();
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::ZERO;
        w.expansion = 0.0;
    }
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, crate::theme::Tokens::control_border());
    v.widgets.inactive.weak_bg_fill = crate::theme::Tokens::control_fill();
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, FIELD_FOCUS);
    v.widgets.hovered.weak_bg_fill = crate::theme::Tokens::tool_hover();
    v.widgets.active.bg_stroke = Stroke::new(1.0, FIELD_FOCUS);
    v.widgets.open.bg_stroke = Stroke::new(1.0, FIELD_FOCUS);
    v.widgets.open.weak_bg_fill = crate::theme::Tokens::tool_hover();
    v.selection.stroke = Stroke::new(1.0, FIELD_FOCUS);
    v.extreme_bg_color = crate::theme::Tokens::field();
    v.text_edit_bg_color = Some(crate::theme::Tokens::field());
    // A chosen radio button shows a large black dot, as on the desktop.
    ui.spacing_mut().icon_width_inner = 12.0;
}

/// A white panel with a light border, for lists and page frames.
pub fn panel() -> Frame {
    Frame::new()
        .fill(crate::theme::Tokens::field())
        .stroke(Stroke::new(1.0, crate::theme::Tokens::border()))
        .inner_margin(Margin::same(6))
}

/// A section heading: its text, then a light line to the right edge.
pub fn section(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    let font = FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font, Tokens::text());
    let h = galley.size().y.max(18.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let text_w = galley.size().x;
    ui.painter().galley(
        Pos2::new(rect.left(), rect.center().y - galley.size().y / 2.0),
        galley,
        Tokens::text(),
    );
    let y = rect.center().y.round() + 0.5;
    ui.painter().hline(
        (rect.left() + text_w + 12.0)..=rect.right(),
        y,
        Stroke::new(1.0, crate::theme::Tokens::border()),
    );
    ui.add_space(4.0);
}

/// A push button of the standard dialog size (100 x 27).
pub fn button(ui: &mut Ui, text: &str) -> egui::Response {
    let resp = ui.add_sized([100.0, 27.0], egui::Button::new(text));
    crate::ui::tips::named(resp, text)
}

// ----- absolute form layout ---------------------------------------------------

/// A bold heading at a point of the body (General, Dimensions).
pub fn heading(ui: &mut Ui, at: Pos2, text: &str) {
    ui.painter().text(
        at,
        Align2::LEFT_CENTER,
        text,
        crate::theme::bold(13.0),
        crate::theme::Tokens::text(),
    );
}

/// A label right-aligned so it ends at `right`, centred on `y`.
pub fn label_right(ui: &mut Ui, right: f32, y: f32, text: &str) {
    ui.painter().text(
        Pos2::new(right, y),
        Align2::RIGHT_CENTER,
        text,
        FontId::proportional(13.0),
        crate::theme::Tokens::text(),
    );
}

/// A label left-aligned from `at`.
pub fn label_at(ui: &mut Ui, at: Pos2, text: &str) {
    ui.painter().text(
        at,
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.0),
        crate::theme::Tokens::text(),
    );
}

/// A one-line text field filling `rect`.
pub fn text_field(ui: &mut Ui, rect: Rect, text: &mut String) -> egui::Response {
    ui.put(
        rect,
        egui::TextEdit::singleline(text)
            .vertical_align(egui::Align::Center)
            .margin(Margin::symmetric(4, 0))
            .min_size(rect.size()),
    )
}

/// A push button filling `rect`.
pub fn button_at(ui: &mut Ui, rect: Rect, text: &str) -> egui::Response {
    let resp = ui.put(rect, egui::Button::new(text));
    crate::ui::tips::named(resp, text)
}

/// A list (combo box) filling `rect`.
pub fn combo<R>(
    ui: &mut Ui,
    id: &str,
    rect: Rect,
    selected: impl Into<egui::WidgetText>,
    add: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let mut out = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.spacing_mut().interact_size.y = rect.height();
        ui.spacing_mut().button_padding = Vec2::new(4.0, 0.0);
        out = egui::ComboBox::from_id_salt(id)
            .width(rect.width())
            .height(400.0)
            .selected_text(selected)
            .show_ui(ui, add)
            .inner;
    });
    out
}

/// A number field with the up and down arrows at its right end (a spin
/// box): typing commits on Enter or when the
/// field loses focus, the arrows step by `step`. Returns the new value.
#[allow(clippy::too_many_arguments)]
pub fn spin_field(
    ui: &mut Ui,
    id: &str,
    rect: Rect,
    value: f64,
    decimals: usize,
    suffix: &str,
    step: f64,
    range: std::ops::RangeInclusive<f64>,
) -> Option<f64> {
    let arrows = 23.0;
    let field = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.right() - arrows + 1.0, rect.bottom()),
    );
    let mut out = number_field(ui, id, field, value, decimals, suffix, range.clone());
    // The arrows: a column at the right inside the same border.
    let col = Rect::from_min_max(Pos2::new(field.right() - 1.0, rect.top()), rect.max);
    let painter = ui.painter();
    painter.rect(
        col,
        0.0,
        crate::theme::Tokens::field(),
        Stroke::new(1.0, crate::theme::Tokens::control_border()),
        egui::StrokeKind::Inside,
    );
    let half = col.height() / 2.0;
    for (i, up) in [(0, true), (1, false)] {
        let r = Rect::from_min_size(
            Pos2::new(col.left(), col.top() + i as f32 * half),
            Vec2::new(col.width(), half),
        );
        let a = ui.interact(r, egui::Id::new((id, "spin", i)), Sense::click());
        if a.hovered() {
            ui.painter()
                .rect_filled(r.shrink(1.0), 0.0, crate::theme::Tokens::tool_hover());
        }
        let c = r.center();
        let pts = if up {
            vec![
                c + Vec2::new(-3.5, 1.5),
                c + Vec2::new(3.5, 1.5),
                c + Vec2::new(0.0, -2.0),
            ]
        } else {
            vec![
                c + Vec2::new(-3.5, -1.5),
                c + Vec2::new(3.5, -1.5),
                c + Vec2::new(0.0, 2.0),
            ]
        };
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            pts,
            Tokens::text_dim(),
            Stroke::NONE,
        ));
        if a.clicked() {
            let base = out.unwrap_or(value);
            let v = (base + if up { step } else { -step }).clamp(*range.start(), *range.end());
            out = Some(v);
        }
    }
    out
}

/// A number typed in a text field: it commits on Enter or when the field
/// loses focus. Returns the new value.
pub fn number_field(
    ui: &mut Ui,
    id: &str,
    rect: Rect,
    value: f64,
    decimals: usize,
    suffix: &str,
    range: std::ops::RangeInclusive<f64>,
) -> Option<f64> {
    let edit_id = egui::Id::new((id, "number_text"));
    let shown = format_number(value, decimals, suffix);
    let mut text = ui
        .ctx()
        .data(|d| d.get_temp::<String>(edit_id))
        .unwrap_or_else(|| shown.clone());
    let resp = text_field(ui, rect, &mut text);
    let mut out = None;
    if resp.has_focus() {
        ui.ctx().data_mut(|d| d.insert_temp(edit_id, text.clone()));
    } else {
        if resp.lost_focus() {
            if let Some(v) = parse_number(&text) {
                let v = v.clamp(*range.start(), *range.end());
                if (v - value).abs() > 1e-12 {
                    out = Some(v);
                }
            }
        }
        ui.ctx().data_mut(|d| d.remove::<String>(edit_id));
    }
    out
}

/// A number as the fields show it: `decimals` places, the suffix after a
/// space.
pub fn format_number(v: f64, decimals: usize, suffix: &str) -> String {
    let s = format!("{v:.decimals$}");
    if suffix.is_empty() {
        s
    } else {
        format!("{s} {suffix}")
    }
}

/// The number typed in a field's text (a unit after it is ignored; a
/// comma counts as the decimal point; sums such as `10+5` work).
pub fn parse_number(text: &str) -> Option<f64> {
    crate::ui::field::parse(text, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The screen less the holes is covered once, and no piece reaches
    /// into a hole (#563).
    #[test]
    fn the_blocker_leaves_the_holes_open() {
        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 800.0));
        let holes = [
            Rect::from_min_max(Pos2::new(0.0, 24.0), Pos2::new(700.0, 60.0)),
            Rect::from_min_max(Pos2::new(300.0, 100.0), Pos2::new(500.0, 140.0)),
        ];
        let pieces = uncovered(screen, &holes);
        let area: f32 = pieces.iter().map(|r| r.area()).sum();
        let holes_area: f32 = holes.iter().map(|r| r.area()).sum();
        assert!((area + holes_area - screen.area()).abs() < 1.0, "{area}");
        for p in &pieces {
            for h in &holes {
                assert!(!p.intersect(*h).is_positive(), "{p:?} {h:?}");
            }
        }
        assert_eq!(uncovered(screen, &[]), vec![screen]);
    }

    /// A dialog opens centred, moves when its title bar is dragged and
    /// stays there while open; opened again it is centred again.
    #[test]
    fn dialogs_open_centred_and_move_by_their_title_bar() {
        let ctx = crate::theme::ui_context();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0));
        let run = |ctx: &Context, events: Vec<egui::Event>, show: bool| -> Option<Rect> {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let mut rect = None;
            let mut out = ctx.run_ui(input, |ui| {
                if show {
                    window("Movable")
                        .show(ui.ctx(), |ui| {
                            ui.label("body");
                            ui.add_space(80.0);
                        })
                        .map(|_| ());
                    rect = ui.ctx().memory(|m| {
                        m.area_rect(egui::Id::new(("chrome_dialog", "Movable".to_string())))
                    });
                }
            });
            out.textures_delta.clear();
            rect
        };
        for _ in 0..3 {
            run(&ctx, Vec::new(), true);
        }
        let r = run(&ctx, Vec::new(), true).expect("shown");
        assert!((r.center() - screen.center()).length() < 2.0, "{r:?}");
        // Drag the title bar 200 px right and 100 px down.
        let from = Pos2::new(r.left() + 40.0, r.top() + TITLE_BAR / 2.0);
        let to = from + Vec2::new(200.0, 100.0);
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(&ctx, vec![egui::Event::PointerMoved(from)], true);
        run(&ctx, vec![press(from, true)], true);
        for i in 1..=5 {
            let p = from + (to - from) * (i as f32 / 5.0);
            run(&ctx, vec![egui::Event::PointerMoved(p)], true);
        }
        run(&ctx, vec![press(to, false)], true);
        let moved = run(&ctx, Vec::new(), true).expect("shown");
        assert!(
            (moved.center() - (r.center() + Vec2::new(200.0, 100.0))).length() < 3.0,
            "{r:?} -> {moved:?}"
        );
        // Closed for a frame, then opened again: centred.
        run(&ctx, Vec::new(), false);
        run(&ctx, Vec::new(), false);
        run(&ctx, Vec::new(), true);
        let again = run(&ctx, Vec::new(), true).expect("shown");
        assert!(
            (again.center() - screen.center()).length() < 2.0,
            "{again:?}"
        );
    }

    /// A dialog nearly as tall as the window moves down and sideways
    /// when its title bar is dragged, out of the window as far as a part
    /// of it stays in (#908): kept wholly inside, it could not move.
    #[test]
    fn a_dialog_as_tall_as_the_window_drags_partly_out_of_it() {
        let ctx = crate::theme::ui_context();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0));
        let id = egui::Id::new(("chrome_dialog", "Tall".to_string()));
        let run = |ctx: &Context, events: Vec<egui::Event>| -> Option<Rect> {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                window("Tall").show(ui.ctx(), |ui| {
                    ui.label("body");
                    ui.add_space(2000.0);
                });
            });
            out.textures_delta.clear();
            ctx.memory(|m| m.area_rect(id))
        };
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let drag = |ctx: &Context, from: Pos2, to: Pos2| {
            run(ctx, vec![egui::Event::PointerMoved(from)]);
            run(ctx, vec![press(from, true)]);
            for i in 1..=5 {
                run(
                    ctx,
                    vec![egui::Event::PointerMoved(
                        from + (to - from) * (i as f32 / 5.0),
                    )],
                );
            }
            run(ctx, vec![press(to, false)]);
            run(ctx, Vec::new()).expect("shown")
        };
        for _ in 0..3 {
            run(&ctx, Vec::new());
        }
        let r = run(&ctx, Vec::new()).expect("shown");
        assert!(r.height() > 700.0, "{r:?}");
        // Down and to the right by 200 px: it follows.
        let from = Pos2::new(r.left() + 40.0, r.top() + TITLE_BAR / 2.0);
        let moved = drag(&ctx, from, from + Vec2::new(200.0, 200.0));
        assert!(
            (moved.min - (r.min + Vec2::new(200.0, 200.0))).length() < 3.0,
            "{r:?} -> {moved:?}"
        );
        // Far down and left: its title bar and a part of it stay in.
        let from = Pos2::new(moved.left() + 40.0, moved.top() + TITLE_BAR / 2.0);
        let far = drag(&ctx, from, from + Vec2::new(-3000.0, 3000.0));
        assert!(far.top() <= screen.bottom() - TITLE_BAR + 0.5, "{far:?}");
        assert!(far.top() > screen.bottom() - 200.0, "{far:?}");
        assert!(far.right() >= screen.left() + KEPT_IN_VIEW - 0.5, "{far:?}");
        assert!(far.right() < screen.left() + 200.0, "{far:?}");
        // Up: never above the window's top edge.
        let from = Pos2::new(far.right() - 40.0, far.top() + TITLE_BAR / 2.0);
        let up = drag(&ctx, from, Pos2::new(from.x, -3000.0));
        assert!(up.top() >= screen.top() - 0.5, "{up:?}");
    }

    #[test]
    fn drag_reach_keeps_a_part_of_the_dialog_in_the_window() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0));
        let r = drag_reach(screen, Vec2::new(400.0, 300.0));
        assert_eq!(r.top(), 0.0);
        assert_eq!(r.left(), -(400.0 - KEPT_IN_VIEW));
        assert_eq!(r.right(), 1000.0 + 400.0 - KEPT_IN_VIEW);
        assert_eq!(r.bottom(), 700.0 + 300.0 - TITLE_BAR);
        // A dialog narrower than what is kept stays inside sideways.
        let r = drag_reach(screen, Vec2::new(60.0, 20.0));
        assert_eq!((r.left(), r.right(), r.bottom()), (0.0, 1000.0, 700.0));
    }

    #[test]
    fn fields_read_numbers_with_units_and_commas() {
        assert_eq!(parse_number("210.0 mm"), Some(210.0));
        assert_eq!(parse_number(" 1920,5 px"), Some(1920.5));
        assert_eq!(parse_number("-3"), Some(-3.0));
        assert_eq!(parse_number("mm"), None);
        assert_eq!(parse_number(""), None);
        assert_eq!(format_number(210.0, 1, "mm"), "210.0 mm");
        assert_eq!(format_number(3.0, 0, ""), "3");
    }
}
