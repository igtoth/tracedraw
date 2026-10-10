//! The document tabs above the rulers: the Welcome Screen, one tab per open
//! drawing (an asterisk marks unsaved changes; the close button shows on
//! the active and the hovered tab) and the New button after the last tab.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

const TAB_HEIGHT: f32 = 22.0;
const PAD: f32 = 10.0;
const CLOSE: f32 = 14.0;

/// What a click on the strip asked for.
enum Action {
    Welcome,
    Switch(usize),
    Close(usize),
    New,
}

pub fn document_tabs(app: &mut App, ui: &mut Ui) {
    let mut action = None;
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
        let plus = ui
            .add_sized(
                [TAB_HEIGHT, TAB_HEIGHT],
                egui::Button::new(egui::RichText::new("+").size(15.0).color(Tokens::TEXT_DIM))
                    .frame(false),
            )
            .on_hover_text(tr("menu.file.new"));
        if plus.clicked() {
            action = Some(Action::New);
        }
    });
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
        Tokens::TOOL_ACTIVE
    } else if hovered {
        Tokens::TOOL_HOVER
    } else {
        Tokens::PANEL_DARK
    }
}

/// The Welcome Screen tab: a house and the title.
fn welcome_tab(ui: &mut Ui, active: bool) -> bool {
    let label = tr("welcome.title");
    let galley = ui
        .painter()
        .layout_no_wrap(label, egui::FontId::proportional(12.0), Tokens::TEXT);
    let icon = 16.0;
    let width = PAD + icon + 6.0 + galley.size().x + PAD + 30.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, TAB_HEIGHT), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, tab_fill(active, resp.hovered()));
    let c = Pos2::new(rect.left() + PAD + icon / 2.0, rect.center().y);
    house(
        painter,
        c,
        if active { Tokens::ACCENT } else { Tokens::ICON },
    );
    painter.galley(
        Pos2::new(
            rect.left() + PAD + icon + 6.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        Tokens::TEXT,
    );
    resp.clicked()
}

fn house(painter: &egui::Painter, c: Pos2, color: Color32) {
    let s = Stroke::new(1.3, color);
    let roof = [
        Pos2::new(c.x - 7.0, c.y),
        Pos2::new(c.x, c.y - 6.5),
        Pos2::new(c.x + 7.0, c.y),
    ];
    painter.line_segment([roof[0], roof[1]], s);
    painter.line_segment([roof[1], roof[2]], s);
    let body = Rect::from_min_max(
        Pos2::new(c.x - 5.0, c.y - 1.0),
        Pos2::new(c.x + 5.0, c.y + 6.0),
    );
    painter.rect_filled(body, 0.0, color);
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(c.x - 1.5, c.y + 2.0),
            Pos2::new(c.x + 1.5, c.y + 6.0),
        ),
        0.0,
        Tokens::PANEL_DARK,
    );
}

enum TabClick {
    None,
    Select,
    Close,
}

fn doc_tab(ui: &mut Ui, i: usize, title: &str, active: bool) -> TabClick {
    let galley = ui.painter().layout_no_wrap(
        title.to_string(),
        egui::FontId::proportional(12.0),
        Tokens::TEXT,
    );
    let width = (PAD + galley.size().x + 6.0 + CLOSE + PAD).max(110.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, TAB_HEIGHT), Sense::click());
    let close_rect = Rect::from_center_size(
        Pos2::new(rect.right() - PAD - CLOSE / 2.0, rect.center().y),
        Vec2::splat(CLOSE),
    );
    let close_id = ui.id().with(("doc_tab_close", i));
    let close_resp = ui.interact(close_rect, close_id, Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, tab_fill(active, resp.hovered()));
    painter.galley(
        Pos2::new(rect.left() + PAD, rect.center().y - galley.size().y / 2.0),
        galley,
        Tokens::TEXT,
    );
    if active || resp.hovered() || close_resp.hovered() {
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
