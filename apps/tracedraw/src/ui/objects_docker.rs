//! The Objects docker: the current page with its layers, topmost first,
//! then the Master Page with the master layers, the guides and the
//! document grid. A layer row has a collapse arrow, an eye (show or
//! hide), a printer (print and export or not), a padlock (lock or
//! unlock), the layer icon and the name, in bold on a blue row for the
//! active layer, the one new objects go to; clicking a layer makes it
//! active. Under each open layer come its objects, topmost first, each
//! with a thumbnail (the object itself drawn small, see
//! `object_thumbs.rs`) and its name (or what it is when it has no name).
//! The thumbnail size is set by the control in the bottom bar, the wheel
//! over it, or Ctrl+wheel over the list. Groups open to show their objects. Clicking an object
//! selects it (Shift adds it) and makes its layer active; double-clicking
//! a name renames the layer or object; a layer's context menu moves,
//! renames or deletes it. The bar at the bottom adds a layer or a master
//! layer and deletes the selected objects or, with none, the active layer.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dockers::kind_name;
use crate::ui::tips::RichTip;
use egui::{Color32, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use tracedraw_core::{Command, Layer, LayerId, MasterScope, Page, Shape, ShapeId, ShapeKind};

/// Height of a row.
const ROW_H: f32 = 22.0;
/// Indent per nesting level.
const INDENT: f32 = 16.0;
/// Height of the bar with the New Layer, New Master Layer and Delete
/// buttons.
const BAR_H: f32 = 30.0;
/// The top sheet of the layer icon: blue for a page's layers, amber for
/// master layers.
const LAYER_TOP: Color32 = Color32::from_rgb(0x4A, 0x90, 0xD9);
const MASTER_TOP: Color32 = Color32::from_rgb(0xE0, 0x9A, 0x2E);
const GUIDE_BLUE: Color32 = Color32::from_rgb(0x1E, 0x6F, 0xD9);
const PLUS_GREEN: Color32 = Color32::from_rgb(0x1E, 0x9E, 0x3C);

/// What the docker asks for after drawing.
#[derive(Default)]
struct Actions {
    visible: Option<(LayerId, bool)>,
    printable: Option<(LayerId, bool)>,
    locked: Option<(LayerId, bool)>,
    scope: Option<(LayerId, MasterScope)>,
    in_front: Option<(LayerId, bool)>,
    pick: Option<LayerId>,
    select: Option<ShapeId>,
    /// An object's eye or padlock was clicked (#832).
    object_visible: Option<(ShapeId, bool)>,
    object_locked: Option<(ShapeId, bool)>,
    rename_layer: Option<(LayerId, String)>,
    rename_object: Option<(ShapeId, String)>,
    rename_page: Option<String>,
    delete_layer: Option<LayerId>,
    reorder: Option<(LayerId, usize)>,
    collapse: Option<LayerId>,
    expand_group: Option<ShapeId>,
    toggle_page: bool,
    toggle_master: bool,
    toggle_guides: bool,
    toggle_grid: bool,
    lock_guides: Option<bool>,
    /// Not an action: the object whose row is scrolled into view.
    scroll_to: Option<ShapeId>,
    /// Not an action: the thumbnail cache, taken from the app while the
    /// tree is drawn, the thumbnail size (points) and the drawing's
    /// (instance, revision).
    thumbs: crate::object_thumbs::ObjectThumbs,
    thumb_size: f32,
    drawing: (u64, u64),
}

/// Where a layer sits: its index in its list (0 at the bottom), how many
/// the list holds, whether it is a master layer and whether it shows on
/// the current page.
#[derive(Clone, Copy)]
struct Place {
    index: usize,
    count: usize,
    master: bool,
    shown: bool,
}

/// How a row is painted.
#[derive(Clone, Copy, PartialEq)]
enum Look {
    Object { selected: bool },
    Layer { active: bool },
    Node,
}

/// The label an object shows: its name, else what it is.
pub fn object_label(s: &Shape) -> String {
    match s.name.as_deref().map(str::trim) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => kind_name(&s.kind),
    }
}

/// An eye, open when `on`, else closed (a lid line), centred in `r`.
fn paint_eye(ui: &Ui, r: Rect, on: bool) {
    let p = ui.painter();
    let c = r.center();
    let color = if on {
        Tokens::icon()
    } else {
        Tokens::icon_off()
    };
    let w = 6.0;
    let pts_top: Vec<Pos2> = (0..=12)
        .map(|i| {
            let t = i as f32 / 12.0;
            let x = -w + 2.0 * w * t;
            Pos2::new(c.x + x, c.y - 3.6 * (1.0 - (x / w).powi(2)))
        })
        .collect();
    let pts_bottom: Vec<Pos2> = pts_top
        .iter()
        .map(|q| Pos2::new(q.x, c.y + (c.y - q.y)))
        .collect();
    let stroke = Stroke::new(1.1, color);
    p.add(egui::Shape::line(pts_top, stroke));
    if on {
        p.add(egui::Shape::line(pts_bottom, stroke));
        p.circle_filled(c, 1.9, color);
    } else {
        // Closed: only the lower lid with lashes.
        p.add(egui::Shape::line(pts_bottom.clone(), stroke));
        for dx in [-3.5f32, 0.0, 3.5] {
            let base = Pos2::new(c.x + dx, c.y + 3.0 * (1.0 - (dx / w).powi(2)));
            p.line_segment([base, base + Vec2::new(dx * 0.15, 2.0)], stroke);
        }
    }
}

/// A printer, dark when `on`, else pale, centred in `r`.
fn paint_printer(ui: &Ui, r: Rect, on: bool) {
    let p = ui.painter();
    let c = r.center();
    let color = if on {
        Tokens::icon()
    } else {
        Tokens::icon_off()
    };
    let stroke = Stroke::new(1.0, color);
    // The sheet going in at the top.
    p.rect_stroke(
        Rect::from_min_max(c + Vec2::new(-3.5, -6.5), c + Vec2::new(3.5, -2.0)),
        0.0,
        stroke,
        egui::epaint::StrokeKind::Inside,
    );
    // The body.
    p.rect_filled(
        Rect::from_min_max(c + Vec2::new(-6.5, -2.5), c + Vec2::new(6.5, 3.5)),
        1.0,
        color,
    );
    // The printed sheet coming out.
    let out = Rect::from_min_max(c + Vec2::new(-3.5, 1.0), c + Vec2::new(3.5, 6.5));
    p.rect_filled(out, 0.0, Color32::WHITE);
    p.rect_stroke(out, 0.0, stroke, egui::epaint::StrokeKind::Inside);
}

/// A padlock, shut when `locked`, else open, centred in `r`: dark when
/// shut, pale when open.
fn paint_lock(ui: &Ui, r: Rect, locked: bool) {
    let color = if locked {
        Tokens::icon()
    } else {
        Tokens::icon_off()
    };
    paint_padlock(ui.painter(), r, locked, color);
}

/// A padlock in `color`, shut when `locked`, else open.
fn paint_padlock(p: &Painter, r: Rect, locked: bool, color: Color32) {
    let c = r.center();
    let body = Rect::from_center_size(c + Vec2::new(0.0, 2.0), Vec2::new(9.0, 7.0));
    p.rect_filled(body, 1.0, color);
    let stroke = Stroke::new(1.3, color);
    let left = body.left() + 2.2;
    let right = body.right() - 2.2;
    let top = body.top() - 4.0;
    let shackle_right = if locked { right } else { right + 3.0 };
    let arc: Vec<Pos2> = (0..=10)
        .map(|i| {
            let a = std::f32::consts::PI * i as f32 / 10.0;
            let mid = (left + shackle_right) / 2.0;
            let rx = (shackle_right - left) / 2.0;
            Pos2::new(mid - rx * a.cos(), top - 0.5 - 2.0 * a.sin() + 2.0)
        })
        .collect();
    p.add(egui::Shape::line(arc, stroke));
    p.line_segment(
        [Pos2::new(left, top + 1.5), Pos2::new(left, body.top())],
        stroke,
    );
    let right_end = if locked { body.top() } else { top + 3.0 };
    p.line_segment(
        [
            Pos2::new(shackle_right, top + 1.5),
            Pos2::new(shackle_right, right_end),
        ],
        stroke,
    );
}

/// Three stacked sheets: the layer icon, its top sheet amber for a master
/// layer.
fn paint_layer_icon(p: &Painter, r: Rect, master: bool) {
    let c = r.center();
    for (i, dy) in [3.0f32, 0.0, -3.0].iter().enumerate() {
        let pts = vec![
            Pos2::new(c.x - 6.0, c.y + dy),
            Pos2::new(c.x, c.y + dy - 2.8),
            Pos2::new(c.x + 6.0, c.y + dy),
            Pos2::new(c.x, c.y + dy + 2.8),
        ];
        let fill = match (i, master) {
            (2, false) => LAYER_TOP,
            (2, true) => MASTER_TOP,
            _ => Color32::from_rgb(0xC9, 0xDD, 0xF2),
        };
        p.add(egui::Shape::convex_polygon(
            pts,
            fill,
            Stroke::new(0.8, Color32::from_rgb(0x2F, 0x6D, 0xB5)),
        ));
    }
}

/// A sheet with a folded corner: the page and master page nodes.
fn paint_page_icon(p: &Painter, r: Rect, master: bool) {
    let c = r.center();
    let (x0, x1, y0, y1) = (c.x - 5.0, c.x + 5.0, c.y - 6.5, c.y + 6.5);
    let fold = 3.5;
    let outline = vec![
        Pos2::new(x0, y0),
        Pos2::new(x1 - fold, y0),
        Pos2::new(x1, y0 + fold),
        Pos2::new(x1, y1),
        Pos2::new(x0, y1),
    ];
    let fill = if master {
        Color32::from_rgb(0xFB, 0xE7, 0xC6)
    } else {
        crate::theme::Tokens::field()
    };
    p.add(egui::Shape::convex_polygon(
        outline,
        fill,
        Stroke::new(1.0, Tokens::icon()),
    ));
    p.add(egui::Shape::line(
        vec![
            Pos2::new(x1 - fold, y0),
            Pos2::new(x1 - fold, y0 + fold),
            Pos2::new(x1, y0 + fold),
        ],
        Stroke::new(0.8, Tokens::icon()),
    ));
}

/// Two crossing guidelines.
fn paint_guides_icon(p: &Painter, r: Rect) {
    let c = r.center();
    let stroke = Stroke::new(1.2, GUIDE_BLUE);
    p.line_segment(
        [c + Vec2::new(-7.0, -2.5), c + Vec2::new(7.0, -2.5)],
        stroke,
    );
    p.line_segment([c + Vec2::new(2.5, -7.0), c + Vec2::new(2.5, 7.0)], stroke);
    p.circle_filled(c + Vec2::new(2.5, -2.5), 1.6, GUIDE_BLUE);
}

/// A three by three grid.
fn paint_grid_icon(p: &Painter, r: Rect) {
    let c = r.center();
    let stroke = Stroke::new(1.0, crate::theme::Tokens::text_dim());
    for i in 0..4 {
        let t = -6.0 + 4.0 * i as f32;
        p.line_segment([c + Vec2::new(t, -6.0), c + Vec2::new(t, 6.0)], stroke);
        p.line_segment([c + Vec2::new(-6.0, t), c + Vec2::new(6.0, t)], stroke);
    }
}

/// The open (pointing down) or closed (pointing right) arrow.
fn paint_arrow(ui: &Ui, r: Rect, open: bool) {
    let c = r.center();
    let pts = if open {
        vec![
            c + Vec2::new(-3.5, -1.8),
            c + Vec2::new(3.5, -1.8),
            c + Vec2::new(0.0, 2.2),
        ]
    } else {
        vec![
            c + Vec2::new(-1.8, -3.5),
            c + Vec2::new(2.2, 0.0),
            c + Vec2::new(-1.8, 3.5),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        pts,
        Tokens::text_dim(),
        Stroke::NONE,
    ));
}

/// The swatch of an object: its fill (or a crossed box for none) inside
/// its outline colour.
fn paint_swatch(ui: &Ui, r: Rect, s: &Shape) {
    let p = ui.painter();
    let fill = crate::app::fill_preview_color(&s.fill);
    let edge = s
        .stroke
        .as_ref()
        .map(|st| crate::canvas::to_color32(st.color))
        .unwrap_or(Tokens::text_dim());
    match fill {
        Some(c) => {
            p.rect_filled(r, 1.0, c);
        }
        None => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.line_segment(
                [r.left_bottom(), r.right_top()],
                Stroke::new(1.0, Color32::from_rgb(0xE0, 0x40, 0x40)),
            );
        }
    }
    p.rect_stroke(
        r,
        1.0,
        Stroke::new(1.0, edge),
        egui::epaint::StrokeKind::Inside,
    );
}

/// A small green badge with a white plus at the top right of `r`.
fn paint_plus_badge(p: &Painter, r: Rect) {
    let badge = Rect::from_min_size(r.right_top() + Vec2::new(-7.0, -1.0), Vec2::splat(8.0));
    p.rect_filled(badge, 1.0, PLUS_GREEN);
    let c = badge.center();
    let w = Stroke::new(1.3, Color32::WHITE);
    p.line_segment([c - Vec2::new(2.5, 0.0), c + Vec2::new(2.5, 0.0)], w);
    p.line_segment([c - Vec2::new(0.0, 2.5), c + Vec2::new(0.0, 2.5)], w);
}

/// A waste bin, pale when `on` is false.
fn paint_bin(p: &Painter, r: Rect, on: bool) {
    let c = r.center();
    let color = if on {
        Tokens::icon()
    } else {
        Tokens::icon_off()
    };
    let stroke = Stroke::new(1.2, color);
    p.line_segment(
        [c + Vec2::new(-6.0, -4.5), c + Vec2::new(6.0, -4.5)],
        stroke,
    );
    p.line_segment(
        [c + Vec2::new(-2.0, -6.5), c + Vec2::new(2.0, -6.5)],
        stroke,
    );
    p.add(egui::Shape::closed_line(
        vec![
            c + Vec2::new(-4.5, -4.5),
            c + Vec2::new(4.5, -4.5),
            c + Vec2::new(3.5, 6.5),
            c + Vec2::new(-3.5, 6.5),
        ],
        stroke,
    ));
    for dx in [-1.6f32, 1.6] {
        p.line_segment(
            [c + Vec2::new(dx, -2.0), c + Vec2::new(dx * 0.8, 4.5)],
            Stroke::new(0.9, color),
        );
    }
}

/// A full-width row; returns its rectangle and its response.
fn row(ui: &mut Ui, look: Look) -> (Rect, egui::Response) {
    row_of(ui, look, ROW_H)
}

/// A full-width row `height` high.
fn row_of(ui: &mut Ui, look: Look, height: f32) -> (Rect, egui::Response) {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    let p = ui.painter();
    match look {
        Look::Object { selected: true } => {
            p.rect_filled(rect, 0.0, crate::theme::Tokens::selected_row());
            p.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, crate::theme::Tokens::selected_row_line()),
                egui::epaint::StrokeKind::Inside,
            );
        }
        Look::Layer { active: true } => {
            p.rect_filled(rect, 0.0, crate::theme::Tokens::tool_active());
            p.rect_filled(
                Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
                0.0,
                Tokens::ACCENT,
            );
        }
        _ if resp.hovered() => {
            p.rect_filled(rect, 0.0, crate::theme::Tokens::hovered_row());
        }
        Look::Layer { active: false } => {
            p.rect_filled(rect, 0.0, crate::theme::Tokens::tab_strip());
        }
        _ => {}
    }
    (rect, resp)
}

/// A clickable square of `size` at `x` in the row `rect`.
fn hot(ui: &mut Ui, rect: Rect, x: f32, size: f32, id: egui::Id) -> (Rect, egui::Response) {
    let r = square(rect, x, size);
    (r, ui.interact(r, id, Sense::click()))
}

/// The square of `size` at `x`, centred on the row `rect`.
fn square(rect: Rect, x: f32, size: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(x, rect.center().y - size / 2.0),
        Vec2::splat(size),
    )
}

/// The docker: the tree above the bar with its buttons.
pub fn objects_docker(app: &mut App, ui: &mut Ui) {
    let content_h = (ui.available_height() - BAR_H - 4.0).max(40.0);
    let out = egui::ScrollArea::vertical()
        .max_height(content_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            objects(app, ui);
        });
    // Ctrl+wheel over the list makes the thumbnails larger or smaller.
    if ui.rect_contains_pointer(out.inner_rect) {
        let notch = wheel_notches(ui, true);
        zoom_thumbs(app, notch);
    }
    bottom_bar(app, ui);
}

/// Wheel notches this frame (up is positive), only those with Ctrl (or
/// Cmd) held when `ctrl`, else all of them.
fn wheel_notches(ui: &Ui, ctrl: bool) -> f32 {
    ui.input(|i| {
        i.events
            .iter()
            .map(|e| match e {
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers,
                    ..
                } if !ctrl || modifiers.command || modifiers.ctrl => {
                    let d = delta.y + delta.x;
                    match unit {
                        egui::MouseWheelUnit::Line => d,
                        egui::MouseWheelUnit::Point => d / 50.0,
                        egui::MouseWheelUnit::Page => d * 10.0,
                    }
                }
                _ => 0.0,
            })
            .sum()
    })
}

/// The thumbnails `notch` wheel notches larger (smaller when negative).
fn zoom_thumbs(app: &mut App, notch: f32) {
    if notch.abs() < 0.5 {
        return;
    }
    let steps = notch.round().clamp(-4.0, 4.0);
    app.settings.object_thumb_size = crate::object_thumbs::clamp_size(
        app.settings.object_thumb_size + steps * crate::object_thumbs::STEP,
    );
}

/// The object whose row the list scrolls to this frame: the first
/// selected one, when the selection is not the one the list last showed
/// (a pick on the page, a Select All, an undo), so a selection far down a
/// long list comes into view (#1044).
fn scroll_target(app: &App, ui: &Ui) -> Option<ShapeId> {
    let seen = egui::Id::new("objects_seen_selection");
    let changed = ui.data_mut(|d| {
        let last = d.get_temp::<Vec<ShapeId>>(seen);
        d.insert_temp(seen, app.selection.clone());
        last.as_ref() != Some(&app.selection)
    });
    changed.then(|| app.selection.first().copied()).flatten()
}

/// The tree: the page and its layers, then the Master Page.
pub fn objects(app: &mut App, ui: &mut Ui) {
    let mut act = Actions {
        scroll_to: scroll_target(app, ui),
        thumbs: std::mem::take(&mut app.object_thumbs),
        thumb_size: crate::object_thumbs::clamp_size(app.settings.object_thumb_size),
        drawing: (app.engine.instance(), app.engine.revision()),
        ..Default::default()
    };
    act.thumbs.begin_frame();
    {
        let doc = app.doc();
        let Ok(page) = doc.page(app.page) else { return };
        let index = doc.pages.iter().position(|p| p.id == app.page).unwrap_or(0);
        let active = app.active_layer();
        ui.spacing_mut().item_spacing.y = 0.0;
        let page_open = !app.objects_page_closed;
        let (toggle, resp) = node_row(ui, "objects_page", &page.name, page_open, false);
        act.toggle_page = toggle;
        // Double-click (or the context menu) renames the page.
        if resp.double_clicked() {
            act.rename_page = Some(page.name.clone());
        }
        resp.context_menu(|ui| {
            if ui.button(tr("docker.rename")).clicked() {
                act.rename_page = Some(page.name.clone());
                ui.close();
            }
        });
        if page_open {
            let count = page.layers.len();
            for (i, layer) in page.layers.iter().enumerate().rev() {
                let place = Place {
                    index: i,
                    count,
                    master: false,
                    shown: true,
                };
                layer_rows(app, ui, layer, place, active, &mut act);
            }
        }
        let master_open = !app.objects_master_closed;
        act.toggle_master = node_row(
            ui,
            "objects_master",
            &tr("docker.master_page"),
            master_open,
            true,
        )
        .0;
        if master_open {
            let count = doc.master.len();
            for (i, layer) in doc.master.iter().enumerate().rev() {
                let place = Place {
                    index: i,
                    count,
                    master: true,
                    shown: layer.scope.applies(index),
                };
                layer_rows(app, ui, layer, place, active, &mut act);
            }
            guides_row(app, ui, page, &mut act);
            grid_row(app, ui, &mut act);
        }
    }
    let mut thumbs = std::mem::take(&mut act.thumbs);
    thumbs.trim(crate::object_thumbs::MAX_BYTES);
    if thumbs.waiting {
        // New pictures wait for the next frames.
        ui.ctx().request_repaint();
    }
    app.object_thumbs = thumbs;
    apply(app, ui, act);
}

/// The page or Master Page node: an arrow, the page icon and the title in
/// bold. True when clicked (to open or close it), and the row's response.
fn node_row(
    ui: &mut Ui,
    id: &str,
    title: &str,
    open: bool,
    master: bool,
) -> (bool, egui::Response) {
    let (rect, resp) = row(ui, Look::Node);
    let resp = resp.rich_tip_with(|| format!("{title}\n{}", tr("docker.page_row_desc")));
    let x = rect.left() + 3.0;
    let (ar, a) = hot(ui, rect, x, 14.0, egui::Id::new((id, "arrow")));
    let a = a.rich_tip_with(|| crate::ui::tips::keyed("docker.expand_row"));
    paint_arrow(ui, ar, open);
    paint_page_icon(ui.painter(), square(rect, x + 16.0, 16.0), master);
    ui.painter().text(
        Pos2::new(x + 38.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        crate::theme::bold(13.0),
        Tokens::text(),
    );
    // A double-click is two clicks, so renaming closes and reopens the
    // node and leaves it as it was.
    (a.clicked() || resp.clicked(), resp)
}

/// A layer's row, then its objects when it is open.
fn layer_rows(
    app: &App,
    ui: &mut Ui,
    layer: &Layer,
    at: Place,
    active: Option<LayerId>,
    act: &mut Actions,
) {
    let open = !app.objects_collapsed.contains(&layer.id);
    let is_active = active == Some(layer.id);
    let (rect, resp) = row(ui, Look::Layer { active: is_active });
    let resp = resp.rich_tip_with(|| format!("{}\n{}", layer.name, tr("docker.layer_row_desc")));
    let base = egui::Id::new(("objects_layer", layer.id.raw()));
    let mut x = rect.left() + 3.0 + INDENT;
    let (ar, a) = hot(ui, rect, x, 14.0, base.with("arrow"));
    let a = a.rich_tip_with(|| crate::ui::tips::keyed("docker.expand_row"));
    paint_arrow(ui, ar, open);
    if a.clicked() {
        act.collapse = Some(layer.id);
    }
    x += 16.0;
    let (er, e) = hot(ui, rect, x, 18.0, base.with("eye"));
    paint_eye(ui, er, layer.visible);
    if e.rich_tip(tr("docker.show_hide")).clicked() {
        act.visible = Some((layer.id, !layer.visible));
    }
    x += 20.0;
    let (pr, p) = hot(ui, rect, x, 18.0, base.with("print"));
    paint_printer(ui, pr, layer.printable);
    if p.rich_tip(tr("docker.print_layer")).clicked() {
        act.printable = Some((layer.id, !layer.printable));
    }
    x += 20.0;
    let (lr, l) = hot(ui, rect, x, 18.0, base.with("lock"));
    paint_lock(ui, lr, layer.locked);
    if l.rich_tip(tr("docker.lock_layer")).clicked() {
        act.locked = Some((layer.id, !layer.locked));
    }
    x += 22.0;
    paint_layer_icon(ui.painter(), square(rect, x, 16.0), at.master);
    x += 22.0;
    let font = if is_active {
        crate::theme::bold(13.0)
    } else {
        egui::FontId::proportional(13.0)
    };
    let color = if at.shown {
        Tokens::text()
    } else {
        Tokens::text_dim()
    };
    let name = ui.painter().text(
        Pos2::new(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &layer.name,
        font,
        color,
    );
    if at.master && layer.scope != MasterScope::All {
        ui.painter().text(
            Pos2::new(name.right() + 6.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            tr(scope_key(layer.scope)),
            egui::FontId::proportional(11.0),
            Tokens::text_dim(),
        );
    }
    if resp.double_clicked() {
        act.rename_layer = Some((layer.id, layer.name.clone()));
    } else if resp.clicked() {
        act.pick = Some(layer.id);
    }
    resp.context_menu(|ui| {
        if ui.button(tr("docker.rename")).clicked() {
            act.rename_layer = Some((layer.id, layer.name.clone()));
            ui.close();
        }
        if ui
            .add_enabled(
                at.index + 1 < at.count,
                egui::Button::new(tr("docker.move_up")),
            )
            .clicked()
        {
            act.reorder = Some((layer.id, at.index + 1));
            ui.close();
        }
        if ui
            .add_enabled(at.index > 0, egui::Button::new(tr("docker.move_down")))
            .clicked()
        {
            act.reorder = Some((layer.id, at.index.saturating_sub(1)));
            ui.close();
        }
        if at.master {
            ui.menu_button(tr("docker.show_on"), |ui| {
                for scope in [MasterScope::All, MasterScope::Odd, MasterScope::Even] {
                    if ui
                        .selectable_label(layer.scope == scope, tr(scope_key(scope)))
                        .clicked()
                    {
                        act.scope = Some((layer.id, scope));
                        ui.close();
                    }
                }
            });
            let mut front = layer.in_front;
            if ui
                .checkbox(&mut front, tr("docker.master_in_front"))
                .clicked()
            {
                act.in_front = Some((layer.id, front));
                ui.close();
            }
        }
        if ui
            .add_enabled(
                at.master || at.count > 1,
                egui::Button::new(tr("docker.delete_layer")),
            )
            .clicked()
        {
            act.delete_layer = Some(layer.id);
            ui.close();
        }
    });
    if open {
        for s in layer.shapes.iter().rev() {
            object_rows(app, ui, s, 2, act);
        }
    }
}

/// The text naming the pages a master layer shows on.
fn scope_key(scope: MasterScope) -> &'static str {
    match scope {
        MasterScope::All => "options.scope_all",
        MasterScope::Odd => "options.scope_odd",
        MasterScope::Even => "options.scope_even",
    }
}

/// The Guides row of the Master Page: the eye shows or hides the
/// guidelines, the padlock locks or unlocks all of the page's; guidelines
/// never print.
fn guides_row(app: &App, ui: &mut Ui, page: &Page, act: &mut Actions) {
    let (rect, resp) = row(ui, Look::Layer { active: false });
    resp.rich_tip_with(|| crate::ui::tips::pair("docker.guides", "docker.guides_row_desc"));
    let base = egui::Id::new("objects_guides");
    let mut x = rect.left() + 3.0 + INDENT + 16.0;
    let (er, e) = hot(ui, rect, x, 18.0, base.with("eye"));
    paint_eye(ui, er, app.show_guides);
    if e.rich_tip(tr("docker.show_hide")).clicked() {
        act.toggle_guides = true;
    }
    x += 20.0;
    paint_printer(ui, square(rect, x, 18.0), false);
    x += 20.0;
    let any = !page.guides.is_empty();
    let locked = any && page.guides.iter().all(|g| g.locked);
    let (lr, l) = hot(ui, rect, x, 18.0, base.with("lock"));
    paint_lock(ui, lr, locked);
    if l.rich_tip(tr("docker.lock_guides")).clicked() && any {
        act.lock_guides = Some(!locked);
    }
    x += 22.0;
    paint_guides_icon(ui.painter(), square(rect, x, 16.0));
    x += 22.0;
    ui.painter().text(
        Pos2::new(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        tr("docker.guides"),
        egui::FontId::proportional(13.0),
        Tokens::text(),
    );
}

/// The Document Grid row of the Master Page: the eye shows or hides the
/// grid, which never prints and cannot be edited.
fn grid_row(app: &App, ui: &mut Ui, act: &mut Actions) {
    let (rect, resp) = row(ui, Look::Layer { active: false });
    resp.rich_tip_with(|| crate::ui::tips::pair("menu.view.document_grid", "docker.grid_row_desc"));
    let mut x = rect.left() + 3.0 + INDENT + 16.0;
    let (er, e) = hot(ui, rect, x, 18.0, egui::Id::new("objects_grid_eye"));
    paint_eye(ui, er, app.show_grid);
    if e.rich_tip(tr("docker.show_hide")).clicked() {
        act.toggle_grid = true;
    }
    x += 20.0;
    paint_printer(ui, square(rect, x, 18.0), false);
    x += 20.0;
    paint_padlock(
        ui.painter(),
        square(rect, x, 18.0),
        true,
        Tokens::icon_off(),
    );
    x += 22.0;
    paint_grid_icon(ui.painter(), square(rect, x, 16.0));
    x += 22.0;
    ui.painter().text(
        Pos2::new(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        tr("menu.view.document_grid"),
        egui::FontId::proportional(13.0),
        Tokens::text(),
    );
}

/// The row of `s` at nesting `depth`, then its children when it is an
/// open group.
fn object_rows(app: &App, ui: &mut Ui, s: &Shape, depth: usize, act: &mut Actions) {
    let selected = app.selection.contains(&s.id);
    let height = (act.thumb_size + 6.0).max(ROW_H);
    let (rect, resp) = row_of(ui, Look::Object { selected }, height);
    if act.scroll_to == Some(s.id) {
        ui.scroll_to_rect(rect, None);
    }
    let children: Option<&Vec<Shape>> = match &s.kind {
        ShapeKind::Group { children } => Some(children),
        _ => None,
    };
    let expanded = app.objects_open_groups.contains(&s.id);
    // Rows scrolled out of sight are only laid out: nothing is drawn or
    // asked of them, so a long list costs little.
    if ui.is_rect_visible(rect) {
        object_row(app, ui, s, (rect, resp), depth, expanded, act);
    }
    if let (Some(children), true) = (children, expanded) {
        for c in children.iter().rev() {
            object_rows(app, ui, c, depth + 1, act);
        }
    }
}

/// The contents of the row of `s`: the group arrow, the thumbnail, the
/// name, the eye and the padlock.
fn object_row(
    app: &App,
    ui: &mut Ui,
    s: &Shape,
    (rect, resp): (Rect, egui::Response),
    depth: usize,
    expanded: bool,
    act: &mut Actions,
) {
    let mut x = rect.left() + 3.0 + depth as f32 * INDENT;
    if matches!(s.kind, ShapeKind::Group { .. }) {
        let (ar, a) = hot(
            ui,
            rect,
            x,
            14.0,
            egui::Id::new(("objects_group", s.id.raw())),
        );
        let a = a.rich_tip_with(|| crate::ui::tips::keyed("docker.expand_row"));
        paint_arrow(ui, ar, expanded);
        if a.clicked() {
            act.expand_group = Some(s.id);
        }
    }
    x += 16.0;
    let size = act.thumb_size;
    paint_thumb(
        app,
        ui,
        Rect::from_min_size(
            Pos2::new(x, rect.center().y - size / 2.0),
            Vec2::splat(size),
        ),
        s,
        act,
    );
    x += size + 7.0;
    let label = object_label(s);
    let resp = resp.rich_tip_with(|| format!("{label}\n{}", tr("docker.object_row_desc")));
    let color = if s.visible {
        Tokens::text()
    } else {
        Tokens::text_dim()
    };
    // The name stops short of the eye and padlock.
    let name_room = Rect::from_min_max(rect.min, Pos2::new(rect.right() - 48.0, rect.max.y));
    ui.painter().with_clip_rect(name_room).text(
        Pos2::new(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &label,
        egui::FontId::proportional(13.0),
        color,
    );
    // Its own eye and padlock at the right end of the row, so an object
    // is hidden or locked from the list, and a locked one unlocked (#832).
    let base = egui::Id::new(("objects_object", s.id.raw()));
    let lock_x = rect.right() - 22.0;
    let eye_x = lock_x - 20.0;
    let (er, e) = hot(ui, rect, eye_x, 18.0, base.with("eye"));
    paint_eye(ui, er, s.visible);
    if e.rich_tip(tr("docker.show_hide")).clicked() {
        act.object_visible = Some((s.id, !s.visible));
    }
    let (lr, l) = hot(ui, rect, lock_x, 18.0, base.with("lock"));
    paint_lock(ui, lr, s.locked);
    let lock_tip = if s.locked {
        "menu.object.unlock_object"
    } else {
        "menu.object.lock_object"
    };
    if l.rich_tip(tr(lock_tip)).clicked() {
        act.object_locked = Some((s.id, !s.locked));
    }
    if resp.double_clicked() {
        act.rename_object = Some((s.id, s.name.clone().unwrap_or_else(|| label.clone())));
    } else if resp.clicked() {
        act.select = Some(s.id);
    }
}

/// The thumbnail of `s` in the square `r`: the object drawn small on a
/// white tile, centred; dimmed when the object is hidden. Until its
/// picture is drawn (a few per frame), the swatch of its fill and outline
/// stands in.
fn paint_thumb(app: &App, ui: &Ui, r: Rect, s: &Shape, act: &mut Actions) {
    let p = ui.painter();
    p.rect_filled(r, 2.0, Color32::WHITE);
    p.rect_stroke(
        r,
        2.0,
        Stroke::new(1.0, Tokens::border()),
        egui::epaint::StrokeKind::Inside,
    );
    let ppp = ui.ctx().pixels_per_point().max(0.5);
    let inner = r.shrink(2.0);
    let px = (inner.width() * ppp).round().max(4.0) as u32;
    match act.thumbs.get(ui.ctx(), app.doc(), act.drawing, s, px) {
        crate::object_thumbs::Thumb::Picture(id, [w, h]) => {
            let size = Vec2::new(w as f32, h as f32) / ppp;
            // An older picture of another size still fits the tile.
            let fit = (inner.width() / size.x.max(size.y).max(1.0)).min(1.0);
            let at = Rect::from_center_size(inner.center(), size * fit);
            let tint = if s.visible {
                Color32::WHITE
            } else {
                Color32::from_white_alpha(90)
            };
            p.image(
                id,
                at,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
        }
        crate::object_thumbs::Thumb::Empty => {}
        crate::object_thumbs::Thumb::Pending => {
            let side = (inner.width() * 0.6).max(8.0);
            paint_swatch(
                ui,
                Rect::from_center_size(inner.center(), Vec2::splat(side)),
                s,
            );
        }
    }
}

/// The bar under the tree: New Layer and New Master Layer (all pages; its
/// context menu offers odd or even pages) at the left, Delete at the right.
fn bottom_bar(app: &mut App, ui: &mut Ui) {
    let (bar, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), BAR_H), Sense::hover());
    ui.painter()
        .hline(bar.x_range(), bar.min.y, Stroke::new(1.0, Tokens::border()));
    let slot =
        |x: f32| Rect::from_center_size(Pos2::new(x, bar.center().y + 1.0), Vec2::splat(24.0));
    let new_r = slot(bar.min.x + 14.0);
    let master_r = slot(bar.min.x + 42.0);
    let delete_r = slot(bar.max.x - 14.0);
    let can_delete = !app.selection.is_empty() || deletable_layer(app).is_some();
    let new = ui
        .interact(new_r, egui::Id::new("objects_new_layer"), Sense::click())
        .rich_tip(tr("docker.new_layer"));
    let master = ui
        .interact(
            master_r,
            egui::Id::new("objects_new_master"),
            Sense::click(),
        )
        .rich_tip(tr("docker.new_master_layer"));
    let delete = ui
        .interact(
            delete_r,
            egui::Id::new("objects_delete"),
            if can_delete {
                Sense::click()
            } else {
                Sense::hover()
            },
        )
        .rich_tip(tr("docker.delete_hint"));
    let p = ui.painter();
    for (r, resp, on) in [
        (new_r, &new, true),
        (master_r, &master, true),
        (delete_r, &delete, can_delete),
    ] {
        if on && resp.hovered() {
            p.rect_filled(r, 2.0, Tokens::tool_hover());
        }
    }
    let icon = |r: Rect| Rect::from_center_size(r.center(), Vec2::splat(16.0));
    paint_layer_icon(p, icon(new_r), false);
    paint_plus_badge(p, icon(new_r));
    paint_layer_icon(p, icon(master_r), true);
    paint_plus_badge(p, icon(master_r));
    paint_bin(p, icon(delete_r), can_delete);
    let mut scoped = None;
    master.context_menu(|ui| {
        for (scope, key) in [
            (MasterScope::All, "options.new_master_all"),
            (MasterScope::Odd, "options.new_master_odd"),
            (MasterScope::Even, "options.new_master_even"),
        ] {
            if ui.button(tr(key)).clicked() {
                scoped = Some(scope);
                ui.close();
            }
        }
    });
    if new.clicked() {
        new_layer(app);
    }
    if master.clicked() {
        new_master_layer(app, MasterScope::All);
    }
    if let Some(scope) = scoped {
        new_master_layer(app, scope);
    }
    if can_delete && delete.clicked() {
        delete_current(app);
    }
    // The thumbnail size between the buttons, when there is room.
    let zoom = Rect::from_min_max(
        Pos2::new(master_r.right() + 10.0, bar.min.y + 2.0),
        Pos2::new(delete_r.left() - 10.0, bar.max.y),
    );
    if zoom.width() >= 70.0 {
        thumb_size_control(app, ui, zoom);
    }
}

/// The thumbnail size control in `r`: a small tile, a slider and a large
/// tile; the wheel over it changes the size too.
fn thumb_size_control(app: &mut App, ui: &mut Ui, r: Rect) {
    use crate::object_thumbs::{clamp_size, MAX_SIZE, MIN_SIZE, STEP};
    let small = Rect::from_center_size(Pos2::new(r.min.x + 4.0, r.center().y), Vec2::splat(7.0));
    let large = Rect::from_center_size(Pos2::new(r.max.x - 7.0, r.center().y), Vec2::splat(13.0));
    for t in [small, large] {
        ui.painter().rect_stroke(
            t,
            1.0,
            Stroke::new(1.0, Tokens::icon()),
            egui::epaint::StrokeKind::Inside,
        );
    }
    let groove = Rect::from_min_max(
        Pos2::new(small.right() + 6.0, r.min.y),
        Pos2::new(large.left() - 6.0, r.max.y),
    );
    let mut size = clamp_size(app.settings.object_thumb_size);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(groove)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().slider_width = groove.width();
            ui.add(
                crate::ui::Slider::new(&mut size, MIN_SIZE..=MAX_SIZE)
                    .step_by(f64::from(STEP))
                    .show_value(false),
            )
            .rich_tip(tr("docker.thumb_size"));
        },
    );
    app.settings.object_thumb_size = clamp_size(size);
    if ui.rect_contains_pointer(r) {
        let notch = wheel_notches(ui, false);
        zoom_thumbs(app, notch);
    }
}

/// `key`'s name with the first number from `from` up that `names` does
/// not use yet.
fn free_name(names: &[String], key: &str, from: usize) -> String {
    (from..)
        .map(|n| trf(key, &[("n", &n.to_string())]))
        .find(|name| !names.contains(name))
        .unwrap_or_default()
}

/// Add a layer on top of the current page's and make it active.
pub fn new_layer(app: &mut App) {
    let page = app.page;
    let names: Vec<String> = app
        .doc()
        .page(page)
        .map(|p| p.layers.iter().map(|l| l.name.clone()).collect())
        .unwrap_or_default();
    let name = free_name(&names, "docker.layer_n", names.len() + 1);
    app.run(Command::AddLayer { page, name });
    app.picked_layer = app
        .doc()
        .page(page)
        .ok()
        .and_then(|p| p.layers.last())
        .map(|l| l.id);
    app.objects_page_closed = false;
}

/// Add a master layer showing on the pages `scope` names and make it
/// active.
pub fn new_master_layer(app: &mut App, scope: MasterScope) {
    let names: Vec<String> = app.doc().master.iter().map(|l| l.name.clone()).collect();
    let name = free_name(&names, "docker.master_layer_n", names.len() + 1);
    app.run(Command::AddMasterLayer { name, scope });
    app.picked_layer = app.doc().master.last().map(|l| l.id);
    app.objects_master_closed = false;
}

/// The active layer when Delete can take it: a master layer, or one of
/// several layers of the page (a page keeps its last layer).
fn deletable_layer(app: &App) -> Option<LayerId> {
    let id = app.active_layer()?;
    let doc = app.doc();
    if doc.master.iter().any(|l| l.id == id) {
        return Some(id);
    }
    let page = doc.page(app.page).ok()?;
    (page.layers.len() > 1 && page.layers.iter().any(|l| l.id == id)).then_some(id)
}

/// The bar's Delete: the selected objects, else the active layer.
pub fn delete_current(app: &mut App) {
    if !app.selection.is_empty() {
        app.delete_selection();
    } else if let Some(layer) = deletable_layer(app) {
        delete_layer(app, layer);
    }
}

fn delete_layer(app: &mut App, layer: LayerId) {
    app.selection.clear();
    app.run(Command::DeleteLayer { layer });
    if app.picked_layer == Some(layer) {
        app.picked_layer = None;
    }
}

/// The layer holding `id` (inside groups too).
fn layer_holding(app: &App, id: ShapeId) -> Option<LayerId> {
    let doc = app.doc();
    let top = doc.top_level_of(id)?;
    doc.all_layers()
        .find(|l| l.shapes.iter().any(|s| s.id == top))
        .map(|l| l.id)
}

fn apply(app: &mut App, ui: &Ui, act: Actions) {
    if let Some((layer, visible)) = act.visible {
        app.run(Command::SetLayerVisible { layer, visible });
    }
    if let Some((layer, printable)) = act.printable {
        app.run(Command::SetLayerPrintable { layer, printable });
    }
    if let Some((layer, locked)) = act.locked {
        app.run(Command::SetLayerLocked { layer, locked });
    }
    if let Some((id, visible)) = act.object_visible {
        app.run(Command::SetVisible {
            shapes: vec![id],
            visible,
        });
    }
    if let Some((id, locked)) = act.object_locked {
        app.run(Command::SetLocked {
            shapes: vec![id],
            locked,
        });
    }
    if let Some((layer, scope)) = act.scope {
        app.run(Command::SetLayerScope { layer, scope });
    }
    if let Some((layer, in_front)) = act.in_front {
        app.run(Command::SetLayerInFront { layer, in_front });
    }
    if let Some(layer) = act.collapse {
        if !app.objects_collapsed.remove(&layer) {
            app.objects_collapsed.insert(layer);
        }
    }
    if let Some(group) = act.expand_group {
        if !app.objects_open_groups.remove(&group) {
            app.objects_open_groups.insert(group);
        }
    }
    if act.toggle_page {
        app.objects_page_closed = !app.objects_page_closed;
    }
    if act.toggle_master {
        app.objects_master_closed = !app.objects_master_closed;
    }
    if act.toggle_guides {
        app.show_guides = !app.show_guides;
    }
    if act.toggle_grid {
        app.show_grid = !app.show_grid;
    }
    if let Some(locked) = act.lock_guides {
        let all: Vec<usize> = (0..app.page_guides().len()).collect();
        app.set_guides_locked(&all, locked);
    }
    if let Some((layer, name)) = act.rename_layer {
        app.dialog = crate::ui::dialogs::Dialog::RenameLayer { layer, name };
    }
    if let Some(name) = act.rename_page {
        app.dialog = crate::ui::dialogs::Dialog::RenamePage { name };
    }
    if let Some((shape, name)) = act.rename_object {
        app.dialog = crate::ui::dialogs::Dialog::RenameObject { shape, name };
    }
    if let Some(layer) = act.delete_layer {
        delete_layer(app, layer);
    }
    if let Some((layer, index)) = act.reorder {
        app.run(Command::ReorderLayer { layer, index });
    }
    if let Some(layer) = act.pick {
        app.picked_layer = Some(layer);
        app.select(Vec::new());
    }
    if let Some(id) = act.select {
        if ui.input(|i| i.modifiers.shift) {
            if !app.selection.contains(&id) {
                app.selection.push(id);
            }
        } else {
            app.select(vec![id]);
        }
        if let Some(layer) = layer_holding(app, id) {
            app.picked_layer = Some(layer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_kind() -> ShapeKind {
        ShapeKind::Rect {
            rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
            radius: 0.0,
            corners: None,
        }
    }

    fn with_ui(mut f: impl FnMut(&mut Ui)) {
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 600.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| f(ui));
        out.textures_delta.clear();
    }

    fn draw(app: &mut App) {
        with_ui(|ui| objects_docker(app, ui));
    }

    #[test]
    fn objects_show_their_names_or_what_they_are() {
        crate::i18n::set_language("en");
        let mut s = Shape::new(ShapeId(1), rect_kind());
        assert_eq!(object_label(&s), "Rectangle");
        s.name = Some("Water".into());
        assert_eq!(object_label(&s), "Water");
        s.name = Some("   ".into());
        assert_eq!(object_label(&s), "Rectangle");
    }

    /// Each object's row has its own eye and padlock: a click hides or
    /// shows it, locks or unlocks it, one undo step each (#832).
    #[test]
    fn an_objects_eye_and_padlock_hide_and_lock_it() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app.new_shape(rect_kind()).expect("shape");
        let run = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 600.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| objects_docker(app, ui));
            out.textures_delta.clear();
        };
        let click = |app: &mut App, what: &str| {
            run(app, Vec::new());
            let wid = egui::Id::new(("objects_object", id.raw())).with(what);
            let at = ctx.read_response(wid).expect("drawn").rect.center();
            run(app, vec![egui::Event::PointerMoved(at)]);
            let button = |pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            run(app, vec![button(true)]);
            run(app, vec![button(false)]);
        };
        click(&mut app, "eye");
        assert!(!app.doc().find_shape(id).expect("shape").visible);
        click(&mut app, "eye");
        assert!(app.doc().find_shape(id).expect("shape").visible);
        click(&mut app, "lock");
        assert!(app.doc().find_shape(id).expect("shape").locked);
        app.undo();
        assert!(!app.doc().find_shape(id).expect("shape").locked);
    }

    /// Selecting an object on the page scrolls the list to its row when
    /// it is out of view, as when it lies far down a long list (#1044).
    /// A selection that stays the same leaves the scroll alone.
    /// One frame of the docker, `size` wide and high, with `events`.
    fn frame(ctx: &egui::Context, app: &mut App, size: f32, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size, size),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| objects_docker(app, ui));
        out.textures_delta.clear();
    }

    fn wheel(at: Pos2, notches: f32, modifiers: egui::Modifiers) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: Vec2::new(0.0, notches),
                phase: egui::TouchPhase::Move,
                modifiers,
            },
        ]
    }

    /// Each object row shows a thumbnail of the object, drawn once and
    /// drawn again only when that object changes (#896).
    #[test]
    fn rows_show_thumbnails_drawn_once_per_change() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let ids: Vec<ShapeId> = (0..3)
            .map(|_| app.new_shape(rect_kind()).expect("shape"))
            .collect();
        app.selection.clear();
        for _ in 0..3 {
            frame(&ctx, &mut app, 400.0, Vec::new());
        }
        assert_eq!(app.object_thumbs.renders, 3);
        assert_eq!(app.object_thumbs.len(), 3);
        assert!(app.object_thumbs.bytes() > 0);
        // A change to one object draws that one again, the others stay.
        app.run(Command::SetFill {
            shapes: vec![ids[1]],
            fill: tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(0, 200, 0)),
        });
        for _ in 0..3 {
            frame(&ctx, &mut app, 400.0, Vec::new());
        }
        assert_eq!(app.object_thumbs.renders, 4);
        // Selecting (no change to the drawing) draws nothing.
        app.select(vec![ids[0]]);
        frame(&ctx, &mut app, 400.0, Vec::new());
        assert_eq!(app.object_thumbs.renders, 4);
    }

    /// Ctrl+wheel over the list and the wheel over the bar's slider
    /// change the thumbnail size by one step a notch; the rows grow.
    #[test]
    fn ctrl_wheel_and_the_bar_change_the_thumbnail_size() {
        use crate::object_thumbs::{DEFAULT_SIZE, MAX_SIZE, STEP};
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app.new_shape(rect_kind()).expect("shape");
        app.selection.clear();
        assert_eq!(app.settings.object_thumb_size, DEFAULT_SIZE);
        let eye = |ctx: &egui::Context| {
            ctx.read_response(egui::Id::new(("objects_object", id.raw())).with("eye"))
                .map(|r| r.rect)
        };
        frame(&ctx, &mut app, 400.0, Vec::new());
        let before = eye(&ctx).expect("row drawn");
        // A plain wheel over the list scrolls; it leaves the size.
        frame(
            &ctx,
            &mut app,
            400.0,
            wheel(Pos2::new(200.0, 60.0), 1.0, egui::Modifiers::NONE),
        );
        assert_eq!(app.settings.object_thumb_size, DEFAULT_SIZE);
        frame(
            &ctx,
            &mut app,
            400.0,
            wheel(Pos2::new(200.0, 60.0), 2.0, egui::Modifiers::CTRL),
        );
        assert_eq!(app.settings.object_thumb_size, DEFAULT_SIZE + 2.0 * STEP);
        // (Responses are read a frame late.)
        frame(&ctx, &mut app, 400.0, Vec::new());
        frame(&ctx, &mut app, 400.0, Vec::new());
        // The row is taller; the thumbnail is drawn again at the new size.
        let after = eye(&ctx).expect("row drawn");
        assert!(after.center().y > before.center().y, "{before:?} {after:?}");
        // The wheel over the slider in the bottom bar, down a notch.
        frame(
            &ctx,
            &mut app,
            400.0,
            wheel(Pos2::new(200.0, 385.0), -1.0, egui::Modifiers::NONE),
        );
        assert_eq!(app.settings.object_thumb_size, DEFAULT_SIZE + STEP);
        // Never past the largest size.
        for _ in 0..10 {
            frame(
                &ctx,
                &mut app,
                400.0,
                wheel(Pos2::new(200.0, 60.0), 4.0, egui::Modifiers::CTRL),
            );
        }
        assert_eq!(app.settings.object_thumb_size, MAX_SIZE);
    }

    /// A drawing of thousands of objects: only the rows in sight get
    /// thumbnails, a frame with nothing changed draws none, and the cache
    /// stays small.
    #[test]
    fn a_long_list_draws_only_the_rows_in_sight() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for i in 0..3000 {
            let x = f64::from(i % 50) * 4.0;
            let y = f64::from(i / 50) * 4.0;
            app.new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(x, y, x + 3.0, y + 3.0),
                radius: 0.0,
                corners: None,
            });
        }
        app.selection.clear();
        for _ in 0..6 {
            frame(&ctx, &mut app, 400.0, Vec::new());
        }
        let drawn = app.object_thumbs.renders;
        assert!((5..40).contains(&drawn), "{drawn}");
        frame(&ctx, &mut app, 400.0, Vec::new());
        assert_eq!(app.object_thumbs.renders, drawn);
        assert!(app.object_thumbs.bytes() < 2 * 1024 * 1024);
    }

    /// Frame times of the docker over 3000 objects (`cargo test --release
    /// -p tracedraw -- --ignored objects_docker_frame_times --nocapture`):
    /// the first frames (thumbnails drawn), a steady frame, and frames
    /// scrolling the list.
    #[test]
    #[ignore]
    fn objects_docker_frame_times() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for i in 0..3000 {
            let x = f64::from(i % 50) * 4.0;
            let y = f64::from(i / 50) * 4.0;
            app.new_shape(ShapeKind::Ellipse {
                rect: tracedraw_core::Rect::new(x, y, x + 3.0, y + 3.0),
                arc: None,
            });
        }
        app.selection.clear();
        let time = |app: &mut App, events: Vec<egui::Event>| {
            let t = web_time::Instant::now();
            frame(&ctx, app, 600.0, events);
            t.elapsed().as_secs_f64() * 1000.0
        };
        let first = time(&mut app, Vec::new());
        let mut fill = 0.0;
        for _ in 0..10 {
            fill += time(&mut app, Vec::new());
        }
        let steady: f64 = (0..20).map(|_| time(&mut app, Vec::new())).sum::<f64>() / 20.0;
        let scroll: f64 = (0..40)
            .map(|_| {
                time(
                    &mut app,
                    wheel(Pos2::new(300.0, 200.0), -3.0, egui::Modifiers::NONE),
                )
            })
            .sum::<f64>()
            / 40.0;
        println!(
            "objects docker, 3000 objects: first {first:.1} ms, next 10 {fill:.1} ms, \
             steady {steady:.2} ms, scrolling {scroll:.2} ms; {} thumbnails drawn, \
             {} kept, {} KB",
            app.object_thumbs.renders,
            app.object_thumbs.len(),
            app.object_thumbs.bytes() / 1024
        );
    }

    #[test]
    fn the_list_scrolls_to_an_object_selected_on_the_page() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let ids: Vec<ShapeId> = (0..80)
            .map(|_| app.new_shape(rect_kind()).expect("shape"))
            .collect();
        app.selection.clear();
        // The scroll lands in one frame.
        ctx.all_styles_mut(|s| s.scroll_animation = egui::style::ScrollAnimation::none());
        let run = |app: &mut App, scroll: f32| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(300.0, 300.0),
                )),
                events: if scroll != 0.0 {
                    vec![
                        egui::Event::PointerMoved(Pos2::new(150.0, 100.0)),
                        egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: Vec2::new(0.0, scroll),
                            phase: egui::TouchPhase::Move,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]
                } else {
                    Vec::new()
                },
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| objects_docker(app, ui));
            out.textures_delta.clear();
        };
        let row_of = |id: ShapeId| {
            ctx.read_response(egui::Id::new(("objects_object", id.raw())).with("eye"))
                .map(|r| r.rect)
        };
        let shown =
            |r: Option<egui::Rect>| r.is_some_and(|r| r.top() >= 0.0 && r.bottom() <= 300.0);
        for _ in 0..4 {
            run(&mut app, 0.0);
        }
        // The first object drawn is the bottom of the list.
        let far = ids[0];
        assert!(!shown(row_of(far)), "{:?}", row_of(far));
        app.select(vec![far]);
        for _ in 0..4 {
            run(&mut app, 0.0);
        }
        assert!(shown(row_of(far)), "{:?}", row_of(far));
        // Scrolled away by hand, the unchanged selection stays out of view.
        for _ in 0..4 {
            run(&mut app, 2000.0);
        }
        assert!(!shown(row_of(far)), "{:?}", row_of(far));
    }

    #[test]
    fn the_docker_draws_layers_groups_and_selection() {
        let mut app = App::headless();
        let id = app.new_shape(rect_kind()).expect("shape");
        app.select(vec![id]);
        new_master_layer(&mut app, MasterScope::Odd);
        let layer = app.doc().pages[0].layers[0].id;
        for collapsed in [false, true] {
            if collapsed {
                app.objects_collapsed.insert(layer);
                app.objects_master_closed = true;
            }
            draw(&mut app);
        }
        assert_eq!(app.selection, vec![id]);
    }

    #[test]
    fn new_layers_get_free_names_and_become_active() {
        crate::i18n::set_language("en");
        let mut app = App::headless();
        let first = app.doc().pages[0].layers[0].id;
        new_layer(&mut app);
        new_layer(&mut app);
        let names: Vec<String> = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.name.clone())
            .collect();
        assert_eq!(names, ["Layer 1", "Layer 2", "Layer 3"]);
        let top = app.doc().pages[0].layers[2].id;
        assert_eq!(app.active_layer(), Some(top));
        // Names in use are skipped.
        app.run(Command::DeleteLayer {
            layer: app.doc().pages[0].layers[1].id,
        });
        new_layer(&mut app);
        assert_eq!(app.doc().pages[0].layers[2].name, "Layer 4");
        // Picking a lower layer sends new objects there.
        app.picked_layer = Some(first);
        let id = app.new_shape(rect_kind()).expect("shape");
        assert!(app.doc().pages[0].layers[0]
            .shapes
            .iter()
            .any(|s| s.id == id));
        // A hidden or locked pick does not take them.
        app.run(Command::SetLayerVisible {
            layer: first,
            visible: false,
        });
        assert_ne!(app.active_layer(), Some(first));
        new_master_layer(&mut app, MasterScope::All);
        new_master_layer(&mut app, MasterScope::Even);
        let master: Vec<String> = app.doc().master.iter().map(|l| l.name.clone()).collect();
        assert_eq!(master, ["Master Layer 1", "Master Layer 2"]);
        // An even-pages master layer does not show on page 1, so it is
        // not where page 1's objects go.
        assert_ne!(app.active_layer(), app.doc().master.last().map(|l| l.id));
        app.picked_layer = app.doc().master.first().map(|l| l.id);
        assert_eq!(app.active_layer(), app.picked_layer);
    }

    #[test]
    fn master_layer_objects_are_picked_where_their_layer_shows() {
        let mut app = App::headless();
        new_master_layer(&mut app, MasterScope::Odd);
        let master = app.doc().master[0].id;
        assert_eq!(app.active_layer(), Some(master));
        let id = app.new_shape(rect_kind()).expect("shape");
        assert_eq!(app.doc().master[0].shapes.len(), 1);
        app.select(Vec::new());
        // On its outline (new rectangles have no fill).
        let inside = tracedraw_core::Point::new(0.0, 5.0);
        assert_eq!(app.hit_test(inside), Some(id));
        app.select_all();
        assert_eq!(app.selection, vec![id]);
        // Page 2 is even: the layer does not show there.
        let page = app.page;
        app.run(Command::duplicate_page(page));
        app.page = app.doc().pages[1].id;
        assert_eq!(app.hit_test(inside), None);
        app.page = page;
        app.run(Command::SetLayerLocked {
            layer: master,
            locked: true,
        });
        assert_eq!(app.hit_test(inside), None);
    }

    #[test]
    fn delete_takes_the_selection_else_the_active_layer() {
        let mut app = App::headless();
        let only = app.doc().pages[0].layers[0].id;
        let id = app.new_shape(rect_kind()).expect("shape");
        app.select(vec![id]);
        delete_current(&mut app);
        assert!(app.doc().find_shape(id).is_none());
        // A page keeps its last layer.
        assert_eq!(deletable_layer(&app), None);
        delete_current(&mut app);
        assert_eq!(app.doc().pages[0].layers.len(), 1);
        new_layer(&mut app);
        delete_current(&mut app);
        assert_eq!(app.doc().pages[0].layers.len(), 1);
        assert_eq!(app.doc().pages[0].layers[0].id, only);
        new_master_layer(&mut app, MasterScope::All);
        delete_current(&mut app);
        assert!(app.doc().master.is_empty());
        assert_eq!(app.picked_layer, None);
        assert_eq!(app.active_layer(), Some(only));
    }

    #[test]
    fn clicking_an_object_in_the_docker_makes_its_layer_active() {
        let mut app = App::headless();
        let low = app.doc().pages[0].layers[0].id;
        let id = app.new_shape(rect_kind()).expect("shape");
        new_layer(&mut app);
        assert_ne!(app.active_layer(), Some(low));
        with_ui(|ui| {
            apply(
                &mut app,
                ui,
                Actions {
                    select: Some(id),
                    ..Default::default()
                },
            );
        });
        assert_eq!(app.selection, vec![id]);
        assert_eq!(app.active_layer(), Some(low));
    }
}
