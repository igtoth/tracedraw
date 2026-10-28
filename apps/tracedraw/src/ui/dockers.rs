//! Dockers: Properties, Objects (layer manager) and Hints.

use crate::app::{App, DockerDrag, DockerPlace, DockerTab};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::Ui;
use tracedraw_core::patch::{FillField, FillMix, StrokeField, StrokeMix};
use tracedraw_core::{
    document::ShapeKind, Arrowhead, Color, Command, Fill, Fountain, FountainKind, FountainRepeat,
    Pattern, PatternTile, Stop, Stroke, Texture, TextureKind,
};

/// The vertical strip of docker tabs on the right edge of the window.
/// Height of a docker's title bar (it runs across the tab column too).
pub const TITLE_H: f32 = 27.0;
/// The slot at the top of each tab where its close cross shows while
/// the pointer is over the tab (#docker tabs).
const CLOSE_SLOT: f32 = 16.0;

/// Where a tab's close cross sits: the slot at the top of the tab `bg`.
pub fn tab_close_rect(bg: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(bg.min.x, bg.min.y + 2.0),
        egui::vec2(bg.width(), CLOSE_SLOT - 2.0),
    )
}

/// The width of a docker tab column.
pub const STRIP_W: f32 = 27.0;

/// A docker tab column: the open dockers of the column at `place` as
/// vertical tabs (an icon on top, the name running downwards), grey, the
/// open one darker, then the button that adds a docker. A tab dragged
/// moves its docker (see [`drag_overlay`]).
pub fn tab_strip(app: &mut App, ui: &mut Ui, place: DockerPlace) {
    let full = ui.max_rect();
    // The title bar's part and the line along the docker.
    ui.painter()
        .rect_filled(full, 0.0, crate::theme::Tokens::tab_strip());
    let line_x = if strip_at_left(place) {
        full.right() - 0.5
    } else {
        full.left() + 0.5
    };
    ui.painter().vline(
        line_x,
        (full.top() + TITLE_H)..=full.bottom(),
        egui::Stroke::new(1.0, crate::theme::Tokens::window_border()),
    );
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.add_space(TITLE_H);
    // More tabs than the column's height: the column slides, with
    // arrows at its ends and the mouse wheel over it, so the last tabs
    // and the add button stay in reach (#950).
    let salt = format!("docker_tabs_{}", place as u8);
    crate::ui::overflow::column(ui, &salt, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let (tabs, front, shown) = app.docker_column(place);
        let tabs = tabs.to_vec();
        for tab in tabs {
            let name = tr(tab.key());
            let active = shown && front == tab;
            let galley = ui.painter().layout_no_wrap(
                name.clone(),
                egui::FontId::proportional(12.0),
                Tokens::text(),
            );
            let w = galley.size().x;
            let gh = galley.size().y;
            let icon_h = if tab_icon(tab).is_some() { 24.0 } else { 0.0 };
            // A slot at the top of the tab for the close cross, shown while
            // the pointer is over the tab.
            let h = CLOSE_SLOT + 14.0 + icon_h + w;
            let (r, resp) =
                ui.allocate_exact_size(egui::vec2(full.width(), h), egui::Sense::click_and_drag());
            let bg = if strip_at_left(place) {
                egui::Rect::from_min_max(r.min, egui::pos2(r.max.x - 1.0, r.max.y))
            } else {
                egui::Rect::from_min_max(egui::pos2(r.min.x + 1.0, r.min.y), r.max)
            };
            let fill = if active {
                crate::theme::Tokens::window_border()
            } else if resp.hovered() {
                crate::theme::Tokens::icon_off()
            } else {
                crate::theme::Tokens::tab()
            };
            ui.painter().rect_filled(bg, 0.0, fill);
            let close = tab_close_rect(bg);
            let over_close = resp.hover_pos().is_some_and(|p| close.contains(p));
            if resp.hovered() {
                let col = if over_close {
                    Tokens::text()
                } else {
                    Tokens::text_dim()
                };
                let c = close.center();
                let s = egui::Stroke::new(1.3, col);
                ui.painter()
                    .line_segment([c + egui::vec2(-3.5, -3.5), c + egui::vec2(3.5, 3.5)], s);
                ui.painter()
                    .line_segment([c + egui::vec2(3.5, -3.5), c + egui::vec2(-3.5, 3.5)], s);
            }
            let resp = if over_close {
                resp.rich_tip(tr("docker.close_docker"))
            } else {
                resp.rich_tip(format!("{name}\n{}", tr("docker.tab_desc")))
            };
            if let Some(icon) = tab_icon(tab) {
                let ir = egui::Rect::from_center_size(
                    egui::pos2(bg.center().x, r.min.y + CLOSE_SLOT + 7.0 + icon_h / 2.0),
                    egui::vec2(16.0, 16.0),
                );
                icon(ui.painter(), ir);
            }
            let text_top = r.min.y + CLOSE_SLOT + 7.0 + icon_h;
            let mut ts = egui::epaint::TextShape::new(
                egui::pos2(bg.center().x + gh / 2.0, text_top),
                galley,
                Tokens::text(),
            );
            if active && crate::theme::theme() == crate::theme::Theme::HighContrast {
                ts.override_text_color = Some(crate::theme::on_highlight());
            }
            ts.angle = std::f32::consts::FRAC_PI_2;
            ui.painter().add(ts);
            if resp.drag_started_by(egui::PointerButton::Primary) {
                let grab = resp
                    .interact_pointer_pos()
                    .map(|p| p - r.min)
                    .unwrap_or(egui::vec2(12.0, 12.0));
                app.docker_drag = Some(DockerDrag {
                    tab,
                    grab: egui::vec2(grab.x.min(60.0), grab.y.min(TITLE_H)),
                    last: None,
                });
            }
            if resp.clicked() {
                let on_close = resp
                    .interact_pointer_pos()
                    .is_some_and(|p| close.contains(p));
                if on_close {
                    app.close_docker(tab);
                } else if active {
                    app.set_docker_column(place, false, None);
                } else {
                    app.set_docker_column(place, true, Some(tab));
                }
            }
            egui::Popup::context_menu(&resp)
                .id(egui::Id::new(("docker_tab_menu", tab.key())))
                .style(crate::ui::menus::menu_popup_style)
                .show(|ui| {
                    crate::ui::menus::body(ui, |ui| {
                        place_menu(app, ui, tab);
                        crate::ui::menus::sep(ui);
                        if crate::ui::menus::item(ui, "docker.close_docker", "", true) {
                            app.close_docker(tab);
                        }
                    })
                });
        }
        // Add a docker.
        ui.add_space(8.0);
        let (r, resp) =
            ui.allocate_exact_size(egui::vec2(full.width(), 22.0), egui::Sense::click());
        let c = r.center() + egui::vec2(0.5, 0.0);
        if resp.hovered() {
            ui.painter().rect_filled(
                egui::Rect::from_center_size(c, egui::vec2(18.0, 18.0)),
                0.0,
                crate::theme::Tokens::icon_off(),
            );
        }
        let s = egui::Stroke::new(1.5, crate::theme::Tokens::text_dim());
        ui.painter()
            .line_segment([c + egui::vec2(-5.0, 0.0), c + egui::vec2(5.0, 0.0)], s);
        ui.painter()
            .line_segment([c + egui::vec2(0.0, -5.0), c + egui::vec2(0.0, 5.0)], s);
        let resp = resp.rich_tip(tr("docker.add_docker"));
        // The panels in the groups of the Window menu, a box ticked at each
        // one open anywhere: ticking opens it in front of this column,
        // unticking closes it.
        egui::Popup::menu(&resp)
            .id(egui::Id::new(("docker_add_menu", place as u8)))
            .style(crate::ui::menus::menu_popup_style)
            .show(|ui| {
                crate::ui::menus::body(ui, |ui| {
                    let row = |ui: &mut Ui, app: &mut App, tab: DockerTab| {
                        let open = app.docker_open(tab);
                        if crate::ui::menus::check(ui, tab.key(), "", open) {
                            if open {
                                app.close_docker(tab);
                            } else {
                                app.move_docker(tab, place, None);
                            }
                        }
                    };
                    for (i, group) in DockerTab::MENU_GROUPS.iter().enumerate() {
                        if i > 0 {
                            crate::ui::menus::sep(ui);
                        }
                        for tab in group.iter() {
                            row(ui, app, *tab);
                        }
                        if i == 5 {
                            crate::ui::menus::sep(ui);
                            for tab in DockerTab::EFFECTS_MENU {
                                row(ui, app, tab);
                            }
                        }
                    }
                })
            });
    });
}

/// The places a docker can go, a dot at the one it has (a tab's menu and
/// a docker's title menu).
fn place_menu(app: &mut App, ui: &mut Ui, tab: DockerTab) {
    let current = app.docker_place(tab);
    for (key, place) in [
        ("bars.right", DockerPlace::Right),
        ("bars.left", DockerPlace::Left),
        ("bars.floating", DockerPlace::Floating),
    ] {
        if crate::ui::menus::radio(ui, key, "", current == Some(place)) && current != Some(place) {
            app.move_docker(tab, place, None);
        }
    }
}

/// A column's tab strip lies at the window's left edge: the other
/// column's (the default column's in a right-to-left language).
fn strip_at_left(place: DockerPlace) -> bool {
    (place == DockerPlace::Left) != crate::i18n::is_rtl()
}

/// Where a docker dropped with the pointer at `p` goes, in a window
/// `screen` wide: within [`crate::ui::bars::EDGE_ZONE`] of a side the
/// column there, elsewhere floating.
pub fn drop_target(screen: egui::Rect, p: egui::Pos2) -> DockerPlace {
    let zone = crate::ui::bars::EDGE_ZONE;
    let (left, right) = if crate::i18n::is_rtl() {
        (DockerPlace::Right, DockerPlace::Left)
    } else {
        (DockerPlace::Left, DockerPlace::Right)
    };
    if p.x <= screen.left() + zone {
        left
    } else if p.x >= screen.right() - zone {
        right
    } else {
        DockerPlace::Floating
    }
}

/// The band (or, floating, the window) a dragged docker would take.
fn drop_zone(
    screen: egui::Rect,
    menu_h: f32,
    place: DockerPlace,
    p: egui::Pos2,
    grab: egui::Vec2,
) -> egui::Rect {
    let top = screen.top() + menu_h;
    let w = STRIP_W + 340.0;
    match place {
        DockerPlace::Floating => {
            egui::Rect::from_min_size(p - grab, crate::app::FloatingDocker::DEFAULT_SIZE)
        }
        place if strip_at_left(place) => egui::Rect::from_min_max(
            egui::pos2(screen.left(), top),
            egui::pos2(screen.left() + w, screen.bottom()),
        ),
        _ => egui::Rect::from_min_max(
            egui::pos2(screen.right() - w, top),
            egui::pos2(screen.right(), screen.bottom()),
        ),
    }
}

/// A docker being moved: the band or window it would take shows under the
/// pointer, and the release puts it there (also when the pointer has left
/// the window: the last place it was seen counts).
pub fn drag_overlay(app: &mut App, ctx: &egui::Context, menu_h: f32) {
    let Some(mut drag) = app.docker_drag else {
        return;
    };
    let screen = ctx.content_rect();
    let (pointer, down) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
    let Some(p) = pointer.or(drag.last) else {
        if !down {
            app.docker_drag = None;
        }
        return;
    };
    drag.last = Some(p);
    app.docker_drag = Some(drag);
    let place = drop_target(screen, p);
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("docker_drag_overlay"),
    ));
    let fill = egui::Color32::from_rgba_unmultiplied(0x1E, 0x78, 0xD7, 46);
    let edge = egui::Stroke::new(1.5, egui::Color32::from_rgb(0x1E, 0x78, 0xD7));
    let zone = drop_zone(screen, menu_h, place, p, drag.grab);
    painter.rect(zone, 2.0, fill, edge, egui::StrokeKind::Inside);
    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    if !down {
        app.docker_drag = None;
        let at = (place == DockerPlace::Floating).then_some(zone.min);
        // Dropped where it already is: nothing moves.
        let same = app.docker_place(drag.tab) == Some(place) && place != DockerPlace::Floating;
        if !same {
            app.move_docker(drag.tab, place, at);
        }
    }
}

/// The floating dockers, each in a window of its own with a title bar
/// (its name; a menu with its places; close), resizable at its edges.
/// Dragging the title moves it (near a side it docks there).
pub fn floating_dockers(app: &mut App, ctx: &egui::Context) {
    let tabs: Vec<DockerTab> = app.floating_dockers.iter().map(|f| f.tab).collect();
    for tab in tabs {
        let Some(fd) = app.floating_dockers.iter_mut().find(|f| f.tab == tab) else {
            continue;
        };
        let screen = ctx.content_rect();
        let mut pos = fd.pos;
        pos.x = pos
            .x
            .clamp(screen.left(), (screen.right() - 60.0).max(screen.left()));
        pos.y = pos
            .y
            .clamp(screen.top(), (screen.bottom() - TITLE_H).max(screen.top()));
        let size = fd.size;
        let place = std::mem::take(&mut fd.place);
        let mut window = egui::Window::new(tr(tab.key()))
            .id(egui::Id::new(("floating_docker", tab.key())))
            .title_bar(false)
            .collapsible(false)
            .movable(false)
            .resizable(true)
            .default_size(size)
            .default_pos(pos)
            .frame(
                egui::Frame::new()
                    .fill(crate::theme::Tokens::field())
                    .stroke(egui::Stroke::new(
                        1.0,
                        crate::theme::Tokens::window_border(),
                    ))
                    .shadow(crate::theme::shadow(egui::epaint::Shadow {
                        offset: [0, 3],
                        blur: 10,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(50),
                    })),
            );
        if place {
            window = window.current_pos(pos).fixed_size(size);
        }
        let shown = window.show(ctx, |ui| {
            ui.set_min_size(ui.available_size());
            docker_title(app, ui, tab, DockerPlace::Floating);
            let content = ui.available_rect_before_wrap();
            let inner = content.shrink2(egui::vec2(8.0, 6.0));
            let mut page = ui.new_child(egui::UiBuilder::new().max_rect(inner));
            page.set_clip_rect(content.intersect(ui.clip_rect()));
            ui.advance_cursor_after_rect(content);
            docker_page(app, &mut page, tab);
        });
        if let Some(shown) = shown {
            if let Some(fd) = app.floating_dockers.iter_mut().find(|f| f.tab == tab) {
                let r = shown.response.rect;
                if r.is_finite() && r.width() >= 120.0 && r.height() >= 120.0 {
                    fd.pos = r.min;
                    fd.size = r.size();
                }
            }
        }
    }
}

type TabIcon = fn(&egui::Painter, egui::Rect);

/// The small picture above a docker tab's name.
fn tab_icon(tab: DockerTab) -> Option<TabIcon> {
    match tab {
        DockerTab::Hints => Some(|p, r| {
            // A pointer with a question mark.
            let s = egui::Stroke::new(1.2, Tokens::icon());
            let o = r.min + egui::vec2(2.0, 1.0);
            let pts = vec![
                o,
                o + egui::vec2(0.0, 11.0),
                o + egui::vec2(3.0, 8.0),
                o + egui::vec2(5.5, 13.0),
                o + egui::vec2(7.0, 12.2),
                o + egui::vec2(4.6, 7.4),
                o + egui::vec2(8.5, 7.4),
            ];
            p.add(egui::epaint::PathShape::convex_polygon(
                pts,
                Tokens::icon(),
                s,
            ));
            p.text(
                r.min + egui::vec2(12.5, 3.5),
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(10.0),
                Tokens::icon(),
            );
        }),
        DockerTab::Properties => Some(|p, r| {
            // A pen over a ruled sheet.
            let s = egui::Stroke::new(1.2, Tokens::icon());
            let c = r.center();
            p.line_segment([c + egui::vec2(-6.0, 6.0), c + egui::vec2(5.0, -5.0)], s);
            p.line_segment([c + egui::vec2(-6.0, 6.0), c + egui::vec2(-3.0, 5.0)], s);
            p.line_segment([c + egui::vec2(3.0, -7.0), c + egui::vec2(7.0, -3.0)], s);
            for y in [2.0, 5.0] {
                p.line_segment([c + egui::vec2(1.0, y), c + egui::vec2(7.0, y)], s);
            }
        }),
        DockerTab::Objects => Some(|p, r| {
            // Three stacked sheets.
            let c = r.center();
            for (k, dy) in [(0u8, 4.0f32), (1, 0.0), (2, -4.0)] {
                let pts = vec![
                    c + egui::vec2(-7.0, dy),
                    c + egui::vec2(0.0, dy - 3.5),
                    c + egui::vec2(7.0, dy),
                    c + egui::vec2(0.0, dy + 3.5),
                ];
                let fill = if k == 2 {
                    Tokens::icon()
                } else {
                    crate::theme::Tokens::field()
                };
                p.add(egui::epaint::PathShape::convex_polygon(
                    pts,
                    fill,
                    egui::Stroke::new(1.0, Tokens::icon()),
                ));
            }
        }),
        _ => None,
    }
}

/// The docker column at `place`: the title bar of the docker in front,
/// then its page in the width the column has.
pub fn dockers(app: &mut App, ui: &mut Ui, place: DockerPlace) {
    let full = ui.max_rect();
    let (_, front, _) = app.docker_column(place);
    docker_title(app, ui, front, place);
    let content = egui::Rect::from_min_max(egui::pos2(full.left(), full.top() + TITLE_H), full.max);
    ui.painter()
        .rect_filled(content, 0.0, crate::theme::Tokens::field());
    // The docker keeps the width it was given (its default or the one the
    // user dragged it to): the page lies in a child that never widens the
    // panel, clipped to the docker, so no section can push it wider.
    let inner = content.shrink2(egui::vec2(8.0, 6.0));
    let mut page = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    page.set_clip_rect(content.intersect(ui.clip_rect()));
    ui.advance_cursor_after_rect(content);
    // The Welcome Screen is no drawing: the dockers that work on one say
    // so instead of showing the hidden drawing's selection (#482); Hints
    // still helps.
    if !app.drawing_shown() && front != DockerTab::Hints {
        page.label(
            egui::RichText::new(crate::i18n::tr("docker.no_drawing"))
                .color(crate::theme::Tokens::text_dim()),
        );
    } else {
        docker_page(app, &mut page, front);
    }
    // How wide the page's widgets came out, for the tests that check
    // that every section keeps to the docker.
    #[cfg(test)]
    {
        let need = page.min_rect().width() - inner.width();
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("docker_overflow"), need));
    }
}

/// A docker's title bar: its name, then collapse (a column's) and close in
/// grey at the right. Dragging it moves the docker; a right click lists
/// its places.
fn docker_title(app: &mut App, ui: &mut Ui, tab: DockerTab, place: DockerPlace) {
    let full = ui.max_rect();
    let (bar, resp) = ui.allocate_exact_size(
        egui::vec2(full.width(), TITLE_H),
        egui::Sense::click_and_drag(),
    );
    let resp = resp.rich_tip(format!("{}\n{}", tr(tab.key()), tr("docker.title_desc")));
    ui.painter()
        .rect_filled(bar, 0.0, crate::theme::Tokens::tab_strip());
    ui.painter().text(
        egui::pos2(bar.left() + 7.0, bar.center().y),
        egui::Align2::LEFT_CENTER,
        tr(tab.key()),
        egui::FontId::proportional(12.0),
        Tokens::text(),
    );
    if resp.drag_started_by(egui::PointerButton::Primary) {
        let grab = resp
            .interact_pointer_pos()
            .map(|p| p - bar.min)
            .unwrap_or(egui::vec2(12.0, 12.0));
        app.docker_drag = Some(DockerDrag {
            tab,
            grab: egui::vec2(grab.x.clamp(0.0, 300.0), grab.y.clamp(0.0, TITLE_H)),
            last: None,
        });
    }
    egui::Popup::context_menu(&resp)
        .id(egui::Id::new(("docker_title_menu", tab.key())))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| {
            crate::ui::menus::body(ui, |ui| {
                place_menu(app, ui, tab);
                crate::ui::menus::sep(ui);
                if crate::ui::menus::item(ui, "docker.close_docker", "", true) {
                    app.close_docker(tab);
                }
            })
        });
    let grey = crate::theme::Tokens::text_dim();
    // Close, at the right end.
    let close = egui::Rect::from_center_size(
        egui::pos2(bar.right() - 12.0, bar.center().y),
        egui::vec2(20.0, 20.0),
    );
    let resp = ui
        .interact(
            close,
            egui::Id::new(("docker_close", tab.key())),
            egui::Sense::click(),
        )
        .rich_tip(tr("docker.close_docker"));
    let col = if resp.hovered() { Tokens::text() } else { grey };
    let c = close.center();
    let s = egui::Stroke::new(1.5, col);
    ui.painter()
        .line_segment([c + egui::vec2(-4.5, -4.5), c + egui::vec2(4.5, 4.5)], s);
    ui.painter()
        .line_segment([c + egui::vec2(4.5, -4.5), c + egui::vec2(-4.5, 4.5)], s);
    if resp.clicked() {
        app.close_docker(tab);
    }
    if place == DockerPlace::Floating {
        return;
    }
    // Collapse: two small triangles.
    let fold = egui::Rect::from_center_size(
        egui::pos2(bar.right() - 36.0, bar.center().y),
        egui::vec2(20.0, 20.0),
    );
    let resp = ui
        .interact(
            fold,
            egui::Id::new(("docker_fold", place as u8)),
            egui::Sense::click(),
        )
        .rich_tip(tr("docker.collapse"));
    let col = if resp.hovered() { Tokens::text() } else { grey };
    for dx in [-3.5f32, 3.0] {
        let c = fold.center() + egui::vec2(dx, 0.0);
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + egui::vec2(-2.5, -4.5),
                c + egui::vec2(2.5, 0.0),
                c + egui::vec2(-2.5, 4.5),
            ],
            col,
            egui::Stroke::NONE,
        ));
    }
    if resp.clicked() {
        app.set_docker_column(place, false, None);
    }
}

/// The page of the docker `tab`: its own layout under the title bar.
fn docker_page(app: &mut App, ui: &mut Ui, tab: DockerTab) {
    // Hints scrolls its page above its own navigation bar.
    if tab == DockerTab::Hints {
        crate::ui::hints::hints_docker(app, ui);
        return;
    }
    // AI Generate keeps Generate at the bottom, under its own scroll.
    if tab == DockerTab::AiGenerate {
        crate::ui::ai_generate_docker::ai_generate(app, ui);
        return;
    }
    // Objects keeps its New Layer and Delete bar under the tree.
    if tab == DockerTab::Objects {
        crate::ui::objects_docker::objects_docker(app, ui);
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt(("docker_scroll", tab.key()))
        .show(ui, |ui| match tab {
            DockerTab::Properties => properties(app, ui),
            DockerTab::Hints | DockerTab::Objects => {}
            DockerTab::Transformations => crate::ui::transform_docker::transformations(app, ui),
            DockerTab::Undo => undo_docker(app, ui),
            other => crate::ui::dockers2::show(app, ui, other),
        });
}

fn color_row(ui: &mut Ui, label: &str, c: &mut Color) -> bool {
    color_row_mixed(ui, label, c, false)
}

/// [`color_row`] for objects whose colours differ when `mixed`: no colour
/// is shown, and the one picked goes to all of them.
fn color_row_mixed(ui: &mut Ui, label: &str, c: &mut Color, mixed: bool) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        // The picker opens beside the swatch and keeps the colour's
        // model.
        let before = ui.cursor().min;
        let ch = crate::ui::color_picker::color_button(ui, c);
        let swatch = egui::Rect::from_min_size(before, egui::vec2(40.0, 18.0));
        if mixed {
            crate::ui::field::mixed_swatch(ui, swatch);
            crate::ui::field::mixed_note(ui);
        } else {
            ui.label(
                egui::RichText::new(crate::app::color_description(*c))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
        }
        ch
    })
    .inner
}

/// Give the selection these further fills and outlines (one undo step).
fn set_extra_paint(app: &mut App, fills: Vec<Fill>, strokes: Vec<tracedraw_core::Stroke>) {
    let shapes = app.selection.clone();
    app.run(Command::SetExtraPaint {
        shapes,
        fills,
        strokes,
    });
}

/// The further fills of the selection (the first object's): each with
/// its editor and a cross; Add Fill puts a copy of the fill on top. It
/// shows only while the object has a fill (`has_fill`): the first fill
/// comes from the kinds above it, not from this button (#487).
fn extra_fills(app: &mut App, ui: &mut Ui, first: &tracedraw_core::Shape, has_fill: bool) {
    let mut fills = first.extra_fills.clone();
    let mut changed = false;
    let mut remove = None;
    for (i, f) in fills.iter_mut().enumerate() {
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong(trf("docker.fill_n", &[("n", &(i + 2).to_string())]));
            if ui
                .small_button("\u{00D7}")
                .rich_tip(tr("docker.remove_fill"))
                .clicked()
            {
                remove = Some(i);
            }
        });
        ui.push_id(("extra_fill", i), |ui| {
            changed |= fill_editor(ui, f);
        });
    }
    if let Some(i) = remove {
        fills.remove(i);
        changed = true;
    }
    if has_fill {
        ui.add_space(4.0);
    }
    if has_fill
        && ui
            .button(tr("docker.add_fill"))
            .rich_tip(tr("docker.add_fill_tip"))
            .clicked()
    {
        fills.push(first.fill.clone());
        changed = true;
    }
    if changed {
        set_extra_paint(app, fills, first.extra_strokes.clone());
    }
}

/// The further outlines of the selection, as [`extra_fills`]: Add Outline
/// shows only while the object has an outline (`has_outline`); the
/// Outline box gives it its first one (#487).
fn extra_strokes(app: &mut App, ui: &mut Ui, first: &tracedraw_core::Shape, has_outline: bool) {
    let mut strokes = first.extra_strokes.clone();
    let custom = crate::ui::line_style::OutlineLib::of(app);
    let mut changed = false;
    let mut remove = None;
    for (i, st) in strokes.iter_mut().enumerate() {
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong(trf("docker.outline_n", &[("n", &(i + 2).to_string())]));
            if ui
                .small_button("\u{00D7}")
                .rich_tip(tr("docker.remove_outline"))
                .clicked()
            {
                remove = Some(i);
            }
        });
        ui.push_id(("extra_stroke", i), |ui| {
            let mut s = Some(st.clone());
            if outline_editor(ui, &mut s, &custom) {
                if let Some(s) = s {
                    *st = s;
                }
                changed = true;
            }
        });
    }
    if let Some(i) = remove {
        strokes.remove(i);
        changed = true;
    }
    if has_outline {
        ui.add_space(4.0);
    }
    if has_outline
        && ui
            .button(tr("docker.add_outline"))
            .rich_tip(tr("docker.add_outline_tip"))
            .clicked()
    {
        let base = first
            .stroke
            .clone()
            .unwrap_or_else(|| tracedraw_core::Stroke::hairline(tracedraw_core::Color::BLACK));
        strokes.push(base);
        changed = true;
    }
    if changed {
        set_extra_paint(app, first.extra_fills.clone(), strokes);
    }
}

/// Where the fill editors find the current colours (see
/// `App::current_colors`).
fn current_colors_id() -> egui::Id {
    egui::Id::new("fill_editor_current_colors")
}

fn properties(app: &mut App, ui: &mut Ui) {
    let cc = app.current_colors;
    ui.data_mut(|d| d.insert_temp(current_colors_id(), cc));
    let shapes = app.selected_rc();
    if shapes.is_empty() {
        ui.label(egui::RichText::new(tr("docker.no_objects_selected")).color(Tokens::text_dim()));
        ui.add_space(6.0);
        ui.strong(tr("docker.default_properties"));
        ui.add_space(4.0);
        // The same sections, in the same order, as with an object
        // selected, so each row of paint kinds sits under its own title
        // (#487).
        if section_header(app, ui, "outline", &tr("docker.outline")) {
            ui.add_space(2.0);
            let mut stroke = app.default_stroke();
            let custom = crate::ui::line_style::OutlineLib::of(app);
            if outline_editor(ui, &mut stroke, &custom) {
                app.set_default_properties(tracedraw_core::DefaultKind::Graphic, |p| {
                    p.stroke = stroke
                });
            }
            ui.add_space(6.0);
        }
        if section_header(app, ui, "fill", &tr("docker.fill")) {
            ui.add_space(2.0);
            let mut fill = app.default_fill();
            if fill_editor(ui, &mut fill) {
                app.set_default_properties(tracedraw_core::DefaultKind::Graphic, |p| p.fill = fill);
            }
        }
        return;
    }
    let first = &shapes[0];
    ui.strong(format!(
        "{}{}",
        kind_name(&first.kind),
        if shapes.len() > 1 {
            trf(
                "docker.and_n_more",
                &[("n", &(shapes.len() - 1).to_string())],
            )
        } else {
            String::new()
        }
    ));
    let b = app.selection_bounds().unwrap_or_default();
    let o = app.to_ruler(tracedraw_core::geometry::Point::new(b.x0, b.y0));
    ui.label(
        egui::RichText::new(format!(
            "x {:.2}  y {:.2}  w {:.2}  h {:.2} {}",
            app.units.world_from_mm(o.x),
            app.units.world_from_mm(o.y),
            app.units.world_from_mm(b.width()),
            app.units.world_from_mm(b.height()),
            app.units.short()
        ))
        .monospace()
        .size(11.0),
    );

    ui.separator();
    // A jump from elsewhere (F11 to the fill, the outline from the status
    // bar) opens that section.
    if let Some(i) = app.properties_open.take() {
        let id = if i == 0 { "fill" } else { "outline" };
        app.settings.properties_closed.retain(|c| c != id);
    }
    // The sections this object has, in the docker's order, with the
    // buttons that open and close them.
    let mut sections: Vec<(&'static str, &'static str, SectionIcon)> = Vec::new();
    // A QR code object's own settings come first.
    if first.qr.is_some() {
        sections.push(("qr", "qr.section", SectionIcon::Qr));
    }
    sections.extend([
        (
            "outline",
            "docker.outline",
            SectionIcon::Tool(crate::tools::Tool::OutlinePen),
        ),
        (
            "fill",
            "docker.fill",
            SectionIcon::Tool(crate::tools::Tool::InteractiveFill),
        ),
        (
            "transparency",
            "docker.transparency",
            SectionIcon::Tool(crate::tools::Tool::Transparency),
        ),
    ]);
    let corners = first.page_corners();
    if corners.is_some() {
        sections.push((
            "corners",
            "kind.rectangle",
            SectionIcon::Tool(crate::tools::Tool::Rectangle),
        ));
    }
    // Bitmap effects: a bitmap's, or a vector object's.
    sections.push(("effects", "docker.fx", SectionIcon::Effects));
    if matches!(first.kind, ShapeKind::Text { .. }) {
        sections.push((
            "character",
            "docker.character",
            SectionIcon::Tool(crate::tools::Tool::Text),
        ));
    }
    sections.push(("summary", "docker.summary", SectionIcon::Summary));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (id, key, icon) in &sections {
            let open = !app.settings.properties_closed.iter().any(|c| c == id);
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 26.0), egui::Sense::click());
            let fill = if open {
                Some(Tokens::tool_active())
            } else if resp.hovered() {
                Some(Tokens::tool_hover())
            } else {
                None
            };
            if let Some(c) = fill {
                ui.painter().rect_filled(rect, 2.0, c);
            }
            icon.paint(
                ui.painter(),
                egui::Rect::from_center_size(rect.center(), egui::vec2(18.0, 18.0)),
            );
            if resp.rich_tip(tr(key)).clicked() {
                toggle_section(app, id);
            }
        }
    });
    ui.add_space(2.0);
    for (id, key, _) in sections {
        if !section_header(app, ui, id, &tr(key)) {
            continue;
        }
        ui.add_space(2.0);
        match id {
            "qr" => {
                if let Some(qr) = &first.qr {
                    qr_section(app, ui, first.id, qr);
                }
            }
            "outline" if app.text_selection_active() => {
                // Characters selected in the text being edited: their own
                // outline, and an edit goes to them alone.
                let old = app.text_selection_stroke().unwrap_or(None);
                let mut stroke = old.clone();
                let custom = crate::ui::line_style::OutlineLib::of(app);
                if outline_editor(ui, &mut stroke, &custom) {
                    app.text_apply_outline(stroke);
                }
            }
            "outline" => {
                // A group shows what is inside it; values that differ
                // across the selection show as mixed, and an edit changes
                // only what was edited on each object.
                let base = app.first_paint_target();
                let old = base
                    .as_ref()
                    .map_or(first.stroke.clone(), |b| b.stroke.clone());
                let mix = app.selection_stroke_mix();
                let mut stroke = old.clone();
                let custom = crate::ui::line_style::OutlineLib::of(app);
                let mut touched = Vec::new();
                if outline_editor_mixed(ui, &mut stroke, &custom, &mix, &mut touched) {
                    // Dragging in the picker (or a slider): one step, and
                    // a new colour joins the document palette on release.
                    let live = app.live_stroke.is_some() || ui.input(|i| i.pointer.any_down());
                    if live {
                        app.patch_selection_stroke_live(&old, &stroke, &touched);
                    } else {
                        app.patch_selection_stroke(&old, &stroke, &touched);
                    }
                }
                let has = stroke.is_some() || mix.presence;
                extra_strokes(app, ui, first, has);
            }
            "fill" if app.text_selection_active() => {
                // Characters selected in the text being edited: their own
                // fill, and an edit goes to them alone.
                let old = app.text_selection_fill().unwrap_or(Fill::None);
                let mut fill = old.clone();
                if fill_editor(ui, &mut fill) {
                    app.text_apply_fill(fill);
                }
            }
            "fill" => {
                let base = app.first_paint_target();
                let old = base.as_ref().map_or(first.fill.clone(), |b| b.fill.clone());
                let mix = app.selection_fill_mix();
                let mut fill = old.clone();
                let mut touched = Vec::new();
                let edited = fill_editor_mixed(ui, &mut fill, &mix, &mut touched);
                let has = !matches!(fill, Fill::None) || mix.kind;
                if edited {
                    // Dragging in the picker: one step, and the colour
                    // joins the document palette on release.
                    let live = app.live_fill.is_some() || ui.input(|i| i.pointer.any_down());
                    if live && !matches!(fill, Fill::Fountain(_)) {
                        app.apply_fill_live(fill);
                    } else {
                        app.patch_selection_fill(&old, &fill, &touched);
                    }
                }
                extra_fills(app, ui, first, has);
            }
            "transparency" => {
                crate::ui::toolbar::transparency_section(app, ui);
            }
            "corners" => {
                if let Some(c) = corners {
                    rectangle_section(app, ui, c);
                }
            }
            "effects" => {
                let id = first.id;
                crate::ui::effect_dialog::fx_section(app, ui, id);
            }
            "character" => {
                if let ShapeKind::Text { spans, .. } = &first.kind {
                    // The selected characters' span while text is edited,
                    // else the first span.
                    let chosen = app.text_selection_span();
                    if let Some(sp) = chosen.as_ref().or(spans.first()) {
                        let u = app.text_units();
                        ui.label(format!(
                            "{} {} {}{}{}",
                            sp.font_family,
                            crate::ui::field::format_number(
                                u.from_pt(sp.size_pt),
                                false,
                                0,
                                Some(2)
                            ),
                            u.short(),
                            if sp.bold {
                                format!(" {}", tr("docker.bold"))
                            } else {
                                String::new()
                            },
                            if sp.italic {
                                format!(" {}", tr("docker.italic"))
                            } else {
                                String::new()
                            }
                        ));
                    }
                    ui.label(
                        egui::RichText::new(tr("docker.edit_with_text_tool"))
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                }
            }
            _ => {
                for s in shapes.iter() {
                    ui.label(format!(
                        "{}  id {}  {}",
                        kind_name(&s.kind),
                        s.id.raw(),
                        s.name.clone().unwrap_or_default()
                    ));
                }
            }
        }
        ui.add_space(6.0);
    }
}

/// The QR Code section of Properties: the kind of content and its
/// fields, the fills of the dark pixels and the background, the pixels'
/// outline, the pixel size, and Validate. Each change makes the code again
/// (one undo step); content that cannot be encoded is shown in red and
/// changes nothing.
fn qr_section(
    app: &mut App,
    ui: &mut Ui,
    id: tracedraw_core::ShapeId,
    qr: &tracedraw_core::QrCode,
) {
    use tracedraw_core::QrKind;
    let mut next = qr.clone();
    let mut changed = false;
    let msg_id = egui::Id::new(("qr_message", id.raw()));
    egui::ComboBox::from_id_salt(("qr_kind", id.raw()))
        .width(ui.available_width() - 8.0)
        .selected_text(tr(qr_kind_key(qr.kind)))
        .show_ui(ui, |ui| {
            for k in QrKind::ALL {
                if ui
                    .selectable_label(qr.kind == k, tr(qr_kind_key(k)))
                    .clicked()
                    && k != qr.kind
                {
                    next.fields = k.carry(qr.kind, &qr.fields);
                    next.kind = k;
                    changed = true;
                }
            }
        });
    // The fields are edited as text and committed when they lose focus or
    // Enter is pressed, so typing does not add an undo step per letter.
    for (i, key) in qr.kind.fields().iter().enumerate() {
        let edit_id = egui::Id::new(("qr_field", id.raw(), i));
        let mut text: String = ui
            .data(|d| d.get_temp::<String>(edit_id))
            .unwrap_or_else(|| qr.fields.get(i).cloned().unwrap_or_default());
        ui.label(egui::RichText::new(tr(key)).size(11.0));
        let multi = matches!(qr.kind, QrKind::Text) || key.ends_with("message");
        let resp = if multi {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            )
        } else {
            ui.add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY))
        };
        if resp.has_focus() {
            ui.data_mut(|d| d.insert_temp(edit_id, text.clone()));
        }
        let commit = resp.lost_focus() || (resp.changed() && !resp.has_focus());
        if commit {
            ui.data_mut(|d| d.remove::<String>(edit_id));
            if next.fields.len() <= i {
                next.fields.resize(i + 1, String::new());
            }
            if next.fields[i] != text {
                next.fields[i] = text;
                changed = true;
            }
        }
    }
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("qr.dark_fill")).size(11.0));
    if fill_editor(ui, &mut next.dark) {
        changed = true;
    }
    ui.label(egui::RichText::new(tr("qr.light_fill")).size(11.0));
    if fill_editor(ui, &mut next.light) {
        changed = true;
    }
    ui.label(egui::RichText::new(tr("qr.outline")).size(11.0));
    let custom = crate::ui::line_style::OutlineLib::of(app);
    if outline_editor(ui, &mut next.outline, &custom) {
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.label(tr("qr.pixel_size"));
        let units = app.units;
        let mut v = units.from_mm(next.pixel_mm);
        if ui
            .add(
                crate::ui::field::NumField::new(&mut v)
                    .speed(0.05)
                    .max_decimals(3)
                    .range(units.from_mm(0.1)..=units.from_mm(50.0))
                    .suffix(format!(" {}", units.short())),
            )
            .changed()
        {
            next.pixel_mm = units.to_mm(v).clamp(0.1, 50.0);
            changed = true;
        }
    });
    if ui
        .add_sized(
            [ui.available_width() - 8.0, 24.0],
            egui::Button::new(tr("qr.validate")),
        )
        .clicked()
    {
        let m = match crate::qr::validate(qr) {
            Ok(_) => (true, tr("qr.validate_ok")),
            Err(e) => (false, e),
        };
        ui.data_mut(|d| d.insert_temp(msg_id, m));
    }
    if changed {
        let result = app.set_qr_code(id, next);
        ui.data_mut(|d| match result {
            Ok(()) => {
                d.remove::<(bool, String)>(msg_id);
            }
            Err(e) => {
                d.insert_temp(msg_id, (false, e));
            }
        });
    }
    if let Some((ok, m)) = ui.data(|d| d.get_temp::<(bool, String)>(msg_id)) {
        let colour = if ok {
            egui::Color32::from_rgb(0x10, 0x7C, 0x10)
        } else {
            egui::Color32::from_rgb(0xC4, 0x2B, 0x1C)
        };
        ui.label(egui::RichText::new(m).color(colour).size(11.0));
    }
}

/// The name of a kind of QR content.
fn qr_kind_key(k: tracedraw_core::QrKind) -> &'static str {
    use tracedraw_core::QrKind;
    match k {
        QrKind::Url => "qr.kind_url",
        QrKind::Email => "qr.kind_email",
        QrKind::Phone => "qr.kind_phone",
        QrKind::Sms => "qr.kind_sms",
        QrKind::Contact => "qr.kind_contact",
        QrKind::Text => "qr.kind_text",
        QrKind::Location => "qr.kind_location",
    }
}

/// The picture on a Properties section's button.
#[derive(Debug, Clone, Copy)]
enum SectionIcon {
    Tool(crate::tools::Tool),
    Effects,
    Summary,
    Qr,
}

impl SectionIcon {
    fn paint(self, painter: &egui::Painter, r: egui::Rect) {
        match self {
            SectionIcon::Tool(t) => crate::ui::icons::draw(painter, r, t, Tokens::icon()),
            SectionIcon::Effects => {
                painter.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "fx",
                    egui::FontId::proportional(13.0),
                    Tokens::icon(),
                );
            }
            SectionIcon::Qr => {
                // Three finder squares and a few pixels.
                let px = r.width() / 7.0;
                let stroke = egui::Stroke::new(1.2, Tokens::icon());
                for (cx, cy) in [(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)] {
                    let sq = egui::Rect::from_min_size(
                        r.min + egui::vec2(cx * px, cy * px),
                        egui::vec2(3.0 * px, 3.0 * px),
                    );
                    painter.rect_stroke(sq.shrink(0.6), 0.0, stroke, egui::StrokeKind::Inside);
                    painter.rect_filled(sq.shrink(px), 0.0, Tokens::icon());
                }
                for (cx, cy) in [(4.0, 4.0), (6.0, 5.0), (5.0, 6.0), (4.0, 6.0)] {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            r.min + egui::vec2(cx * px, cy * px),
                            egui::vec2(px, px),
                        ),
                        0.0,
                        Tokens::icon(),
                    );
                }
            }
            SectionIcon::Summary => {
                painter.circle_stroke(r.center(), 7.0, egui::Stroke::new(1.3, Tokens::icon()));
                painter.text(
                    r.center() + egui::vec2(0.0, 0.5),
                    egui::Align2::CENTER_CENTER,
                    "i",
                    egui::FontId::proportional(11.0),
                    Tokens::icon(),
                );
            }
        }
    }
}

/// Open or close a Properties section (kept in the preferences).
fn toggle_section(app: &mut App, id: &str) {
    let closed = &mut app.settings.properties_closed;
    if closed.iter().any(|c| c == id) {
        closed.retain(|c| c != id);
    } else {
        closed.push(id.to_string());
    }
}

/// A Properties section's header: a bar with the open or closed triangle
/// and the title; a click opens or closes the section. True when open.
fn section_header(app: &mut App, ui: &mut Ui, id: &str, title: &str) -> bool {
    let open = !app.settings.properties_closed.iter().any(|c| c == id);
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
    let fill = if resp.hovered() {
        Tokens::tool_hover()
    } else {
        crate::theme::Tokens::control_fill()
    };
    ui.painter().rect_filled(rect, 2.0, fill);
    let c = egui::pos2(rect.left() + 10.0, rect.center().y);
    let points = if open {
        vec![
            c + egui::vec2(-4.0, -2.0),
            c + egui::vec2(4.0, -2.0),
            c + egui::vec2(0.0, 3.0),
        ]
    } else {
        vec![
            c + egui::vec2(-2.0, -4.0),
            c + egui::vec2(3.0, 0.0),
            c + egui::vec2(-2.0, 4.0),
        ]
    };
    ui.painter().add(egui::epaint::PathShape::convex_polygon(
        points,
        Tokens::icon(),
        egui::Stroke::NONE,
    ));
    ui.painter().text(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        crate::theme::bold(12.5),
        Tokens::text(),
    );
    if resp.clicked() {
        toggle_section(app, id);
        return !open;
    }
    open
}

/// The Properties docker's rectangle part: corner style, the four corner
/// sizes, editing them together and relative corner scaling, as on the
/// property bar.
fn rectangle_section(app: &mut App, ui: &mut Ui, c: tracedraw_core::Corners) {
    use crate::ui::propbar::{edit_corners, CornerEdit};
    use tracedraw_core::{CornerKind, Corners};
    let mut edit = None;
    // Values that differ across the selected rectangles show as mixed.
    let mix = app.selection_corner_mix().map_or([false; 6], |(_, m)| m);
    // The corner kinds one under the other, so that long names keep to
    // the docker's width.
    for (kind, key) in [
        (CornerKind::Round, "toolbar.corner_round"),
        (CornerKind::Scallop, "toolbar.corner_scallop"),
        (CornerKind::Chamfer, "toolbar.corner_chamfer"),
    ] {
        if ui.radio(!mix[0] && c.kind == kind, tr(key)).clicked() {
            edit = Some(CornerEdit::Kind(kind));
        }
    }
    let u = app.units;
    // The four radii as the corners lie, two by two, each named above
    // its field. Both columns take half the docker, and a name that does
    // not fit goes on to a second line rather than being cut (#487).
    let gap = 10.0;
    let column = ((ui.available_width() - gap) / 2.0).max(60.0);
    let field_w = column.min(96.0);
    egui::Grid::new("docker_corners")
        .num_columns(2)
        .spacing([gap, 4.0])
        .min_col_width(column)
        .max_col_width(column)
        .show(ui, |ui| {
            for pair in [
                [
                    (Corners::TOP_LEFT, "toolbar.corner_top_left"),
                    (Corners::TOP_RIGHT, "toolbar.corner_top_right"),
                ],
                [
                    (Corners::BOTTOM_LEFT, "toolbar.corner_bottom_left"),
                    (Corners::BOTTOM_RIGHT, "toolbar.corner_bottom_right"),
                ],
            ] {
                for (i, key) in pair {
                    ui.vertical(|ui| {
                        ui.set_width(column);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(tr(key))
                                    .color(Tokens::text_dim())
                                    .size(11.0),
                            )
                            .wrap(),
                        );
                        let mut v = u.world_from_mm(c.radii[i]);
                        if mix.get(i + 1).copied().unwrap_or(false) {
                            // Empty until a size is typed, which goes to all.
                            if ui
                                .add(
                                    crate::ui::field::NumField::new(&mut v)
                                        .range(0.0..=10_000.0)
                                        .suffix(format!(" {}", u.short()))
                                        .width(field_w)
                                        .mixed(true),
                                )
                                .changed()
                            {
                                edit = Some(CornerEdit::Size(i, u.world_to_mm(v)));
                            }
                        } else {
                            let r = ui.add(
                                crate::ui::field::NumField::new(&mut v)
                                    .range(0.0..=10_000.0)
                                    .speed(0.1)
                                    .suffix(format!(" {}", u.short()))
                                    .width(field_w),
                            );
                            if r.changed() && (u.world_to_mm(v) - c.radii[i]).abs() > 1e-12 {
                                edit = Some(CornerEdit::Size(i, u.world_to_mm(v)));
                            }
                        }
                    });
                }
                ui.end_row();
            }
        });
    ui.checkbox(&mut app.corners_together, tr("toolbar.corners_together"));
    let mut relative = !c.fixed;
    if ui
        .add(
            egui::Checkbox::new(&mut relative, tr("toolbar.relative_corners"))
                .indeterminate(mix[5]),
        )
        .changed()
    {
        edit = Some(CornerEdit::Scaling(relative));
    }
    if let Some(e) = edit {
        edit_corners(app, e);
    }
}

/// The picture of a fill kind (0 none, 1 uniform, 2 gradient, 3 pattern,
/// 4 texture, 5 mesh) in a 16 x 16 square.
pub(crate) fn fill_kind_picture(painter: &egui::Painter, r: egui::Rect, kind: usize) {
    let edge = egui::Stroke::new(1.0, Tokens::icon());
    let sq = r.shrink(1.0);
    match kind {
        0 => {
            painter.rect_stroke(sq, 0.0, edge, egui::StrokeKind::Inside);
            painter.line_segment(
                [sq.left_bottom(), sq.right_top()],
                egui::Stroke::new(1.3, egui::Color32::from_rgb(0xD0, 0x30, 0x30)),
            );
        }
        1 => {
            painter.rect_filled(sq, 0.0, Tokens::icon());
        }
        2 => {
            let mut mesh = egui::Mesh::default();
            let dark = Tokens::icon();
            let light = egui::Color32::WHITE;
            mesh.colored_vertex(sq.left_top(), dark);
            mesh.colored_vertex(sq.right_top(), light);
            mesh.colored_vertex(sq.right_bottom(), light);
            mesh.colored_vertex(sq.left_bottom(), dark);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(mesh);
            painter.rect_stroke(sq, 0.0, edge, egui::StrokeKind::Inside);
        }
        3 => {
            let n = 4;
            let w = sq.width() / n as f32;
            for y in 0..n {
                for x in 0..n {
                    if (x + y) % 2 == 0 {
                        let cell = egui::Rect::from_min_size(
                            sq.min + egui::vec2(x as f32 * w, y as f32 * w),
                            egui::vec2(w, w),
                        );
                        painter.rect_filled(cell, 0.0, Tokens::icon());
                    }
                }
            }
            painter.rect_stroke(sq, 0.0, edge, egui::StrokeKind::Inside);
        }
        4 => {
            // Speckles of a texture.
            for (x, y, rad) in [
                (3.0, 4.0, 1.6),
                (9.0, 3.0, 1.2),
                (12.0, 9.0, 1.8),
                (5.0, 11.0, 1.4),
                (8.5, 7.5, 1.0),
                (12.5, 13.0, 1.0),
            ] {
                painter.circle_filled(sq.min + egui::vec2(x, y), rad, Tokens::icon());
            }
            painter.rect_stroke(sq, 0.0, edge, egui::StrokeKind::Inside);
        }
        _ => {
            painter.rect_stroke(sq, 0.0, edge, egui::StrokeKind::Inside);
            for t in [1.0 / 3.0, 2.0 / 3.0] {
                let x = sq.left() + sq.width() * t;
                let y = sq.top() + sq.height() * t;
                painter.line_segment(
                    [egui::pos2(x, sq.top()), egui::pos2(x + 1.5, sq.bottom())],
                    egui::Stroke::new(0.9, Tokens::icon()),
                );
                painter.line_segment(
                    [egui::pos2(sq.left(), y), egui::pos2(sq.right(), y - 1.5)],
                    egui::Stroke::new(0.9, Tokens::icon()),
                );
            }
        }
    }
}

fn fill_editor(ui: &mut Ui, fill: &mut Fill) -> bool {
    fill_editor_mixed(ui, fill, &FillMix::default(), &mut Vec::new())
}

/// [`fill_editor`] for a selection whose fills differ as `mix` says: no
/// type is highlighted when the types differ (and no settings are shown
/// until one is picked), a uniform colour that differs shows no colour.
/// A choice that equals the value shown still applies to every object:
/// it is added to `touched`.
fn fill_editor_mixed(
    ui: &mut Ui,
    fill: &mut Fill,
    mix: &FillMix,
    touched: &mut Vec<FillField>,
) -> bool {
    let mut changed = false;
    let mut kind = tracedraw_core::patch::fill_kind(fill);
    let shown_kind = (!mix.kind).then_some(kind);
    // The kinds as pictures: none, uniform, gradient, pattern, texture,
    // mesh.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, n) in [
            "docker.fill_none",
            "docker.fill_uniform",
            "docker.fill_fountain",
            "docker.fill_pattern",
            "docker.fill_texture",
            "docker.fill_mesh",
        ]
        .iter()
        .enumerate()
        {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 26.0), egui::Sense::click());
            if shown_kind == Some(i) {
                ui.painter().rect_filled(rect, 2.0, Tokens::tool_active());
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
            }
            fill_kind_picture(
                ui.painter(),
                egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0)),
                i,
            );
            if resp.rich_tip(tr(n)).clicked() && shown_kind != Some(i) {
                if kind == i {
                    // The type the first object has, picked for objects
                    // of other types: they all take its fill.
                    touched.push(FillField::All);
                }
                kind = i;
                changed = true;
            }
        }
    });
    if mix.kind && !changed {
        crate::ui::field::mixed_note(ui);
        return false;
    }
    if changed && tracedraw_core::patch::fill_kind(fill) != kind {
        let base = fill
            .preview_color()
            .unwrap_or(Color::cmyk_pct(0.0, 0.0, 0.0, 20.0));
        *fill = match kind {
            0 => Fill::None,
            1 => Fill::Solid(base),
            2 => Fill::linear(base, Color::WHITE, 0.0),
            3 => Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Checker,
                front: base,
                back: Color::WHITE,
                size_mm: 10.0,
            }),
            4 => Fill::Texture(Texture {
                kind: TextureKind::Clouds,
                color_a: base,
                color_b: Color::WHITE,
                scale: 20.0,
                seed: 1,
            }),
            _ => Fill::Mesh(tracedraw_core::Mesh::new(
                tracedraw_core::geometry::Rect::new(0.0, 0.0, 100.0, 100.0),
                2,
                2,
                base,
            )),
        };
    }
    if !mix.kind && !mix.fields.is_empty() && !matches!(fill, Fill::Solid(_)) {
        crate::ui::field::mixed_note(ui);
    }
    match fill {
        Fill::None => {}
        Fill::Solid(c) => {
            let mixed = !mix.kind && mix.has(FillField::All);
            if color_row_mixed(ui, &tr("docker.colour"), c, mixed) {
                changed = true;
                if mixed {
                    touched.push(FillField::All);
                }
            }
        }
        Fill::Fountain(f) => changed |= fountain_editor(ui, f),
        Fill::Pattern(p) => changed |= pattern_editor(ui, p),
        Fill::Texture(t) => changed |= texture_editor(ui, t),
        Fill::Mesh(m) => {
            ui.label(trf(
                "docker.mesh_n",
                &[("r", &m.rows.to_string()), ("c", &m.cols.to_string())],
            ));
            ui.horizontal(|ui| {
                ui.label(tr("docker.grid"));
                let (mut r, mut c) = (m.rows, m.cols);
                if ui
                    .add(crate::ui::field::NumField::new(&mut r).range(1..=50))
                    .changed()
                    || ui
                        .add(crate::ui::field::NumField::new(&mut c).range(1..=50))
                        .changed()
                {
                    // The colours stay where the old grid had them.
                    *m = m.regrid(r, c);
                    changed = true;
                }
            });
            changed |= ui
                .checkbox(&mut m.smooth, tr("docker.smooth_colour"))
                .changed();
            ui.label(
                egui::RichText::new(tr("docker.mesh_hint"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
        }
    }
    changed
}

/// Stops drawn left to right as one mesh.
pub(crate) fn gradient_swatch(p: &egui::Painter, r: egui::Rect, stops: &[tracedraw_core::Stop]) {
    let mut mesh = egui::Mesh::default();
    for (i, st) in stops.iter().enumerate() {
        let [cr, cg, cb] = st.color.to_rgb8();
        let c = egui::Color32::from_rgb(cr, cg, cb);
        let x = r.left() + r.width() * st.pos.clamp(0.0, 1.0) as f32;
        mesh.colored_vertex(egui::pos2(x, r.top()), c);
        mesh.colored_vertex(egui::pos2(x, r.bottom()), c);
        if i > 0 {
            let a = (i as u32 - 1) * 2;
            mesh.add_triangle(a, a + 1, a + 2);
            mesh.add_triangle(a + 1, a + 3, a + 2);
        }
    }
    p.add(egui::Shape::mesh(mesh));
}

/// Stops drawn bottom to top as one mesh.
fn gradient_swatch_up(p: &egui::Painter, r: egui::Rect, stops: &[tracedraw_core::Stop]) {
    let mut mesh = egui::Mesh::default();
    for (i, st) in stops.iter().enumerate() {
        let [cr, cg, cb] = st.color.to_rgb8();
        let c = egui::Color32::from_rgb(cr, cg, cb);
        let y = r.bottom() - r.height() * st.pos.clamp(0.0, 1.0) as f32;
        mesh.colored_vertex(egui::pos2(r.left(), y), c);
        mesh.colored_vertex(egui::pos2(r.right(), y), c);
        if i > 0 {
            let a = (i as u32 - 1) * 2;
            mesh.add_triangle(a, a + 1, a + 2);
            mesh.add_triangle(a + 1, a + 3, a + 2);
        }
    }
    p.add(egui::Shape::mesh(mesh));
}

/// The i18n key of a fountain kind's short name.
pub fn fountain_kind_key(k: FountainKind) -> &'static str {
    match k {
        FountainKind::Linear => "fill.linear",
        FountainKind::Radial => "fill.radial",
        FountainKind::Conical => "fill.conical",
        FountainKind::Square => "fill.square",
        FountainKind::Shaped => "fill.shaped",
    }
}

fn fountain_editor(ui: &mut Ui, f: &mut Fountain) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("docker.type"));
        for k in FountainKind::ALL {
            if ui
                .selectable_label(f.kind == k, tr(fountain_kind_key(k)))
                .clicked()
                && f.kind != k
            {
                f.kind = k;
                changed = true;
            }
        }
    });
    // The whole run as it is drawn: repeats, mirror and inversion in.
    let (r, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(240.0), 14.0),
        egui::Sense::hover(),
    );
    gradient_swatch(ui.painter(), r, &f.resolved_stops());
    ui.painter().rect_stroke(
        r,
        2.0,
        egui::Stroke::new(1.0, crate::theme::Tokens::border()),
        egui::StrokeKind::Inside,
    );
    // Built-in gradients: a click replaces the stops.
    egui::CollapsingHeader::new(tr("docker.gradient_presets"))
        .id_salt("fountain_presets")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                // First, the current colours: the last fill colour picked
                // to the last outline colour picked.
                let (a, b) = ui
                    .data(|d| d.get_temp::<(Color, Color)>(current_colors_id()))
                    .unwrap_or((Color::BLACK, Color::WHITE));
                let current = vec![Stop::new(0.0, a), Stop::new(1.0, b)];
                let (r, resp) =
                    ui.allocate_exact_size(egui::vec2(34.0, 20.0), egui::Sense::click());
                gradient_swatch(ui.painter(), r, &current);
                let line = if resp.hovered() {
                    crate::theme::Tokens::ACCENT
                } else {
                    crate::theme::Tokens::border()
                };
                ui.painter().rect_stroke(
                    r,
                    2.0,
                    egui::Stroke::new(1.0, line),
                    egui::StrokeKind::Inside,
                );
                if resp.rich_tip(tr("grad.current_colors")).clicked() {
                    f.stops = current;
                    changed = true;
                }
                for (i, (key, _)) in crate::gradient_presets::PRESETS.iter().enumerate() {
                    let (r, resp) =
                        ui.allocate_exact_size(egui::vec2(34.0, 20.0), egui::Sense::click());
                    if let Some(stops) = crate::gradient_presets::stops(i) {
                        let angle = crate::gradient_presets::angle(i);
                        if angle.is_some() {
                            // Landscapes show ground at the bottom, sky on top.
                            gradient_swatch_up(ui.painter(), r, &stops);
                        } else {
                            gradient_swatch(ui.painter(), r, &stops);
                        }
                        let line = if resp.hovered() {
                            crate::theme::Tokens::ACCENT
                        } else {
                            crate::theme::Tokens::border()
                        };
                        ui.painter().rect_stroke(
                            r,
                            2.0,
                            egui::Stroke::new(1.0, line),
                            egui::StrokeKind::Inside,
                        );
                        if resp.rich_tip(tr(key)).clicked() {
                            f.stops = stops;
                            if let Some(a) = angle.filter(|_| f.kind == FountainKind::Linear) {
                                f.angle = a;
                            }
                            changed = true;
                        }
                    }
                }
            });
        });
    // Stops: position and colour, add after, remove (keep at least two).
    let mut remove = None;
    let mut insert = None;
    let n = f.stops.len();
    for i in 0..n {
        let stop = &mut f.stops[i];
        ui.horizontal(|ui| {
            if crate::ui::color_picker::color_button(ui, &mut stop.color) {
                changed = true;
            }
            let mut pct = stop.pos * 100.0;
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut pct)
                        .range(0.0..=100.0)
                        .suffix("%")
                        .speed(1.0),
                )
                .changed()
            {
                stop.pos = pct / 100.0;
                changed = true;
            }
            if ui
                .small_button("+")
                .rich_tip(tr("docker.add_stop"))
                .clicked()
            {
                insert = Some(i);
            }
            if n > 2
                && ui
                    .small_button("x")
                    .rich_tip(tr("docker.remove_stop"))
                    .clicked()
            {
                remove = Some(i);
            }
        });
    }
    // A `.ggr` gradient file replaces the stops (of this editor: the fill
    // and the outline each have one).
    let ggr_key = format!("fountain_ggr{:?}", ui.id());
    if ui
        .button(tr("docker.import_gradient"))
        .rich_tip(tr("docker.import_gradient_tip"))
        .clicked()
    {
        crate::files::Dialog::new()
            .add_filter("GIMP gradient", &["ggr"])
            .pick_into(&ggr_key);
    }
    if let Some(path) = crate::files::take_picked(&ggr_key) {
        match crate::files::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| tracedraw_io::ggr::read(&text))
        {
            Ok(g) => {
                f.stops = g.stops;
                changed = true;
            }
            Err(e) => log::warn!("{}: {e}", path.display()),
        }
    }
    if let Some(i) = insert {
        let next = f.stops.get(i + 1).map(|s| s.pos).unwrap_or(1.0);
        let pos = (f.stops[i].pos + next) / 2.0;
        // The stops' own colour there (not the repeats or the inversion).
        let color = tracedraw_core::style::sample_stops(&f.sorted_stops(), pos);
        f.stops.insert(i + 1, Stop::new(pos, color));
        changed = true;
    }
    if let Some(i) = remove {
        f.stops.remove(i);
        changed = true;
    }
    if matches!(
        f.kind,
        FountainKind::Linear | FountainKind::Conical | FountainKind::Square
    ) {
        changed |= ui
            .add(crate::ui::Rail(
                egui::Slider::new(&mut f.angle, -180.0..=180.0)
                    .text(tr("docker.angle_lc"))
                    .suffix("°"),
            ))
            .drag_stopped();
    }
    if !matches!(f.kind, FountainKind::Linear | FountainKind::Shaped) {
        ui.horizontal(|ui| {
            ui.label(tr("docker.centre_offset"));
            changed |= ui
                .add(
                    crate::ui::field::NumField::new(&mut f.offset.x)
                        .range(-1.0..=1.0)
                        .speed(0.01),
                )
                .changed();
            changed |= ui
                .add(
                    crate::ui::field::NumField::new(&mut f.offset.y)
                        .range(-1.0..=1.0)
                        .speed(0.01),
                )
                .changed();
        });
    }
    let mut pad = f.edge_pad * 100.0;
    if ui
        .add(crate::ui::Rail(
            egui::Slider::new(&mut pad, 0.0..=49.0)
                .text(tr("docker.edge_pad"))
                .suffix("%"),
        ))
        .drag_stopped()
    {
        f.edge_pad = pad / 100.0;
        changed = true;
    }
    changed |= fountain_spread_editor(ui, f);
    changed
}

/// The repeat mode and count, the bilinear mirror and the inversion.
fn fountain_spread_editor(ui: &mut Ui, f: &mut Fountain) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("docker.repeat"));
        for (m, key) in [
            (FountainRepeat::None, "docker.repeat_none"),
            (FountainRepeat::Repeat, "docker.repeat_sawtooth"),
            (FountainRepeat::Mirror, "docker.repeat_mirror"),
        ] {
            if ui.selectable_label(f.repeat == m, tr(key)).clicked() && f.repeat != m {
                f.repeat = m;
                if m != FountainRepeat::None && f.repeats < 2 {
                    f.repeats = 2;
                }
                changed = true;
            }
        }
    });
    if f.repeat != FountainRepeat::None {
        ui.horizontal(|ui| {
            ui.label(tr("docker.repeat_count"));
            changed |= ui
                .add(
                    crate::ui::field::NumField::new(&mut f.repeats)
                        .range(1..=Fountain::MAX_REPEATS),
                )
                .changed();
        });
    }
    ui.horizontal_wrapped(|ui| {
        changed |= ui
            .checkbox(&mut f.bilinear, tr("docker.bilinear"))
            .rich_tip(tr("docker.bilinear_tip"))
            .changed();
        changed |= ui
            .checkbox(&mut f.invert, tr("docker.invert_colours"))
            .rich_tip(tr("docker.invert_colours_tip"))
            .changed();
    });
    changed
}

pub(crate) fn pattern_editor(ui: &mut Ui, p: &mut Pattern) -> bool {
    let mut changed = false;
    match p {
        Pattern::TwoColor {
            tile,
            front,
            back,
            size_mm,
        } => {
            egui::ComboBox::from_label(tr("docker.tile"))
                .selected_text(tile.name())
                .show_ui(ui, |ui| {
                    for t in PatternTile::ALL {
                        if ui.selectable_label(*tile == t, t.name()).clicked() {
                            *tile = t;
                            changed = true;
                        }
                    }
                });
            changed |= color_row(ui, &tr("docker.front"), front);
            changed |= color_row(ui, &tr("docker.back"), back);
            changed |= ui
                .add(crate::ui::Rail(
                    egui::Slider::new(size_mm, 1.0..=100.0)
                        .text(tr("docker.tile_size_mm"))
                        .logarithmic(true),
                ))
                .drag_stopped();
        }
        Pattern::Bitmap {
            width_px,
            height_px,
            size_mm,
            ..
        } => {
            ui.label(trf(
                "docker.bitmap_tile_n",
                &[("w", &width_px.to_string()), ("h", &height_px.to_string())],
            ));
            changed |= ui
                .add(crate::ui::Rail(
                    egui::Slider::new(size_mm, 1.0..=200.0)
                        .text(tr("docker.tile_size_mm"))
                        .logarithmic(true),
                ))
                .drag_stopped();
        }
        Pattern::Vector { shapes, tile } => {
            ui.label(trf(
                "docker.vector_tile_n",
                &[("n", &shapes.len().to_string())],
            ));
            let (mut w, mut h) = (tile.width, tile.height);
            changed |= ui
                .add(crate::ui::Rail(
                    egui::Slider::new(&mut w, 1.0..=200.0)
                        .text(tr("docker.tile_width"))
                        .logarithmic(true),
                ))
                .drag_stopped();
            changed |= ui
                .add(crate::ui::Rail(
                    egui::Slider::new(&mut h, 1.0..=200.0)
                        .text(tr("docker.tile_height"))
                        .logarithmic(true),
                ))
                .drag_stopped();
            if (w - tile.width).abs() > 1e-9 || (h - tile.height).abs() > 1e-9 {
                // Scale the tile content with its size so the drawing keeps its shape.
                let sx = w / tile.width.max(1e-6);
                let sy = h / tile.height.max(1e-6);
                for s in shapes.iter_mut() {
                    s.transform =
                        tracedraw_core::geometry::Affine::scale_non_uniform(sx, sy) * s.transform;
                }
                tile.width = w;
                tile.height = h;
            }
        }
    }
    changed
}

pub(crate) fn texture_editor(ui: &mut Ui, t: &mut Texture) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (k, n) in [
            (TextureKind::Clouds, "texture.clouds"),
            (TextureKind::Marble, "texture.marble"),
            (TextureKind::Noise, "texture.noise"),
            (TextureKind::Wood, "texture.wood"),
        ] {
            if ui.selectable_label(t.kind == k, tr(n)).clicked() && t.kind != k {
                t.kind = k;
                changed = true;
            }
        }
    });
    changed |= color_row(ui, &tr("docker.colour_a"), &mut t.color_a);
    changed |= color_row(ui, &tr("docker.colour_b"), &mut t.color_b);
    changed |= ui
        .add(crate::ui::Rail(
            egui::Slider::new(&mut t.scale, 1.0..=200.0)
                .text(tr("docker.scale_mm"))
                .logarithmic(true),
        ))
        .drag_stopped();
    ui.horizontal(|ui| {
        ui.label(tr("docker.seed"));
        changed |= ui
            .add(crate::ui::field::NumField::new(&mut t.seed))
            .changed();
        if ui.small_button(tr("docker.regenerate")).clicked() {
            t.seed = t.seed.wrapping_add(1);
            changed = true;
        }
    });
    changed
}

pub(crate) fn arrow_combo(
    ui: &mut Ui,
    id: &str,
    label: &str,
    a: &mut Arrowhead,
    custom: &[Arrowhead],
    mixed: bool,
) -> bool {
    let mut changed = false;
    let shown = if mixed {
        format!("{label}:")
    } else {
        format!("{label}: {}", a.name())
    };
    egui::ComboBox::from_id_salt(id)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for k in Arrowhead::ALL {
                if ui.selectable_label(!mixed && *a == k, k.name()).clicked() {
                    *a = k;
                    changed = true;
                }
            }
            for (i, k) in custom.iter().enumerate() {
                let name = if k.name().is_empty() {
                    trf("docker.custom_arrowhead_n", &[("n", &(i + 1).to_string())])
                } else {
                    k.name().to_string()
                };
                if ui.selectable_label(!mixed && a == k, name).clicked() {
                    *a = k.clone();
                    changed = true;
                }
            }
        });
    changed
}

/// An outline's paint as a fill: its own fill, or its colour as a
/// uniform fill.
fn outline_paint_of(s: &Stroke) -> Fill {
    match s.fill.as_deref() {
        Some(f) if !matches!(f, Fill::None | Fill::Solid(_)) => f.clone(),
        _ => Fill::Solid(s.color),
    }
}

/// Give an outline the paint chosen in its fill editor: a uniform paint
/// becomes its colour, the other kinds its own fill. True when the
/// paint is none (the outline goes).
fn set_outline_paint(s: &mut Stroke, paint: Fill) -> bool {
    match paint {
        Fill::None => true,
        Fill::Solid(c) => {
            s.color = c;
            s.fill = None;
            false
        }
        other => {
            s.fill = Some(Box::new(other));
            false
        }
    }
}

fn outline_editor(
    ui: &mut Ui,
    stroke: &mut Option<Stroke>,
    lib: &crate::ui::line_style::OutlineLib,
) -> bool {
    outline_editor_mixed(ui, stroke, lib, &StrokeMix::default(), &mut Vec::new())
}

/// [`outline_editor`] for a selection whose outlines differ as `mix`
/// says: a value that differs shows empty (no colour, an empty width box,
/// no button highlighted, a check box neither on nor off). Every control
/// the user sets adds its field to `touched`, so a value chosen in a
/// mixed control applies to every object even when it equals the one
/// shown.
fn outline_editor_mixed(
    ui: &mut Ui,
    stroke: &mut Option<Stroke>,
    lib: &crate::ui::line_style::OutlineLib,
    mix: &StrokeMix,
    touched: &mut Vec<StrokeField>,
) -> bool {
    let custom = lib.arrows.as_slice();
    let mut changed = false;
    let mut has = stroke.is_some();
    if ui
        .add(egui::Checkbox::new(&mut has, tr("docker.outline")).indeterminate(mix.presence))
        .changed()
    {
        *stroke = if has {
            // Black in the drawing's primary colour mode.
            Some(Stroke {
                color: lib.black,
                ..Stroke::default()
            })
        } else {
            None
        };
        changed = true;
    }
    let mut remove = false;
    if let Some(s) = stroke.as_mut() {
        // The outline's paint offers every kind a fill does, with the
        // same editors: none takes the outline away, uniform is its
        // colour, the others its own fill.
        ui.label(tr("docker.outline_fill"));
        let mut paint = outline_paint_of(s);
        let paint_mix = if mix.has(StrokeField::Paint) {
            FillMix {
                kind: false,
                fields: vec![FillField::All],
            }
        } else {
            FillMix::default()
        };
        let mut paint_touched = Vec::new();
        let edited = ui
            .push_id("outline_paint", |ui| {
                fill_editor_mixed(ui, &mut paint, &paint_mix, &mut paint_touched)
            })
            .inner;
        if edited {
            changed = true;
            touched.push(StrokeField::Paint);
            remove = set_outline_paint(s, paint);
        }
    }
    if remove {
        *stroke = None;
    }
    if let Some(s) = stroke {
        let width_mixed = mix.has(StrokeField::Width);
        let mut hair = s.width <= Stroke::HAIRLINE + 1e-9;
        if ui
            .add(egui::Checkbox::new(&mut hair, tr("docker.hairline")).indeterminate(width_mixed))
            .changed()
        {
            s.width = if hair { Stroke::HAIRLINE } else { 0.5 };
            touched.push(StrokeField::Width);
            changed = true;
        }
        if !hair || width_mixed {
            // The slider applies when let go; the box takes any width
            // typed (Enter or leaving it applies).
            ui.horizontal_wrapped(|ui| {
                let r = ui.add(crate::ui::Rail(
                    egui::Slider::new(&mut s.width, 0.1..=25.0)
                        .logarithmic(true)
                        .show_value(false),
                ));
                let mut w = r.drag_stopped() || (r.changed() && !r.dragged());
                // The width in the outline unit; the model keeps mm.
                let u = lib.units;
                let mut v = u.from_mm(s.width);
                if ui
                    .add(
                        crate::ui::field::NumField::new(&mut v)
                            .range(u.from_mm(0.001)..=u.from_mm(1000.0))
                            .unit_mm(u.mm())
                            .max_decimals(4)
                            .width(56.0)
                            .mixed(width_mixed),
                    )
                    .changed()
                {
                    s.width = u.to_mm(v).clamp(0.001, 1000.0);
                    w = true;
                }
                if let Some(nu) =
                    crate::ui::propbar::outline_unit_combo(ui, "outline_units_docker", u)
                {
                    crate::ui::line_style::request_units(ui, nu);
                }
                if w {
                    touched.push(StrokeField::Width);
                    changed = true;
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("docker.caps"));
            let mixed = mix.has(StrokeField::Cap);
            for (c, n) in [
                (tracedraw_core::LineCap::Butt, "docker.cap_butt"),
                (tracedraw_core::LineCap::Round, "docker.cap_round"),
                (tracedraw_core::LineCap::Square, "docker.cap_square"),
            ] {
                if ui.selectable_label(!mixed && s.cap == c, tr(n)).clicked() {
                    s.cap = c;
                    touched.push(StrokeField::Cap);
                    changed = true;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("docker.corners"));
            let mixed = mix.has(StrokeField::Join);
            for (j, n) in [
                (tracedraw_core::LineJoin::Miter, "docker.join_miter"),
                (tracedraw_core::LineJoin::Round, "docker.join_round"),
                (tracedraw_core::LineJoin::Bevel, "docker.join_bevel"),
            ] {
                if ui.selectable_label(!mixed && s.join == j, tr(n)).clicked() {
                    s.join = j;
                    touched.push(StrokeField::Join);
                    changed = true;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (field, id, key) in [
                (StrokeField::StartArrow, "Start", "docker.arrow_start"),
                (StrokeField::EndArrow, "End", "docker.arrow_end"),
            ] {
                let a = if field == StrokeField::StartArrow {
                    &mut s.start_arrow
                } else {
                    &mut s.end_arrow
                };
                if arrow_combo(ui, id, &tr(key), a, custom, mix.has(field)) {
                    touched.push(field);
                    changed = true;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("docker.dash"));
            let mixed = mix.has(StrokeField::Dash);
            for (d, n) in [
                (vec![], "docker.dash_solid"),
                (vec![4.0, 2.0], "docker.dash_dashed"),
                (vec![1.0, 1.0], "docker.dash_dotted"),
                (vec![6.0, 2.0, 1.0, 2.0], "docker.dash_dash_dot"),
            ] {
                if ui.selectable_label(!mixed && s.dash == d, tr(n)).clicked() {
                    s.dash = d;
                    touched.push(StrokeField::Dash);
                    changed = true;
                }
            }
        });
        if !lib.dashes.is_empty() {
            // Custom dash patterns from Edit Line Style.
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("docker.dash_custom"));
                for d in &lib.dashes {
                    let r = ui
                        .push_id(
                            ("custom_dash", d.len(), d.iter().sum::<f64>() as i64),
                            |ui| {
                                let r = crate::ui::line_style::preview(ui, d, 48.0);
                                if s.dash == *d {
                                    ui.painter().rect_stroke(
                                        r.rect.expand(2.0),
                                        0.0,
                                        egui::Stroke::new(1.0, Tokens::ACCENT),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                                r
                            },
                        )
                        .inner;
                    if r.clicked() {
                        s.dash = d.clone();
                        touched.push(StrokeField::Dash);
                        changed = true;
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !s.dash.is_empty(),
                    egui::Checkbox::new(&mut s.align_dashes, tr("docker.align_dashes")),
                )
                .rich_tip(tr("docker.align_dashes_tip"))
                .changed()
            {
                touched.push(StrokeField::AlignDashes);
                changed = true;
            }
            if ui.button(tr("docker.edit_line_style")).clicked() {
                crate::ui::line_style::request(ui);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("docker.outline_position"));
            for (pos, n) in [
                (
                    tracedraw_core::OutlinePosition::Outside,
                    "docker.position_outside",
                ),
                (
                    tracedraw_core::OutlinePosition::Center,
                    "docker.position_center",
                ),
                (
                    tracedraw_core::OutlinePosition::Inside,
                    "docker.position_inside",
                ),
            ] {
                let shown = !mix.has(StrokeField::Position) && s.position == pos;
                if ui.selectable_label(shown, tr(n)).clicked() {
                    s.position = pos;
                    touched.push(StrokeField::Position);
                    changed = true;
                }
            }
        });
        for (field, key) in [
            (StrokeField::BehindFill, "docker.behind_fill"),
            (StrokeField::ScaleWithObject, "docker.scale_with_object"),
        ] {
            let v = if field == StrokeField::BehindFill {
                &mut s.behind_fill
            } else {
                &mut s.scale_with_object
            };
            if ui
                .add(egui::Checkbox::new(v, tr(key)).indeterminate(mix.has(field)))
                .changed()
            {
                touched.push(field);
                changed = true;
            }
        }
        // The settings few outlines need, under their own header so the
        // common ones are found at a glance (#487): open at first when
        // one of them is set away from its default or differs between
        // the objects, so nothing set is hidden.
        let advanced_set = s.miter_limit != Stroke::DEFAULT_MITER_LIMIT
            || s.stretch != 1.0
            || s.nib_angle != 0.0
            || [
                StrokeField::MiterLimit,
                StrokeField::Stretch,
                StrokeField::NibAngle,
            ]
            .iter()
            .any(|f| mix.has(*f));
        egui::CollapsingHeader::new(tr("docker.outline_advanced"))
            .id_salt("docker_outline_advanced")
            .default_open(advanced_set)
            .show(ui, |ui| {
                if s.join == tracedraw_core::LineJoin::Miter {
                    ui.horizontal(|ui| {
                        ui.label(tr("docker.miter_limit"));
                        if ui
                            .add(
                                crate::ui::field::NumField::new(&mut s.miter_limit)
                                    .range(Stroke::MITER_LIMIT_RANGE)
                                    .speed(0.5)
                                    .max_decimals(1)
                                    .suffix("°")
                                    .width(56.0)
                                    .mixed(mix.has(StrokeField::MiterLimit)),
                            )
                            .rich_tip(tr("docker.miter_limit_tip"))
                            .changed()
                        {
                            touched.push(StrokeField::MiterLimit);
                            changed = true;
                        }
                    });
                }
                for (field, key, range, suffix) in [
                    (StrokeField::Stretch, "docker.nib_stretch", 0.1..=1.0, ""),
                    (
                        StrokeField::NibAngle,
                        "docker.nib_angle_lc",
                        -90.0..=90.0,
                        "°",
                    ),
                ] {
                    let v = if field == StrokeField::Stretch {
                        &mut s.stretch
                    } else {
                        &mut s.nib_angle
                    };
                    // A slider cannot be empty: a mixed one says so in its text.
                    let text = if mix.has(field) {
                        format!("{} ({})", tr(key), tr("docker.mixed"))
                    } else {
                        tr(key)
                    };
                    if ui
                        .add(crate::ui::Rail(
                            egui::Slider::new(v, range).text(text).suffix(suffix),
                        ))
                        .drag_stopped()
                    {
                        touched.push(field);
                        changed = true;
                    }
                }
            });
    }
    changed
}

/// Stable, language-independent object type for scripts (`Shape.Type`).
pub fn kind_id(k: &ShapeKind) -> &'static str {
    match k {
        ShapeKind::Rect { .. } => "Rectangle",
        ShapeKind::Ellipse { .. } => "Ellipse",
        ShapeKind::Polygon {
            complex: Some(_), ..
        } => "ComplexStar",
        ShapeKind::Polygon { sharpness, .. } if *sharpness > 0.0 => "Star",
        ShapeKind::Polygon { .. } => "Polygon",
        ShapeKind::Path { .. } => "Curve",
        ShapeKind::Text { .. } => "Text",
        ShapeKind::Group { .. } => "Group",
        ShapeKind::Bitmap { .. } => "Bitmap",
        ShapeKind::ClipFrame { .. } => "ClipFrame",
        ShapeKind::Table(_) => "Table",
        ShapeKind::SymbolInstance { .. } => "Symbol",
    }
}

pub fn kind_name(k: &ShapeKind) -> String {
    match k {
        ShapeKind::Rect { .. } => tr("kind.rectangle"),
        ShapeKind::Ellipse { .. } => tr("kind.ellipse"),
        ShapeKind::Polygon {
            sharpness,
            points,
            complex,
            ..
        } => {
            let n = points.to_string();
            if complex.is_some() {
                trf("kind.complex_star_n", &[("n", &n)])
            } else if *sharpness > 0.0 {
                trf("kind.star_n", &[("n", &n)])
            } else {
                trf("kind.polygon_n", &[("n", &n)])
            }
        }
        ShapeKind::Path { path, .. } => {
            let n = tracedraw_core::nodes::nodes(path).len();
            trf("kind.curve_n", &[("n", &n.to_string())])
        }
        ShapeKind::Text { spans, frame, .. } => {
            let t: String = spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<String>()
                .chars()
                .take(20)
                .collect();
            let key = if frame.is_some() {
                "kind.paragraph_text"
            } else {
                "kind.artistic_text"
            };
            trf(key, &[("t", &t)])
        }
        ShapeKind::Group { children } => trf("kind.group_n", &[("n", &children.len().to_string())]),
        ShapeKind::Bitmap {
            width_px,
            height_px,
            ..
        } => trf(
            "kind.bitmap_n",
            &[("w", &width_px.to_string()), ("h", &height_px.to_string())],
        ),
        ShapeKind::ClipFrame { contents, .. } => {
            trf("kind.clip_frame_n", &[("n", &contents.len().to_string())])
        }
        ShapeKind::Table(t) => trf(
            "kind.table_n",
            &[("r", &t.rows().to_string()), ("c", &t.cols().to_string())],
        ),
        ShapeKind::SymbolInstance { index } => {
            trf("kind.symbol_n", &[("n", &(index + 1).to_string())])
        }
    }
}

fn undo_docker(app: &mut App, ui: &mut Ui) {
    let (undo, redo) = app.engine.history_labels();
    ui.label(
        egui::RichText::new(tr("docker.undo_hint"))
            .color(Tokens::text_dim())
            .size(11.0),
    );
    ui.add_space(4.0);
    let n = undo.len();
    let mut goto: Option<usize> = None;
    for (i, l) in undo.iter().enumerate() {
        let is_last = i + 1 == n;
        if ui.selectable_label(is_last, l.to_string()).clicked() && !is_last {
            goto = Some(n - 1 - i);
        }
    }
    if let Some(steps) = goto {
        for _ in 0..steps {
            app.undo();
        }
    }
    for l in redo.iter().rev() {
        let _ = ui.selectable_label(
            false,
            egui::RichText::new(l.to_string()).color(Tokens::text_dim()),
        );
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(n > 0, egui::Button::new(tr("docker.undo")))
            .clicked()
        {
            app.undo();
        }
        if ui
            .add_enabled(!redo.is_empty(), egui::Button::new(tr("docker.redo")))
            .clicked()
        {
            app.redo();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(ctx: &egui::Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    /// The pointer over a docker tab shows a close cross at its top; a
    /// click on the cross closes that docker, a click lower on the tab
    /// brings it to the front as before.
    #[test]
    fn a_docker_tab_closes_with_the_cross_at_its_top() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let tabs = app.open_dockers.clone();
        assert!(tabs.len() >= 2, "{tabs:?}");
        let run = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(27.0, 900.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| tab_strip(app, ui, DockerPlace::Right));
            out.textures_delta.clear();
        };
        run(&mut app, Vec::new());
        // The first tab starts under the title band; its cross sits in
        // the slot at its top.
        let first = tabs[0];
        let cross = egui::pos2(13.5, TITLE_H + 2.0 + (CLOSE_SLOT - 2.0) / 2.0);
        let click = |app: &mut App, at: egui::Pos2| {
            run(app, vec![egui::Event::PointerMoved(at)]);
            run(app, vec![egui::Event::PointerMoved(at)]);
            for pressed in [true, false] {
                run(
                    app,
                    vec![egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            run(app, Vec::new());
        };
        // Lower on the tab: brought to the front, still open.
        app.docker_tab = tabs[1];
        click(&mut app, egui::pos2(13.5, TITLE_H + CLOSE_SLOT + 30.0));
        assert_eq!(app.docker_tab, first);
        assert!(app.open_dockers.contains(&first));
        // On the cross: closed.
        click(&mut app, cross);
        assert!(!app.open_dockers.contains(&first), "{:?}", app.open_dockers);
    }

    /// More docker tabs than the column's height: the mouse wheel over
    /// the column slides it, so the last tab can be clicked to the
    /// front. It lay past the window's bottom edge, out of reach (#950).
    #[test]
    fn docker_tabs_past_the_column_bottom_come_into_reach() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for tab in DockerTab::MENU_GROUPS[0].iter() {
            if !app.open_dockers.contains(tab) {
                app.move_docker(*tab, DockerPlace::Right, None);
            }
        }
        let first = app.open_dockers[0];
        app.docker_tab = first;
        let last = *app.open_dockers.last().unwrap();
        let height = 400.0;
        let run = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(27.0, height),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| tab_strip(app, ui, DockerPlace::Right));
            out.textures_delta.clear();
        };
        let click = |app: &mut App, at: egui::Pos2| {
            run(app, vec![egui::Event::PointerMoved(at)]);
            for pressed in [true, false] {
                run(
                    app,
                    vec![egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            run(app, Vec::new());
        };
        run(&mut app, Vec::new());
        run(&mut app, Vec::new());
        // Wheel down over the column until it slides no further.
        let over = egui::pos2(13.5, height / 2.0);
        for _ in 0..200 {
            run(
                &mut app,
                vec![
                    egui::Event::PointerMoved(over),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -200.0),
                        modifiers: egui::Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    },
                ],
            );
        }
        for _ in 0..60 {
            run(&mut app, vec![egui::Event::PointerMoved(over)]);
        }
        // The last tab lies just above the add button and the down arrow
        // at the column's foot: a click there brings it to the front.
        click(
            &mut app,
            egui::pos2(
                13.5,
                height - crate::ui::overflow::BUTTON - 22.0 - 8.0 - 30.0,
            ),
        );
        assert_eq!(app.docker_tab, last, "{:?}", app.open_dockers);
    }

    /// A docker dragged by its tab (or its title) and dropped near the
    /// left edge goes to the column there, and the window draws with both
    /// columns and a floating docker.
    #[test]
    fn a_docker_dragged_to_a_side_docks_there_and_the_window_draws_them_all() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        let run = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
        };
        run(&mut app, Vec::new());
        // A drag under way (as a tab starts one), the pointer held down
        // at the left edge: the band shows; the release docks it there.
        app.docker_drag = Some(DockerDrag {
            tab: DockerTab::Hints,
            grab: egui::vec2(10.0, 10.0),
            last: None,
        });
        let at = egui::pos2(20.0, 450.0);
        run(
            &mut app,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(app.docker_drag.is_some());
        assert_eq!(app.docker_place(DockerTab::Hints), Some(DockerPlace::Right));
        run(
            &mut app,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.docker_drag.is_none());
        assert_eq!(app.docker_place(DockerTab::Hints), Some(DockerPlace::Left));
        assert!(app.show_left_dockers && app.left_tab == DockerTab::Hints);
        // The middle of the window: floating, with its top left corner
        // where the grab point puts it.
        app.docker_drag = Some(DockerDrag {
            tab: DockerTab::Objects,
            grab: egui::vec2(10.0, 10.0),
            last: None,
        });
        let at = egui::pos2(600.0, 300.0);
        run(
            &mut app,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        run(
            &mut app,
            vec![egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(
            app.docker_place(DockerTab::Objects),
            Some(DockerPlace::Floating)
        );
        let fd = app.floating_dockers[0];
        assert_eq!(fd.pos, egui::pos2(590.0, 290.0));
        // Both columns and the floating window draw, and the floating
        // window keeps near where it was put.
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let fd = app.floating_dockers[0];
        assert!(
            (fd.pos.x - 590.0).abs() < 2.0 && (fd.pos.y - 290.0).abs() < 2.0,
            "{fd:?}"
        );
        assert!(fd.size.x > 200.0 && fd.size.y > 200.0, "{fd:?}");
        assert_eq!(app.open_dockers, [DockerTab::Properties]);
        // Dropped at the right edge: back in the default column, in front.
        assert_eq!(
            drop_target(ctx.content_rect(), egui::pos2(1390.0, 400.0)),
            DockerPlace::Right
        );
        app.move_docker(DockerTab::Objects, DockerPlace::Right, None);
        assert!(app.floating_dockers.is_empty());
        assert_eq!(
            app.open_dockers,
            [DockerTab::Properties, DockerTab::Objects]
        );
        assert_eq!(app.docker_tab, DockerTab::Objects);
        run(&mut app, Vec::new());
    }

    /// Dragging in the outline colour picker, as with the fill: every
    /// colour passed over shows on the outline, but the drag is one undo
    /// step and only the colour it ends on joins the document palette.
    #[test]
    fn an_outline_dragged_in_the_picker_is_one_step_and_one_palette_colour() {
        use tracedraw_core::{Color, Stroke};
        let mut app = App::headless();
        app.settings.palette.auto_update_document = true;
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        let stroke = |app: &App| app.doc().find_shape(id).expect("shape").stroke.clone();
        let before = stroke(&app);
        let old = Some(before.clone().unwrap_or_default());
        let steps = |app: &App| app.engine.history_labels().0.len();
        let n = steps(&app);
        let palette = app.doc().palette.clone();
        let colours = [
            Color::rgb8(250, 10, 10),
            Color::rgb8(10, 250, 10),
            Color::rgb8(10, 10, 250),
        ];
        for c in colours {
            let new = Some(Stroke {
                color: c,
                ..old.clone().unwrap_or_default()
            });
            app.patch_selection_stroke_live(&old, &new, &[]);
            assert_eq!(stroke(&app).map(|s| s.color), Some(c));
            assert_eq!(steps(&app), n + 1, "one step for the drag");
            assert_eq!(app.doc().palette, palette, "nothing added while dragging");
        }
        // A frame with the button up ends the drag: the last colour only.
        let ctx = crate::theme::ui_context();
        draw(&ctx, &mut app);
        assert!(app.live_stroke.is_none());
        let added: Vec<Color> = app
            .doc()
            .palette
            .iter()
            .filter(|c| !palette.contains(c))
            .copied()
            .collect();
        assert_eq!(added, vec![colours[2]]);
        assert_eq!(steps(&app), n + 1);
        // One undo takes the whole drag back, palette colour included.
        app.undo();
        assert_eq!(stroke(&app), before);
        assert_eq!(app.doc().palette, palette);
    }

    /// Dragging in the fill colour picker: every colour passed over shows
    /// on the object, but the drag is one undo step and only the colour
    /// it ends on joins the document palette, when the button is let go.
    #[test]
    fn a_fill_dragged_in_the_picker_is_one_step_and_one_palette_colour() {
        use tracedraw_core::Color;
        let mut app = App::headless();
        app.settings.palette.auto_update_document = true;
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        let before_fill = app.doc().find_shape(id).expect("shape").fill.clone();
        let steps = |app: &App| app.engine.history_labels().0.len();
        let n = steps(&app);
        let palette = app.doc().palette.clone();
        let colours = [
            Color::rgb8(250, 10, 10),
            Color::rgb8(10, 250, 10),
            Color::rgb8(10, 10, 250),
        ];
        for c in colours {
            app.apply_fill_live(Fill::Solid(c));
            assert_eq!(
                app.doc().find_shape(id).expect("shape").fill,
                Fill::Solid(c)
            );
            assert_eq!(steps(&app), n + 1, "one step for the drag");
            assert_eq!(app.doc().palette, palette, "nothing added while dragging");
        }
        // A frame with the button up ends the drag: the last colour only.
        let ctx = crate::theme::ui_context();
        draw(&ctx, &mut app);
        assert!(app.live_fill.is_none());
        let added: Vec<Color> = app
            .doc()
            .palette
            .iter()
            .filter(|c| !palette.contains(c))
            .copied()
            .collect();
        assert_eq!(added, vec![colours[2]]);
        assert_eq!(steps(&app), n + 1);
        // One undo takes the whole drag back, palette colour included.
        app.undo();
        assert_eq!(app.doc().find_shape(id).expect("shape").fill, before_fill);
        assert_eq!(app.doc().palette, palette);
        // A new drag after other work starts its own step.
        app.apply_fill_live(Fill::Solid(colours[0]));
        app.end_live_fill();
        app.apply_fill_live(Fill::Solid(colours[1]));
        app.end_live_fill();
        assert_eq!(steps(&app), n + 2);
    }

    /// Every text the frame painted.
    fn painted_texts(ctx: &egui::Context, app: &mut App) -> Vec<String> {
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
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
        let mut texts = Vec::new();
        for c in &out.shapes {
            walk(&c.shape, &mut texts);
        }
        texts
    }

    /// Two objects with different outline colours and widths: the panel
    /// shows the colour and the width as mixed (no colour, an empty width
    /// box); one object alone shows its values.
    #[test]
    fn mixed_values_show_empty() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        // Widths in millimetres, as the numbers below are.
        app.settings.outline_units = crate::app::Units::Millimeters.id().to_string();
        let mut ids = Vec::new();
        for (x, c, w) in [
            (10.0, tracedraw_core::Color::rgb8(0, 0, 255), 1.234),
            (70.0, tracedraw_core::Color::rgb8(255, 255, 0), 3.456),
        ] {
            let id = app
                .new_shape(ShapeKind::Rect {
                    rect: tracedraw_core::geometry::Rect::new(x, 10.0, x + 40.0, 40.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("rect");
            app.run(Command::SetStroke {
                shapes: vec![id],
                stroke: Some(Stroke::new(c, w)),
            });
            ids.push(id);
        }
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
        let mixed = tr("docker.mixed");
        app.select(ids[..1].to_vec());
        draw(&ctx, &mut app);
        let texts = painted_texts(&ctx, &mut app);
        assert!(!texts.iter().any(|t| *t == mixed));
        assert!(texts.iter().any(|t| t == "1.234"), "the width shows");
        app.select(ids.clone());
        draw(&ctx, &mut app);
        let texts = painted_texts(&ctx, &mut app);
        assert!(texts.iter().any(|t| *t == mixed), "the colour is mixed");
        assert!(
            !texts
                .iter()
                .any(|t| t.contains("1.234") || t.contains("3.456")),
            "no width shows: {:?}",
            texts
                .iter()
                .filter(|t| t.contains("1.234") || t.contains("3.456"))
                .collect::<Vec<_>>()
        );
        let mix = app.selection_stroke_mix();
        assert!(mix.has(StrokeField::Width) && mix.has(StrokeField::Paint));
    }

    /// The outline's paint editor works on the outline: uniform is its
    /// colour, the other kinds its own fill, none takes it away.
    #[test]
    fn outline_paint_offers_every_fill_kind() {
        let red = tracedraw_core::Color::rgb8(255, 0, 0);
        let mut s = tracedraw_core::Stroke::new(red, 1.0);
        assert_eq!(outline_paint_of(&s), Fill::Solid(red));
        let texture = Fill::Texture(Texture {
            kind: TextureKind::Wood,
            color_a: red,
            color_b: tracedraw_core::Color::WHITE,
            scale: 5.0,
            seed: 1,
        });
        for paint in [
            Fill::linear(red, tracedraw_core::Color::WHITE, 0.0),
            Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Dots,
                front: red,
                back: tracedraw_core::Color::WHITE,
                size_mm: 4.0,
            }),
            texture,
            Fill::Mesh(tracedraw_core::Mesh::new(
                tracedraw_core::geometry::Rect::new(0.0, 0.0, 10.0, 10.0),
                1,
                1,
                red,
            )),
        ] {
            assert!(!set_outline_paint(&mut s, paint.clone()));
            assert_eq!(s.fill.as_deref(), Some(&paint));
            assert_eq!(outline_paint_of(&s), paint);
        }
        let blue = tracedraw_core::Color::rgb8(0, 0, 255);
        assert!(!set_outline_paint(&mut s, Fill::Solid(blue)));
        assert_eq!((s.color, s.fill.is_none()), (blue, true));
        assert!(set_outline_paint(&mut s, Fill::None));
        // A plain colour kept as the outline's fill reads as uniform.
        s.fill = Some(Box::new(Fill::Solid(red)));
        assert_eq!(outline_paint_of(&s), Fill::Solid(blue));
        assert_eq!(fountain_kind_key(FountainKind::Shaped), "fill.shaped");
        assert_ne!(tr("fill.shaped"), "fill.shaped");
    }

    /// The Properties docker has the same titled sections with and without
    /// a selection, so each row of paint kinds is named; Add Outline and
    /// Add Fill show only once there is an outline or a fill to add to;
    /// the corner names are never cut short (#487).
    #[test]
    fn properties_sections_are_titled_and_add_buttons_need_a_paint() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.settings.autocorrect.enabled = false;
        app.settings.properties_closed.clear();
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
        let count = |texts: &[String], key: &str| texts.iter().filter(|t| **t == tr(key)).count();
        // Nothing selected: an Outline title over the outline's kinds and
        // a Fill title over the fill's.
        draw(&ctx, &mut app);
        let texts = painted_texts(&ctx, &mut app);
        assert_eq!(count(&texts, "docker.fill"), 1, "{texts:?}");
        assert!(
            count(&texts, "docker.outline") >= 2,
            "title and box: {texts:?}"
        );
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 2.0,
                corners: None,
            })
            .expect("rect");
        app.run(Command::SetStroke {
            shapes: vec![id],
            stroke: None,
        });
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::None,
        });
        app.select(vec![id]);
        draw(&ctx, &mut app);
        let texts = painted_texts(&ctx, &mut app);
        assert_eq!(count(&texts, "docker.add_outline"), 0, "{texts:?}");
        assert_eq!(count(&texts, "docker.add_fill"), 0, "{texts:?}");
        // Every corner named in full.
        for key in [
            "toolbar.corner_top_left",
            "toolbar.corner_top_right",
            "toolbar.corner_bottom_left",
            "toolbar.corner_bottom_right",
        ] {
            let name = tr(key);
            assert!(
                texts.iter().any(|t| t.replace('\n', " ") == name),
                "{name} cut short: {texts:?}"
            );
        }
        assert!(!texts.iter().any(|t| t.ends_with('\u{2026}')), "{texts:?}");
        // With an outline and a fill, a further one can be added.
        app.run(Command::SetStroke {
            shapes: vec![id],
            stroke: Some(Stroke::new(tracedraw_core::Color::BLACK, 1.0)),
        });
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(tracedraw_core::Color::BLACK),
        });
        draw(&ctx, &mut app);
        let texts = painted_texts(&ctx, &mut app);
        assert_eq!(count(&texts, "docker.add_outline"), 1, "{texts:?}");
        assert_eq!(count(&texts, "docker.add_fill"), 1, "{texts:?}");
    }

    /// Every outline option stays in the Properties docker, with and
    /// without a selection: the common ones in view, the mitre limit and
    /// the nib under an Advanced header that a click opens (open at once
    /// when one of them is set), and no label cut short (#487).
    #[test]
    fn every_outline_option_is_in_the_properties_docker() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.settings.autocorrect.enabled = false;
        app.settings.properties_closed.clear();
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
        // The texts painted, with where they are.
        let frame = |app: &mut App, events: Vec<egui::Event>| -> Vec<(String, egui::Rect)> {
            fn walk(s: &egui::Shape, out: &mut Vec<(String, egui::Rect)>) {
                match s {
                    egui::Shape::Text(t) => out.push((
                        t.galley.text().to_string(),
                        t.galley.rect.translate(t.pos.to_vec2()),
                    )),
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                    _ => {}
                }
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 1900.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
            let mut texts = Vec::new();
            out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
            texts
        };
        let common = [
            "docker.hairline",
            "docker.caps",
            "docker.cap_butt",
            "docker.cap_round",
            "docker.cap_square",
            "docker.corners",
            "docker.join_miter",
            "docker.join_round",
            "docker.join_bevel",
            "docker.dash",
            "docker.dash_solid",
            "docker.dash_dashed",
            "docker.dash_dotted",
            "docker.dash_dash_dot",
            "docker.align_dashes",
            "docker.edit_line_style",
            "docker.outline_position",
            "docker.position_outside",
            "docker.position_center",
            "docker.position_inside",
            "docker.behind_fill",
            "docker.scale_with_object",
            "docker.outline_advanced",
        ];
        let advanced = [
            "docker.miter_limit",
            "docker.nib_stretch",
            "docker.nib_angle_lc",
        ];
        let has = |texts: &[(String, egui::Rect)], key: &str| {
            let want = tr(key);
            texts
                .iter()
                .any(|(t, _)| t.replace('\n', " ") == want || t.starts_with(&want))
        };
        let check = |app: &mut App, what: &str| {
            for _ in 0..3 {
                frame(app, Vec::new());
            }
            let texts = frame(app, Vec::new());
            for key in common
                .iter()
                .chain(["docker.arrow_start", "docker.arrow_end"].iter())
            {
                assert!(has(&texts, key), "{what}: {key} missing in {texts:?}");
            }
            for key in advanced {
                assert!(!has(&texts, key), "{what}: {key} not under Advanced");
            }
            assert!(
                !texts.iter().any(|(t, _)| t.ends_with('\u{2026}')),
                "{what}: a label cut short in {texts:?}"
            );
            // A click on the header shows the rest.
            let at = texts
                .iter()
                .find(|(t, _)| *t == tr("docker.outline_advanced"))
                .map(|(_, r)| r.center())
                .expect("the Advanced header");
            let press = |pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            frame(app, vec![egui::Event::PointerMoved(at)]);
            frame(app, vec![press(true)]);
            frame(app, vec![press(false)]);
            for _ in 0..3 {
                frame(app, Vec::new());
            }
            let texts = frame(app, Vec::new());
            for key in advanced {
                assert!(
                    has(&texts, key),
                    "{what}: {key} missing when open: {texts:?}"
                );
            }
            assert!(!texts.iter().any(|(t, _)| t.ends_with('\u{2026}')));
            // Closed again for the next check.
            frame(app, vec![egui::Event::PointerMoved(at)]);
            frame(app, vec![press(true)]);
            frame(app, vec![press(false)]);
        };
        // Nothing selected: the outline of new objects (mitred corners, so
        // the mitre limit shows).
        let mut pen = app.default_stroke().unwrap_or_default();
        pen.join = tracedraw_core::LineJoin::Miter;
        app.change_defaults(
            &crate::defaults::DefaultChange::Outline(Some(pen.clone())),
            [true, true, true],
        );
        check(&mut app, "nothing selected");
        // An object with that outline.
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.run(Command::SetStroke {
            shapes: vec![id],
            stroke: Some(pen.clone()),
        });
        app.select(vec![id]);
        check(&mut app, "a rectangle");
        // A nib set away from its default: the header comes open.
        let ctx2 = crate::theme::ui_context();
        let mut app2 = App::headless();
        app2.show_welcome = false;
        app2.settings.autocorrect.enabled = false;
        app2.settings.properties_closed.clear();
        app2.show_dockers = true;
        app2.docker_tab = DockerTab::Properties;
        let id = app2
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app2.run(Command::SetStroke {
            shapes: vec![id],
            stroke: Some(Stroke {
                nib_angle: 30.0,
                ..pen
            }),
        });
        app2.select(vec![id]);
        for _ in 0..3 {
            draw(&ctx2, &mut app2);
        }
        let texts = painted_texts(&ctx2, &mut app2);
        assert!(
            texts.iter().any(|t| *t == tr("docker.nib_angle_lc")),
            "{texts:?}"
        );
    }

    /// The Properties docker keeps its default width with every section
    /// open, whatever the object, its fill, outline and transparency, in
    /// the languages with the longest words: no section needs more room
    /// than the docker has, and selecting an object never widens it.
    #[test]
    fn properties_sections_fit_the_default_docker_width() {
        use tracedraw_core::geometry::{Point, Rect};
        use tracedraw_core::Color;
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.settings.autocorrect.enabled = false;
        app.settings.properties_closed.clear();
        let rect = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 2.0,
                corners: None,
            })
            .expect("rect");
        let ellipse = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(70.0, 10.0, 120.0, 40.0),
                arc: None,
            })
            .expect("ellipse");
        let polygon = app
            .new_shape(ShapeKind::Polygon {
                rect: Rect::new(130.0, 10.0, 170.0, 40.0),
                points: 5,
                sharpness: 50.0,
                complex: None,
                twist: 0.0,
            })
            .expect("polygon");
        app.start_text(Point::new(20.0, 80.0), None);
        app.text_insert("Docker test");
        app.finish_text();
        let text = app.selection[0];
        app.start_text(
            Point::new(20.0, 100.0),
            Some(tracedraw_core::geometry::Size::new(60.0, 30.0)),
        );
        app.text_insert("Paragraph");
        app.finish_text();
        let paragraph = app.selection[0];
        app.select(vec![ellipse]);
        app.duplicate();
        app.convert_to_bitmap(50.0, true);
        let bitmap = app.selection[0];
        app.select(Vec::new());
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
        let overflow = |ctx: &egui::Context, app: &mut App| {
            for _ in 0..3 {
                draw(ctx, app);
            }
            let need: f32 = ctx
                .data(|d| d.get_temp(egui::Id::new("docker_overflow")))
                .expect("the docker is drawn");
            let width = egui::containers::panel::PanelState::load(ctx, egui::Id::new("dockers"))
                .map(|s| s.size().x)
                .expect("the docker is shown");
            (need, width)
        };
        let red = Color::rgb8(255, 0, 0);
        let fills = [
            Fill::Solid(red),
            Fill::linear(red, Color::WHITE, 0.0),
            Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Dots,
                front: red,
                back: Color::WHITE,
                size_mm: 5.0,
            }),
            Fill::Texture(tracedraw_core::style::Texture {
                kind: tracedraw_core::style::TextureKind::Wood,
                color_a: red,
                color_b: Color::WHITE,
                scale: 5.0,
                seed: 1,
            }),
        ];
        for lang in ["en", "de", "ru", "pt-BR"] {
            crate::i18n::with_thread_language(lang, || {
                app.select(Vec::new());
                let (need, empty) = overflow(&ctx, &mut app);
                assert!(need <= 0.5, "{lang}, nothing selected: {need} too wide");
                assert!(empty <= 341.0, "{lang}: {empty}");
                for (name, id) in [
                    ("rectangle", rect),
                    ("ellipse", ellipse),
                    ("polygon", polygon),
                    ("text", text),
                    ("paragraph", paragraph),
                    ("bitmap", bitmap),
                ] {
                    app.select(vec![id]);
                    let (need, w) = overflow(&ctx, &mut app);
                    assert!(need <= 0.5, "{lang}, {name}: {need} too wide");
                    assert!((w - empty).abs() < 1.0, "{lang}, {name}: {w} after {empty}");
                }
                app.select(vec![rect]);
                for (k, fill) in fills.iter().enumerate() {
                    app.apply_fill(fill.clone());
                    for kind in 0..4u8 {
                        app.transparency_default.set_kind(kind);
                        let t = app.transparency_default.clone();
                        app.patch_transparency(&t, &[crate::mixed::TransparencyField::Kind]);
                        let (need, w) = overflow(&ctx, &mut app);
                        assert!(need <= 0.5, "{lang}, fill {k}, transparency {kind}: {need}");
                        assert!((w - empty).abs() < 1.0, "{lang}, fill {k}: {w}");
                    }
                }
            });
        }
    }

    /// The Properties sections come open; a button or a header closes and
    /// opens one (kept in the preferences), and a jump to the fill or the
    /// outline opens it again.
    #[test]
    fn properties_sections_open_close_and_jump() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
        assert!(app.settings.properties_closed.is_empty());
        draw(&ctx, &mut app);
        toggle_section(&mut app, "fill");
        toggle_section(&mut app, "transparency");
        assert_eq!(app.settings.properties_closed, ["fill", "transparency"]);
        draw(&ctx, &mut app);
        toggle_section(&mut app, "transparency");
        assert_eq!(app.settings.properties_closed, ["fill"]);
        // F11 (the fill) opens the fill section again.
        app.properties_open = Some(0);
        draw(&ctx, &mut app);
        assert!(app.settings.properties_closed.is_empty());
        assert!(app.properties_open.is_none());
        // Further fills and outlines show with their editors.
        set_extra_paint(
            &mut app,
            vec![Fill::Solid(tracedraw_core::Color::BLACK)],
            vec![tracedraw_core::Stroke::hairline(
                tracedraw_core::Color::WHITE,
            )],
        );
        let s = app.doc().find_shape(id).expect("shape").clone();
        assert_eq!((s.extra_fills.len(), s.extra_strokes.len()), (1, 1));
        draw(&ctx, &mut app);
        app.undo();
        assert!(app
            .doc()
            .find_shape(id)
            .expect("shape")
            .extra_fills
            .is_empty());
        // Every outline paint and the gradient options draw in the docker.
        let mut fountain = Fountain::two(
            FountainKind::Shaped,
            tracedraw_core::Color::BLACK,
            tracedraw_core::Color::WHITE,
            0.0,
        );
        fountain.repeat = FountainRepeat::Mirror;
        fountain.repeats = 3;
        fountain.bilinear = true;
        fountain.invert = true;
        app.apply_fill(Fill::Fountain(fountain.clone()));
        for paint in [
            Fill::Fountain(fountain),
            Fill::Texture(Texture {
                kind: TextureKind::Marble,
                color_a: tracedraw_core::Color::BLACK,
                color_b: tracedraw_core::Color::WHITE,
                scale: 5.0,
                seed: 1,
            }),
            Fill::Mesh(tracedraw_core::Mesh::new(
                tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
                2,
                2,
                tracedraw_core::Color::BLACK,
            )),
        ] {
            let mut stroke = tracedraw_core::Stroke::new(tracedraw_core::Color::BLACK, 2.0);
            stroke.fill = Some(Box::new(paint));
            app.run(Command::SetStroke {
                shapes: vec![id],
                stroke: Some(stroke),
            });
            draw(&ctx, &mut app);
            let s = app.doc().find_shape(id).expect("shape");
            assert!(s.stroke.as_ref().and_then(|st| st.fill.as_ref()).is_some());
        }
    }
}
