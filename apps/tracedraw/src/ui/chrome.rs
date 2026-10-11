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
pub const WINDOW_BORDER: Color32 = Color32::from_gray(0xB2);
/// Fields and lists.
pub const FIELD_BORDER: Color32 = Color32::from_gray(0xB2);
pub const FIELD_FOCUS: Color32 = Color32::from_rgb(0x00, 0xAD, 0xFE);
/// Lists, buttons.
pub const CONTROL_FILL: Color32 = Color32::from_gray(0xEA);
pub const CONTROL_HOVER: Color32 = Color32::from_rgb(0xE5, 0xF1, 0xFB);
/// The close button turns red under the pointer.
const CLOSE_HOVER: Color32 = Color32::from_rgb(0xE8, 0x11, 0x23);
const TITLE_FONT: f32 = 13.0;

/// A modal-looking window of `size` (borders and title bar included)
/// centred on the screen. `add` draws the body, whose rect starts under
/// the title bar at the window's left edge. Returns true when the close
/// button was clicked or Escape pressed.
pub fn dialog(ctx: &Context, id: &str, title: &str, size: Vec2, add: impl FnOnce(&mut Ui)) -> bool {
    let mut closed = false;
    egui::Window::new(id)
        .id(egui::Id::new(id))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(size)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            Frame::new()
                .fill(Color32::WHITE)
                .stroke(Stroke::new(1.0, WINDOW_BORDER))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: Color32::from_black_alpha(60),
                })
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
                Tokens::PANEL,
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
        Color32::BLACK,
    );
    galley.size().x + 12.0 + 24.0 + CLOSE_W
}

impl Window {
    /// Draw the window with `add` as its body. Returns what `add` returned.
    pub fn show<R>(self, ctx: &Context, add: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let mut out = None;
        let mut closed = false;
        egui::Window::new(&self.title)
            .id(self.id)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(window_frame())
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let min_w = title_width(ui, &self.title).max(280.0);
                ui.set_min_width(min_w);
                let top = ui.cursor().min;
                ui.add_space(TITLE_BAR);
                let inner = Frame::new()
                    .fill(Tokens::PANEL)
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
                        add(ui)
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

/// The frame of every dialog window: white, a grey border and a shadow.
fn window_frame() -> Frame {
    Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0, WINDOW_BORDER))
        .shadow(egui::epaint::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(60),
        })
        .inner_margin(Margin::ZERO)
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
}

/// Paint a 32 px message icon at `rect`: a blue disc with a white
/// question mark.
fn paint_message_icon(ui: &Ui, rect: Rect, icon: MessageIcon) {
    let p = ui.painter();
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0;
    match icon {
        MessageIcon::Question => {
            p.circle_filled(c, r, Color32::from_rgb(0x1F, 0x6F, 0xC5));
            p.circle_stroke(
                c,
                r - 0.5,
                Stroke::new(1.0, Color32::from_rgb(0x14, 0x52, 0x96)),
            );
            p.text(
                c + Vec2::new(0.0, 0.5),
                Align2::CENTER_CENTER,
                "?",
                crate::theme::bold(r * 1.25),
                Color32::WHITE,
            );
        }
    }
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
                color: Tokens::TEXT,
                underline: Stroke::new(1.0, Tokens::TEXT),
                ..Default::default()
            },
        );
        job.append(
            chars.as_str(),
            0.0,
            egui::TextFormat {
                font_id: font,
                color: Tokens::TEXT,
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
                        w.inactive.weak_bg_fill = Color32::from_rgb(0xE5, 0xF1, 0xFB);
                    }
                    ui.add_sized([100.0, 27.0], egui::Button::new(text))
                })
                .inner;
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
                ui.add(egui::Label::new(egui::RichText::new(text).size(13.5)).wrap());
            });
        });
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
    painter.rect_filled(bar, 0.0, Color32::WHITE);
    painter.text(
        Pos2::new(bar.left() + 12.0, bar.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(TITLE_FONT),
        Color32::BLACK,
    );
    let close = Rect::from_min_max(
        Pos2::new(bar.right() - CLOSE_W, bar.top()),
        bar.right_bottom(),
    );
    let resp = ui.interact(close, egui::Id::new((id, "close")), Sense::click());
    let color = if resp.hovered() {
        ui.painter().rect_filled(close, 0.0, CLOSE_HOVER);
        Color32::WHITE
    } else {
        Color32::BLACK
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
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, FIELD_BORDER);
    v.widgets.inactive.weak_bg_fill = CONTROL_FILL;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, FIELD_FOCUS);
    v.widgets.hovered.weak_bg_fill = CONTROL_HOVER;
    v.widgets.active.bg_stroke = Stroke::new(1.0, FIELD_FOCUS);
    v.widgets.open.bg_stroke = Stroke::new(1.0, FIELD_FOCUS);
    v.widgets.open.weak_bg_fill = CONTROL_HOVER;
    v.selection.stroke = Stroke::new(1.0, FIELD_FOCUS);
    v.extreme_bg_color = Color32::WHITE;
    v.text_edit_bg_color = Some(Color32::WHITE);
    // A chosen radio button shows a large black dot, as on the desktop.
    ui.spacing_mut().icon_width_inner = 12.0;
}

/// A white panel with a light border, for lists and page frames.
pub fn panel() -> Frame {
    Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0, Color32::from_gray(0xD9)))
        .inner_margin(Margin::same(6))
}

/// A section heading: its text, then a light line to the right edge.
pub fn section(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    let font = FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font, Tokens::TEXT);
    let h = galley.size().y.max(18.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let text_w = galley.size().x;
    ui.painter().galley(
        Pos2::new(rect.left(), rect.center().y - galley.size().y / 2.0),
        galley,
        Tokens::TEXT,
    );
    let y = rect.center().y.round() + 0.5;
    ui.painter().hline(
        (rect.left() + text_w + 12.0)..=rect.right(),
        y,
        Stroke::new(1.0, Color32::from_gray(0xDC)),
    );
    ui.add_space(4.0);
}

/// A push button of the standard dialog size (100 x 27).
pub fn button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add_sized([100.0, 27.0], egui::Button::new(text))
}

// ----- absolute form layout ---------------------------------------------------

/// A bold heading at a point of the body (General, Dimensions).
pub fn heading(ui: &mut Ui, at: Pos2, text: &str) {
    ui.painter().text(
        at,
        Align2::LEFT_CENTER,
        text,
        crate::theme::bold(13.0),
        Color32::BLACK,
    );
}

/// A label right-aligned so it ends at `right`, centred on `y`.
pub fn label_right(ui: &mut Ui, right: f32, y: f32, text: &str) {
    ui.painter().text(
        Pos2::new(right, y),
        Align2::RIGHT_CENTER,
        text,
        FontId::proportional(13.0),
        Color32::BLACK,
    );
}

/// A label left-aligned from `at`.
pub fn label_at(ui: &mut Ui, at: Pos2, text: &str) {
    ui.painter().text(
        at,
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.0),
        Color32::BLACK,
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
    ui.put(rect, egui::Button::new(text))
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
        Color32::WHITE,
        Stroke::new(1.0, FIELD_BORDER),
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
            ui.painter().rect_filled(r.shrink(1.0), 0.0, CONTROL_HOVER);
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
            Tokens::TEXT_DIM,
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
