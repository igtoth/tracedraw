//! Dialog windows drawn like the target design's: a white title bar with
//! the title on the left and a close button on the right, a light grey
//! body, centred over the main window, a one-pixel grey border.

use crate::theme::Tokens;
use egui::{Align2, Color32, Context, FontId, Frame, Margin, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// Height of the title bar.
pub const TITLE_BAR: f32 = 30.0;
/// The close button's width.
const CLOSE_W: f32 = 46.0;
const WINDOW_BORDER: Color32 = Color32::from_gray(0xAB);
/// The close button turns red under the pointer.
const CLOSE_HOVER: Color32 = Color32::from_rgb(0xE8, 0x11, 0x23);

/// A modal-looking window of `size` (title bar included) centred on the
/// screen. `add` draws the body. Returns true when the close button was
/// clicked or Escape pressed.
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
                .fill(Tokens::PANEL)
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
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(Rect::from_min_max(
                    Pos2::new(bar.left(), bar.bottom()),
                    Pos2::new(bar.right(), bar.top() + size.y),
                )),
                add,
            );
        });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        closed = true;
    }
    closed
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
        FontId::proportional(12.0),
        Tokens::TEXT,
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
        Tokens::TEXT
    };
    let c = close.center();
    let s = Stroke::new(1.0, color);
    let d = 5.0;
    ui.painter()
        .line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], s);
    ui.painter()
        .line_segment([c + Vec2::new(d, -d), c + Vec2::new(-d, d)], s);
    resp.clicked()
}

/// A white panel with a light border, as the target design's list and
/// page frames.
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

/// A push button of the target design's size (100 x 27).
pub fn button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add_sized([100.0, 27.0], egui::Button::new(text))
}
