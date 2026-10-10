//! The document tabs above the rulers: the Welcome Screen, one tab per open
//! drawing (an asterisk marks unsaved changes; the close button shows on
//! the hovered tab) and the New tab after the last one, over a blue line
//! that runs the width of the window, as the target design draws them.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// Height of a tab; the blue line adds one pixel under it.
pub const TAB_HEIGHT: f32 = 26.0;
const PAD: f32 = 15.0;
const CLOSE: f32 = 14.0;
const FONT: f32 = 13.0;
/// The strip behind the tabs (also the one-pixel gaps between tabs).
pub const STRIP: Color32 = Color32::from_gray(0xEA);
const TAB: Color32 = Color32::from_gray(0xD8);
const TAB_ACTIVE: Color32 = Color32::from_rgb(0xCE, 0xE3, 0xFF);
const TAB_HOVER: Color32 = Color32::from_rgb(0xE5, 0xF1, 0xFB);
/// The line under the tabs.
const LINE: Color32 = Color32::from_rgb(0x00, 0xAD, 0xFE);

/// What a click on the strip asked for.
enum Action {
    Welcome,
    Switch(usize),
    Close(usize),
    New,
}

pub fn document_tabs(app: &mut App, ui: &mut Ui) {
    let mut action = None;
    let strip = ui.max_rect();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 1.0;
        if welcome_tab(ui, app.show_welcome || !app.has_document()) {
            action = Some(Action::Welcome);
        }
        for i in 0..app.docs.len() {
            let title = app.document_tab_title(i);
            let active = !app.show_welcome && i == app.active_doc;
            match doc_tab(ui, i, &title, active) {
                TabClick::Select => action = Some(Action::Switch(i)),
                TabClick::Close => action = Some(Action::Close(i)),
                TabClick::None => {}
            }
        }
        if new_tab(ui) {
            action = Some(Action::New);
        }
    });
    let line_y = strip.top() + TAB_HEIGHT;
    ui.painter().rect_filled(
        Rect::from_min_max(
            Pos2::new(strip.left(), line_y),
            Pos2::new(strip.right(), line_y + 1.0),
        ),
        0.0,
        LINE,
    );
    ui.allocate_space(Vec2::new(0.0, 1.0));
    match action {
        Some(Action::Welcome) => app.show_welcome = true,
        Some(Action::Switch(i)) => app.switch_document(i),
        Some(Action::Close(i)) => {
            app.switch_document(i);
            app.close_document();
        }
        Some(Action::New) => app.request_new_document(),
        None => {}
    }
}

fn tab_fill(active: bool, hovered: bool) -> Color32 {
    if active {
        TAB_ACTIVE
    } else if hovered {
        TAB_HOVER
    } else {
        TAB
    }
}

/// The Welcome Screen tab: a house and the title.
fn welcome_tab(ui: &mut Ui, active: bool) -> bool {
    let label = tr("welcome.title");
    let galley =
        ui.painter()
            .layout_no_wrap(label, egui::FontId::proportional(FONT), Color32::BLACK);
    let icon = 16.0;
    let width = 6.0 + icon + 15.0 + galley.size().x + 27.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, TAB_HEIGHT), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, tab_fill(active, resp.hovered()));
    house(
        painter,
        Pos2::new(rect.left() + 6.0, rect.top() + 5.0),
        Color32::from_gray(0x33),
    );
    painter.galley(
        Pos2::new(
            rect.left() + 6.0 + icon + 15.0,
            (rect.center().y - galley.size().y / 2.0).round(),
        ),
        galley,
        Color32::BLACK,
    );
    resp.clicked()
}

/// A filled house, 16 pixels square, top-left at `o`: a roof with eaves
/// and a body with a door.
fn house(painter: &egui::Painter, o: Pos2, color: Color32) {
    let roof = vec![
        Pos2::new(o.x + 8.0, o.y),
        Pos2::new(o.x + 16.0, o.y + 8.5),
        Pos2::new(o.x, o.y + 8.5),
    ];
    painter.add(egui::Shape::convex_polygon(roof, color, Stroke::NONE));
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(o.x + 2.0, o.y + 8.0),
            Pos2::new(o.x + 14.0, o.y + 16.0),
        ),
        0.0,
        color,
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(o.x + 6.0, o.y + 12.0),
            Pos2::new(o.x + 10.0, o.y + 16.0),
        ),
        0.0,
        TAB,
    );
}

/// The New tab: a grey plus on a tab-coloured square.
fn new_tab(ui: &mut Ui) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(TAB_HEIGHT), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, tab_fill(false, resp.hovered()));
    let c = rect.center();
    let s = Stroke::new(2.0, Color32::from_gray(178));
    painter.line_segment([c - Vec2::new(7.0, 0.0), c + Vec2::new(7.0, 0.0)], s);
    painter.line_segment([c - Vec2::new(0.0, 7.0), c + Vec2::new(0.0, 7.0)], s);
    resp.on_hover_text(tr("menu.file.new")).clicked()
}

enum TabClick {
    None,
    Select,
    Close,
}

fn doc_tab(ui: &mut Ui, i: usize, title: &str, active: bool) -> TabClick {
    let galley = ui.painter().layout_no_wrap(
        title.to_string(),
        egui::FontId::proportional(FONT),
        Color32::BLACK,
    );
    let width = (PAD + galley.size().x + 6.0 + CLOSE + 8.0).max(116.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, TAB_HEIGHT), Sense::click());
    let close_rect = Rect::from_center_size(
        Pos2::new(rect.right() - 8.0 - CLOSE / 2.0, rect.center().y),
        Vec2::splat(CLOSE),
    );
    let close_id = ui.id().with(("doc_tab_close", i));
    let close_resp = ui.interact(close_rect, close_id, Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, tab_fill(active, resp.hovered()));
    painter.galley(
        Pos2::new(
            rect.left() + PAD,
            (rect.center().y - galley.size().y / 2.0).round(),
        ),
        galley,
        Color32::BLACK,
    );
    if resp.hovered() || close_resp.hovered() {
        if close_resp.hovered() {
            painter.rect_filled(close_rect, 2.0, Tokens::BORDER);
        }
        let s = Stroke::new(1.2, Tokens::TEXT_DIM);
        let r = close_rect.shrink(3.5);
        painter.line_segment([r.left_top(), r.right_bottom()], s);
        painter.line_segment([r.right_top(), r.left_bottom()], s);
    }
    let resp = resp.on_hover_text(title);
    if close_resp.clicked() || resp.middle_clicked() {
        TabClick::Close
    } else if resp.clicked() {
        TabClick::Select
    } else {
        TabClick::None
    }
}
