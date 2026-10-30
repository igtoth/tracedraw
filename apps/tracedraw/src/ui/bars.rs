//! Movable bars. The toolbox, the colour palette, the standard toolbar,
//! the property bar, the Text, Zoom and Transform toolbars, the time
//! tracker and the status bar dock on an edge of the window or float over
//! it (the status bar docks only). The toolbars docked along the top or
//! the bottom lie in rows, several side by side in a row (#837, #855): a
//! toolbar dropped on a row joins it where it is dropped, dropped on the
//! line between two rows it starts a row of its own. A docked bar starts with a
//! grip of dots; dragging the grip, or the title of a floating bar, shows
//! where the bar will go: near an edge it docks there, elsewhere it floats
//! where it is dropped. A right click on a grip or on the empty part of a
//! bar opens a menu with the bar's positions, the bars to show and Lock
//! Toolbars, which keeps every bar where it is. Double-clicking a floating
//! bar's title docks it back. The layout is kept in the preferences.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{pos2, vec2, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use serde::{Deserialize, Serialize};

/// Where a bar sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Dock {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
    Floating,
}

impl Dock {
    pub fn key(self) -> &'static str {
        match self {
            Dock::Top => "bars.top",
            Dock::Bottom => "bars.bottom",
            Dock::Left => "bars.left",
            Dock::Right => "bars.right",
            Dock::Floating => "bars.floating",
        }
    }

    /// True for the docks a bar lies along horizontally in.
    pub fn horizontal(self) -> bool {
        matches!(self, Dock::Top | Dock::Bottom)
    }
}

/// The bars that move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bar {
    Toolbox,
    Palette,
    Standard,
    Property,
    Status,
    Text,
    Zoom,
    Transform,
    TimeTracker,
}

impl Bar {
    /// The bars shown at first (Reset Bar Positions shows them again).
    pub const ALL: [Bar; 5] = [
        Bar::Standard,
        Bar::Property,
        Bar::Toolbox,
        Bar::Palette,
        Bar::Status,
    ];

    /// The toolbars that lie in rows along the top or the bottom, in the
    /// order of their first rows.
    pub const ROWS: [Bar; 6] = [
        Bar::Standard,
        Bar::Property,
        Bar::Text,
        Bar::Zoom,
        Bar::Transform,
        Bar::TimeTracker,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Bar::Toolbox => "menu.window.toolbar_toolbox",
            Bar::Palette => "bars.palette",
            Bar::Standard => "menu.window.toolbar_standard",
            Bar::Property => "menu.window.toolbar_property_bar",
            Bar::Status => "menu.window.toolbar_status_bar",
            Bar::Text => "menu.window.toolbar_text",
            Bar::Zoom => "menu.window.toolbar_zoom",
            Bar::Transform => "menu.window.toolbar_transform",
            Bar::TimeTracker => "menu.window.toolbar_time_tracker",
        }
    }

    /// The bar's name in the preferences.
    pub fn id(self) -> &'static str {
        match self {
            Bar::Toolbox => "toolbox",
            Bar::Palette => "palette",
            Bar::Standard => "standard",
            Bar::Property => "property",
            Bar::Status => "status",
            Bar::Text => "text",
            Bar::Zoom => "zoom",
            Bar::Transform => "transform",
            Bar::TimeTracker => "time_tracker",
        }
    }

    /// The toolbar lies in a row when docked along the top or the bottom.
    pub fn in_rows(self) -> bool {
        Bar::ROWS.contains(&self)
    }

    /// The height of the toolbar's band in a row, its line included.
    pub fn row_height(self) -> f32 {
        match self {
            Bar::Property => 53.0,
            _ => 36.0,
        }
    }

    /// The room the toolbar's buttons take across its row: the band less
    /// the margins above and below them.
    fn inner_height(self) -> f32 {
        match self {
            Bar::Property => 37.0,
            _ => 32.0,
        }
    }

    /// Where the bar can go.
    pub fn docks(self) -> &'static [Dock] {
        match self {
            Bar::Toolbox | Bar::Palette => &[
                Dock::Top,
                Dock::Bottom,
                Dock::Left,
                Dock::Right,
                Dock::Floating,
            ],
            Bar::Standard
            | Bar::Property
            | Bar::Text
            | Bar::Zoom
            | Bar::Transform
            | Bar::TimeTracker => &[Dock::Top, Dock::Bottom, Dock::Floating],
            Bar::Status => &[Dock::Top, Dock::Bottom],
        }
    }

    pub fn default_dock(self) -> Dock {
        match self {
            Bar::Toolbox => Dock::Left,
            Bar::Palette => Dock::Bottom,
            Bar::Status => Dock::Bottom,
            Bar::Standard
            | Bar::Property
            | Bar::Text
            | Bar::Zoom
            | Bar::Transform
            | Bar::TimeTracker => Dock::Top,
        }
    }

    pub fn visible(self, app: &App) -> bool {
        match self {
            Bar::Toolbox => app.show_toolbox,
            Bar::Palette => app.settings.palette.show_palette,
            Bar::Standard => app.show_standard_toolbar,
            Bar::Property => app.show_property_bar,
            Bar::Status => app.show_status_bar,
            Bar::Text => app.show_text_toolbar,
            Bar::Zoom => app.show_zoom_toolbar,
            Bar::Transform => app.show_transform_toolbar,
            Bar::TimeTracker => app.settings.time_tracker.show_bar,
        }
    }

    pub fn set_visible(self, app: &mut App, on: bool) {
        match self {
            Bar::Toolbox => app.show_toolbox = on,
            Bar::Palette => app.settings.palette.show_palette = on,
            Bar::Standard => app.show_standard_toolbar = on,
            Bar::Property => app.show_property_bar = on,
            Bar::Status => app.show_status_bar = on,
            Bar::Text => app.show_text_toolbar = on,
            Bar::Zoom => app.show_zoom_toolbar = on,
            Bar::Transform => app.show_transform_toolbar = on,
            Bar::TimeTracker => app.settings.time_tracker.show_bar = on,
        }
    }

    /// The size of the bar floating (points), title strip included.
    pub fn floating_size(self, app: &App) -> Vec2 {
        match self {
            Bar::Toolbox => vec2(crate::ui::toolbox::floating_width(app) + 8.0, 400.0),
            Bar::Palette => vec2(
                460.0,
                crate::ui::palette::thickness(app, crate::ui::palette::Row::Main)
                    + crate::ui::palette::ROW_H * app.extra_palettes.len() as f32
                    + TITLE_H
                    + 4.0,
            ),
            Bar::Standard => vec2(940.0, 36.0 + TITLE_H + 4.0),
            Bar::Property => vec2(1100.0, 53.0 + TITLE_H + 4.0),
            Bar::Status => vec2(900.0, 40.0),
            Bar::Text => vec2(760.0, 36.0 + TITLE_H + 4.0),
            Bar::Zoom => vec2(420.0, 36.0 + TITLE_H + 4.0),
            Bar::Transform => vec2(820.0, 36.0 + TITLE_H + 4.0),
            Bar::TimeTracker => vec2(520.0, 36.0 + TITLE_H + 4.0),
        }
    }
}

/// Where a toolbar docked along the top or the bottom lies: its row
/// (0 the topmost) and its place in the row (0 the leftmost).
pub type RowPlace = [u32; 2];

/// Height of a floating bar's title strip.
pub const TITLE_H: f32 = 18.0;
/// How near an edge a dropped bar docks there.
pub const EDGE_ZONE: f32 = 56.0;

/// Where each bar is, and whether bars are locked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarLayout {
    pub toolbox: Dock,
    pub palette: Dock,
    pub standard: Dock,
    pub property: Dock,
    pub status: Dock,
    /// Top-left corners of the floating bars (screen points).
    pub toolbox_pos: [f32; 2],
    pub palette_pos: [f32; 2],
    pub standard_pos: [f32; 2],
    pub property_pos: [f32; 2],
    pub text: Dock,
    pub zoom: Dock,
    pub transform: Dock,
    pub time_tracker: Dock,
    /// Top-left corners of the other floating toolbars, by [`Bar::id`].
    pub floating_pos: std::collections::BTreeMap<String, [f32; 2]>,
    /// The rows of the toolbars docked along the top or the bottom, by
    /// [`Bar::id`]; a toolbar missing here takes its first row (each its
    /// own, in the order of [`Bar::ROWS`]).
    pub rows: std::collections::BTreeMap<String, RowPlace>,
    pub locked: bool,
    /// The layout's version: preferences written at 1 had the palette at
    /// the right edge by default (see [`BarLayout::migrate`]).
    #[serde(default)]
    pub version: u32,
    /// Whether the docked palette lies beside the drawing (between it and
    /// the docker column or toolbox on that side) rather than at the
    /// window's edge. None: its first place for the side, beside the
    /// drawing at the top and the bottom, at the edge at the left and the
    /// right (as before the choice existed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette_beside: Option<bool>,
}

/// The current layout version.
pub const LAYOUT_VERSION: u32 = 2;

impl Default for BarLayout {
    fn default() -> Self {
        BarLayout {
            toolbox: Dock::Left,
            palette: Dock::Bottom,
            standard: Dock::Top,
            property: Dock::Top,
            status: Dock::Bottom,
            toolbox_pos: [120.0, 160.0],
            palette_pos: [300.0, 600.0],
            standard_pos: [200.0, 140.0],
            property_pos: [200.0, 200.0],
            text: Dock::Top,
            zoom: Dock::Top,
            transform: Dock::Top,
            time_tracker: Dock::Top,
            floating_pos: Default::default(),
            rows: Default::default(),
            locked: false,
            version: LAYOUT_VERSION,
            palette_beside: None,
        }
    }
}

impl BarLayout {
    /// Bring a layout of older preferences up to date. Version 1 stood the
    /// palette at the right edge by default (and moved it there from the
    /// bottom); its first place is the bottom again, so a version 1
    /// palette at the right goes back there. Earlier layouts, and a
    /// palette put anywhere else, stay as they were; whatever the user
    /// chooses from now on is kept.
    pub fn migrate(&mut self) {
        if self.version == 1 && self.palette == Dock::Right {
            self.palette = Dock::Bottom;
        }
        self.version = self.version.max(LAYOUT_VERSION);
    }

    /// Whether the docked palette lies beside the drawing (see
    /// [`BarLayout::palette_beside`]).
    pub fn palette_beside(&self) -> bool {
        self.palette_beside
            .unwrap_or(self.dock(Bar::Palette).horizontal())
    }

    /// The palette's place: its dock and, docked, whether beside the
    /// drawing (false when floating).
    pub fn palette_place(&self) -> (Dock, bool) {
        let dock = self.dock(Bar::Palette);
        (dock, dock != Dock::Floating && self.palette_beside())
    }

    pub fn dock(&self, bar: Bar) -> Dock {
        let d = match bar {
            Bar::Toolbox => self.toolbox,
            Bar::Palette => self.palette,
            Bar::Standard => self.standard,
            Bar::Property => self.property,
            Bar::Status => self.status,
            Bar::Text => self.text,
            Bar::Zoom => self.zoom,
            Bar::Transform => self.transform,
            Bar::TimeTracker => self.time_tracker,
        };
        if bar.docks().contains(&d) {
            d
        } else {
            bar.default_dock()
        }
    }

    /// Put `bar` at `dock`; false (and nothing changes) where it cannot go.
    pub fn set_dock(&mut self, bar: Bar, dock: Dock) -> bool {
        if !bar.docks().contains(&dock) {
            return false;
        }
        match bar {
            Bar::Toolbox => self.toolbox = dock,
            Bar::Palette => self.palette = dock,
            Bar::Standard => self.standard = dock,
            Bar::Property => self.property = dock,
            Bar::Status => self.status = dock,
            Bar::Text => self.text = dock,
            Bar::Zoom => self.zoom = dock,
            Bar::Transform => self.transform = dock,
            Bar::TimeTracker => self.time_tracker = dock,
        }
        true
    }

    pub fn pos(&self, bar: Bar) -> Pos2 {
        let p = match bar {
            Bar::Toolbox => self.toolbox_pos,
            Bar::Palette => self.palette_pos,
            Bar::Standard => self.standard_pos,
            Bar::Property | Bar::Status => self.property_pos,
            _ => self
                .floating_pos
                .get(bar.id())
                .copied()
                .unwrap_or([240.0, 260.0]),
        };
        pos2(p[0], p[1])
    }

    pub fn set_pos(&mut self, bar: Bar, p: Pos2) {
        let v = [p.x, p.y];
        match bar {
            Bar::Toolbox => self.toolbox_pos = v,
            Bar::Palette => self.palette_pos = v,
            Bar::Standard => self.standard_pos = v,
            Bar::Property | Bar::Status => self.property_pos = v,
            _ => {
                self.floating_pos.insert(bar.id().to_string(), v);
            }
        }
    }

    /// The row and place of a toolbar along the top or the bottom.
    pub fn row_place(&self, bar: Bar) -> RowPlace {
        self.rows.get(bar.id()).copied().unwrap_or_else(|| {
            let first = Bar::ROWS.iter().position(|b| *b == bar).unwrap_or(0);
            [first as u32, 0]
        })
    }

    /// The rows of the toolbars docked at `dock`, top to bottom, each
    /// left to right; hidden toolbars keep their places in them.
    pub fn rows_at(&self, dock: Dock) -> Vec<Vec<Bar>> {
        let mut placed: Vec<(RowPlace, usize, Bar)> = Bar::ROWS
            .iter()
            .enumerate()
            .filter(|(_, b)| self.dock(**b) == dock)
            .map(|(i, b)| (self.row_place(*b), i, *b))
            .collect();
        placed.sort_by_key(|(place, first, _)| (*place, *first));
        let mut rows: Vec<Vec<Bar>> = Vec::new();
        let mut last: Option<u32> = None;
        for ([row, _], _, b) in placed {
            match (last, rows.last_mut()) {
                (Some(r), Some(cur)) if r == row => cur.push(b),
                _ => rows.push(vec![b]),
            }
            last = Some(row);
        }
        rows
    }

    /// Keep `rows` (top to bottom, each left to right) as the places of
    /// their toolbars, numbered from 0.
    fn set_rows(&mut self, rows: &[Vec<Bar>]) {
        for (r, row) in rows.iter().enumerate() {
            for (k, b) in row.iter().enumerate() {
                self.rows.insert(b.id().to_string(), [r as u32, k as u32]);
            }
        }
    }

    /// Put the toolbar `bar` at `target`, docking it there; the other
    /// toolbars of that edge keep their order.
    pub fn place_in_row(&mut self, bar: Bar, target: RowTarget) {
        let dock = target.dock();
        if !bar.in_rows() || !self.set_dock(bar, dock) {
            return;
        }
        let mut rows: Vec<Vec<Bar>> = self
            .rows_at(dock)
            .into_iter()
            .map(|r| r.into_iter().filter(|b| *b != bar).collect::<Vec<_>>())
            .collect();
        match target {
            RowTarget::Into { row, before, .. } => match rows.get_mut(row) {
                Some(r) => {
                    let at = before
                        .and_then(|x| r.iter().position(|b| *b == x))
                        .unwrap_or(r.len());
                    r.insert(at, bar);
                }
                None => rows.push(vec![bar]),
            },
            RowTarget::NewRow { at, .. } => {
                let at = at.min(rows.len());
                rows.insert(at, vec![bar]);
            }
        }
        rows.retain(|r| !r.is_empty());
        self.set_rows(&rows);
    }
}

/// Where a toolbar dropped along the top or the bottom goes: into a row
/// (before a toolbar of it, or at its end), or a row of its own at `at`
/// (an index into [`BarLayout::rows_at`], the rows from there on moving
/// one down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowTarget {
    Into {
        dock: Dock,
        row: usize,
        before: Option<Bar>,
    },
    NewRow {
        dock: Dock,
        at: usize,
    },
}

impl RowTarget {
    pub fn dock(self) -> Dock {
        match self {
            RowTarget::Into { dock, .. } | RowTarget::NewRow { dock, .. } => dock,
        }
    }
}

/// A row of toolbars as drawn this frame: its edge, its index in
/// [`BarLayout::rows_at`], its band and each shown toolbar's part of it.
#[derive(Debug, Clone, PartialEq)]
pub struct RowSpot {
    pub dock: Dock,
    pub row: usize,
    pub rect: Rect,
    pub slots: Vec<(Bar, Rect)>,
}

/// How near the line between two rows a dropped toolbar starts a row of
/// its own there.
pub const ROW_EDGE: f32 = 7.0;

/// Where the toolbar `bar` dropped at `p` goes among the rows drawn
/// (`spots`): over a row, into it before the first toolbar whose middle
/// lies right of `p`; near a row's upper or lower line, a new row there.
/// `None` off every row.
pub fn row_target(spots: &[RowSpot], bar: Bar, p: Pos2) -> Option<RowTarget> {
    if !bar.in_rows() {
        return None;
    }
    let spot = spots.iter().find(|s| s.rect.contains(p))?;
    let dock = spot.dock;
    if p.y < spot.rect.top() + ROW_EDGE {
        return Some(RowTarget::NewRow { dock, at: spot.row });
    }
    if p.y > spot.rect.bottom() - ROW_EDGE {
        return Some(RowTarget::NewRow {
            dock,
            at: spot.row + 1,
        });
    }
    let before = spot
        .slots
        .iter()
        .filter(|(b, _)| *b != bar)
        .find(|(_, r)| r.center().x > p.x)
        .map(|(b, _)| *b);
    Some(RowTarget::Into {
        dock,
        row: spot.row,
        before,
    })
}

/// The mark a drop at `t` shows: a bar between toolbars of a row, or a
/// band across the window between rows.
fn row_mark(spots: &[RowSpot], t: RowTarget, bar: Bar) -> Option<Rect> {
    match t {
        RowTarget::Into { dock, row, before } => {
            let spot = spots.iter().find(|s| s.dock == dock && s.row == row)?;
            let x = match before.and_then(|b| spot.slots.iter().find(|(s, _)| *s == b)) {
                Some((_, r)) => r.left(),
                None => spot
                    .slots
                    .iter()
                    .filter(|(b, _)| *b != bar)
                    .map(|(_, r)| r.right())
                    .fold(spot.rect.left(), f32::max),
            };
            Some(Rect::from_min_max(
                pos2(x - 2.0, spot.rect.top()),
                pos2(x + 2.0, spot.rect.bottom()),
            ))
        }
        RowTarget::NewRow { dock, at } => {
            let mut of_dock = spots.iter().filter(|s| s.dock == dock);
            let y = match of_dock.clone().find(|s| s.row == at) {
                Some(s) => s.rect.top(),
                None => of_dock.next_back()?.rect.bottom(),
            };
            let any = spots.iter().find(|s| s.dock == dock)?;
            Some(Rect::from_min_max(
                pos2(any.rect.left(), y - 3.0),
                pos2(any.rect.right(), y + 3.0),
            ))
        }
    }
}

/// A bar being dragged to a new place: which one, and where the pointer
/// holds it from its top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarDrag {
    pub bar: Bar,
    pub grab: Vec2,
    /// The last pointer position seen: a release outside the window drops
    /// the bar there.
    pub last: Option<Pos2>,
}

/// Where `bar` goes when dropped with the pointer at `p` in a window of
/// `screen` (the menu bar's height below its top): the nearest edge
/// within [`EDGE_ZONE`] it can dock at, else floating, else (a bar that
/// cannot float) the nearest edge it can dock at.
pub fn target_dock(bar: Bar, screen: Rect, menu_h: f32, p: Pos2) -> Dock {
    let distances = [
        (Dock::Left, p.x - screen.left()),
        (Dock::Right, screen.right() - p.x),
        (Dock::Top, p.y - (screen.top() + menu_h)),
        (Dock::Bottom, screen.bottom() - p.y),
    ];
    let mut allowed: Vec<(Dock, f32)> = distances
        .iter()
        .copied()
        .filter(|(d, _)| bar.docks().contains(d))
        .collect();
    allowed.sort_by(|a, b| a.1.total_cmp(&b.1));
    match allowed.first() {
        Some(&(d, dist)) if dist <= EDGE_ZONE => d,
        _ if bar.docks().contains(&Dock::Floating) => Dock::Floating,
        Some(&(d, _)) => d,
        None => bar.default_dock(),
    }
}

/// Paint a grip of dots in `r`, along its longer side.
fn paint_grip(ui: &Ui, r: Rect) {
    let p = ui.painter();
    let color = crate::theme::Tokens::text_dim();
    let vertical = r.height() >= r.width();
    let n = ((if vertical { r.height() } else { r.width() } - 6.0) / 4.0)
        .floor()
        .clamp(1.0, 8.0) as i32;
    let c = r.center();
    for i in 0..n {
        let t = (i as f32 - (n - 1) as f32 / 2.0) * 4.0;
        let (a, b) = if vertical {
            (pos2(c.x - 1.5, c.y + t), pos2(c.x + 1.5, c.y + t + 2.0))
        } else {
            (pos2(c.x + t, c.y - 1.5), pos2(c.x + t + 2.0, c.y + 1.5))
        };
        p.rect_filled(Rect::from_center_size(a, vec2(1.5, 1.5)), 0.0, color);
        p.rect_filled(Rect::from_center_size(b, vec2(1.5, 1.5)), 0.0, color);
    }
}

/// The grip at the start of a docked bar: `horizontal` for a bar running
/// left to right (the grip is then a narrow column of dots).
pub fn grip(app: &mut App, ui: &mut Ui, bar: Bar, horizontal: bool) {
    let size = if horizontal {
        vec2(9.0, ui.available_height().max(16.0))
    } else {
        vec2(ui.available_width().max(16.0), 9.0)
    };
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if !app.settings.bars.locked {
        paint_grip(ui, rect.shrink2(vec2(0.0, 3.0)));
    }
    grip_zone(app, ui, bar, rect, 0);
}

/// Make `r` (already painted as a grip) drag `bar` and open its menu;
/// `salt` tells apart several grips of one bar (the palette's rows).
pub fn grip_zone(app: &mut App, ui: &mut Ui, bar: Bar, r: Rect, salt: u8) {
    let id = egui::Id::new(("bar_grip", bar as u8, salt));
    let desc = if app.settings.bars.locked {
        "bars.grip_locked_desc"
    } else {
        "bars.grip_desc"
    };
    let resp = ui
        .interact(r, id, Sense::click_and_drag())
        .rich_tip(crate::ui::tips::pair(bar.key(), desc));
    let resp = if app.settings.bars.locked {
        resp
    } else {
        resp.on_hover_cursor(egui::CursorIcon::Move)
    };
    if resp.drag_started() && !app.settings.bars.locked {
        // The floating bar comes up with its title under the pointer.
        let size = bar.floating_size(app);
        app.bar_drag = Some(BarDrag {
            bar,
            grab: vec2(12.0f32.min(size.x / 2.0), (TITLE_H / 2.0).min(size.y / 2.0)),
            last: resp.interact_pointer_pos(),
        });
    }
    bar_menu(app, &resp, bar);
}

/// A background response for the empty part of a bar: a right click there
/// opens the bar's menu. Call before drawing the bar's controls.
pub fn background(app: &mut App, ui: &mut Ui, bar: Bar) -> egui::Response {
    let id = egui::Id::new(("bar_background", bar as u8));
    let resp = ui.interact(ui.max_rect(), id, Sense::click());
    bar_menu(app, &resp, bar);
    resp
}

/// A right click anywhere in `rect`, the part of the window the bar
/// takes, on its buttons and boxes too, opens the bar's menu, as the
/// toolbox does (#835). Call after drawing the bar's controls, every
/// frame: the menu keeps a popup of its own, shown here for as long as it
/// is open. `opened` says a menu of the bar opened from this click
/// already (its background got it): showing a second menu then, or one
/// popup twice in a frame, would take the opening click for a click
/// outside and close it at once (#994).
fn menu_anywhere(app: &mut App, ui: &mut Ui, bar: Bar, rect: Rect, opened: bool) {
    let id = anywhere_menu_id(bar);
    let hit = !opened
        && ui.input(|i| {
            i.pointer.secondary_clicked()
                && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
        });
    if !hit && !egui::Popup::is_id_open(ui.ctx(), id) {
        return;
    }
    egui::Popup::menu(&ui.response())
        .id(id)
        .open_memory(hit.then_some(egui::SetOpenCommand::Bool(true)))
        .at_pointer_fixed()
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| crate::ui::menus::body(ui, |ui| menu_body(app, ui, bar)));
}

/// The popup of [`menu_anywhere`] for `bar`.
fn anywhere_menu_id(bar: Bar) -> egui::Id {
    egui::Id::new(("bar_anywhere", bar as u8))
}

/// The bar's context menu on `resp`. Each place that opens it (a grip, the
/// bar's background, a floating title) keeps its own popup, so one menu
/// never shows twice in a frame.
pub fn bar_menu(app: &mut App, resp: &egui::Response, bar: Bar) {
    egui::Popup::context_menu(resp)
        .id(resp.id.with("bar_menu"))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| crate::ui::menus::body(ui, |ui| menu_body(app, ui, bar)));
}

fn menu_body(app: &mut App, ui: &mut Ui, bar: Bar) {
    use crate::ui::menus::{check, item, radio, radio_if, sep, sub};
    ui.set_min_width(200.0);
    let current = app.settings.bars.dock(bar);
    let free = !app.settings.bars.locked;
    if bar == Bar::Palette {
        // Every place: each side at the window's edge or beside the
        // drawing, and floating.
        let now = app.settings.bars.palette_place();
        for (place, key) in PALETTE_PLACES {
            if radio_if(ui, key, "", now == place, free) {
                move_palette(app, place.0, place.1, None);
            }
        }
    } else {
        for d in bar.docks() {
            if radio_if(ui, d.key(), "", current == *d, free) {
                move_bar(app, bar, *d, None);
            }
        }
    }
    sep(ui);
    // Every bar, as Window > Toolbars lists them.
    crate::ui::menus::toolbars_list(app, ui);
    sep(ui);
    sub(ui, "bars.customize", |ui| {
        sub(ui, "bars.default_button_size", |ui| {
            for size in crate::settings::ButtonSize::ALL {
                if radio(ui, size.key(), "", app.settings.button_size == size) {
                    app.settings.button_size = size;
                }
            }
        });
        sub(ui, "bars.toolbox_toolbar", |ui| {
            if item(ui, "bars.toolbox_reset", "", true) {
                crate::ui::toolbox::reset_toolbox(app);
            }
            sep(ui);
            sub(ui, "bars.button_size", |ui| {
                let own = app.settings.toolbox_button_size;
                if radio(ui, "bars.size_default", "", own.is_none()) {
                    app.settings.toolbox_button_size = None;
                }
                for size in crate::settings::ButtonSize::ALL {
                    if radio(ui, size.key(), "", own == Some(size)) {
                        app.settings.toolbox_button_size = Some(size);
                    }
                }
            });
            sep(ui);
            for c in crate::settings::Captions::ALL {
                if radio(ui, c.key(), "", app.settings.toolbox_captions == c) {
                    app.settings.toolbox_captions = c;
                }
            }
        });
        sub(ui, "menu.window.workspace", |ui| {
            if item(ui, "menu.window.workspace_export", "", true) {
                app.export_workspace_dialog(crate::workspace_file::WorkspaceParts::ALL);
            }
            if item(ui, "menu.window.workspace_import", "", true) {
                app.import_workspace_dialog(crate::workspace_file::WorkspaceParts::ALL, true);
            }
        });
        sep(ui);
        // The Command Bars options: every bar, its lock, the toolbox.
        if item(ui, "bars.customize_options", "", true) {
            app.options_page = crate::ui::options::OptionsPage::CommandBars;
            app.dialog = crate::ui::dialogs::Dialog::Options;
        }
    });
    sep(ui);
    let locked = app.settings.bars.locked;
    if check(ui, "bars.lock", "", locked) {
        app.settings.bars.locked = !locked;
    }
    if item(ui, "bars.reset", "", true) {
        reset(app);
    }
}

/// Every bar back at its first place and shown; Lock Toolbars stays as it
/// was.
pub fn reset(app: &mut App) {
    let locked = app.settings.bars.locked;
    app.settings.bars = BarLayout {
        locked,
        ..BarLayout::default()
    };
    for b in Bar::ALL {
        b.set_visible(app, true);
    }
    crate::ui::bar_items::reset(app);
}

/// Where the window drawing keeps the room the bars and dockers leave
/// (the drawing and its tabs) for the palette's drop bands.
pub const DRAWING_ROOM: &str = "bars_drawing_room";

/// The palette's places in its menu, with their names: each side beside
/// the drawing and at the window's edge (the plain names keep the places
/// they had before the choice existed), then floating.
pub const PALETTE_PLACES: [((Dock, bool), &str); 9] = [
    ((Dock::Top, true), "bars.top"),
    ((Dock::Top, false), "bars.palette_top_edge"),
    ((Dock::Bottom, true), "bars.bottom"),
    ((Dock::Bottom, false), "bars.palette_bottom_edge"),
    ((Dock::Left, false), "bars.left"),
    ((Dock::Left, true), "bars.palette_left_beside"),
    ((Dock::Right, false), "bars.right"),
    ((Dock::Right, true), "bars.palette_right_beside"),
    ((Dock::Floating, false), "bars.floating"),
];

/// Put the palette at `dock`, beside the drawing or at the window's edge
/// (floating at `at` when given). The place is kept in the preferences.
pub fn move_palette(app: &mut App, dock: Dock, beside: bool, at: Option<Pos2>) {
    move_bar(app, Bar::Palette, dock, at);
    if dock != Dock::Floating && app.settings.bars.dock(Bar::Palette) == dock {
        app.settings.bars.palette_beside = Some(beside);
    }
}

/// Put `bar` at `dock` (floating at `at` when given). A toolbar coming to
/// the top or the bottom from elsewhere takes a row of its own at the
/// window's edge.
pub fn move_bar(app: &mut App, bar: Bar, dock: Dock, at: Option<Pos2>) {
    let before = app.settings.bars.dock(bar);
    if bar.in_rows() && dock.horizontal() && before != dock {
        let at = match dock {
            Dock::Top => 0,
            _ => app.settings.bars.rows_at(dock).len(),
        };
        app.settings
            .bars
            .place_in_row(bar, RowTarget::NewRow { dock, at });
        bar.set_visible(app, true);
        return;
    }
    if app.settings.bars.set_dock(bar, dock) {
        if let (Dock::Floating, Some(p)) = (dock, at) {
            app.settings.bars.set_pos(bar, p);
        }
        bar.set_visible(app, true);
    }
}

/// While a bar is dragged: the place it would go, highlighted; on release
/// it goes there.
pub fn drag_overlay(app: &mut App, ctx: &egui::Context, menu_h: f32) {
    let Some(mut drag) = app.bar_drag else {
        return;
    };
    let screen = ctx.content_rect();
    let (pointer, down) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down()));
    let Some(p) = pointer.or(drag.last) else {
        if !down {
            app.bar_drag = None;
        }
        return;
    };
    drag.last = Some(p);
    app.bar_drag = Some(drag);
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("bar_drag_overlay"),
    ));
    let fill = Color32::from_rgba_unmultiplied(0x1E, 0x78, 0xD7, 46);
    let edge = Stroke::new(1.5, Color32::from_rgb(0x1E, 0x78, 0xD7));
    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    // Over a row of toolbars: into it, or a row of its own between two.
    if let Some(t) = row_target(&app.bar_rows, drag.bar, p) {
        if let Some(mark) = row_mark(&app.bar_rows, t, drag.bar) {
            painter.rect(
                mark,
                1.0,
                edge.color,
                Stroke::NONE,
                egui::StrokeKind::Inside,
            );
        }
        if !down {
            app.bar_drag = None;
            app.settings.bars.place_in_row(drag.bar, t);
            drag.bar.set_visible(app, true);
        }
        return;
    }
    if drag.bar == Bar::Palette {
        // The palette also docks beside the drawing: between it and the
        // docker column, the toolbox or the toolbars on that side.
        let drawing = ctx
            .data(|d| d.get_temp::<Rect>(egui::Id::new(DRAWING_ROOM)))
            .unwrap_or(Rect::NOTHING);
        let (dock, beside) = palette_target(screen, menu_h, drawing, p);
        let zone = if beside {
            beside_zone(app, dock, drawing)
        } else {
            drop_zone(app, drag, dock, screen, menu_h, p)
        };
        painter.rect(zone, 2.0, fill, edge, egui::StrokeKind::Inside);
        if !down {
            app.bar_drag = None;
            move_palette(app, dock, beside, Some(p - drag.grab));
        }
        return;
    }
    let dock = target_dock(drag.bar, screen, menu_h, p);
    let zone = drop_zone(app, drag, dock, screen, menu_h, p);
    painter.rect(zone, 2.0, fill, edge, egui::StrokeKind::Inside);
    if !down {
        app.bar_drag = None;
        move_bar(app, drag.bar, dock, Some(p - drag.grab));
    }
}

/// Where the palette goes when dropped with the pointer at `p`: the
/// nearest of the window's edges and the edges of `drawing` (the room the
/// bars and dockers leave) within [`EDGE_ZONE`], with whether that is
/// beside the drawing; else floating. Where an edge of the drawing lies
/// on the window's edge, the window's edge wins.
pub fn palette_target(screen: Rect, menu_h: f32, drawing: Rect, p: Pos2) -> (Dock, bool) {
    let top = screen.top() + menu_h;
    let along_y = p.y >= drawing.top() - EDGE_ZONE && p.y <= drawing.bottom() + EDGE_ZONE;
    let along_x = p.x >= drawing.left() - EDGE_ZONE && p.x <= drawing.right() + EDGE_ZONE;
    let mut spots = vec![
        (Dock::Left, false, p.x - screen.left()),
        (Dock::Right, false, screen.right() - p.x),
        (Dock::Top, false, p.y - top),
        (Dock::Bottom, false, screen.bottom() - p.y),
    ];
    if drawing.is_positive() {
        let beside = [
            (
                Dock::Left,
                drawing.left(),
                screen.left(),
                along_y,
                (p.x - drawing.left()).abs(),
            ),
            (
                Dock::Right,
                drawing.right(),
                screen.right(),
                along_y,
                (drawing.right() - p.x).abs(),
            ),
            (
                Dock::Top,
                drawing.top(),
                top,
                along_x,
                (p.y - drawing.top()).abs(),
            ),
            (
                Dock::Bottom,
                drawing.bottom(),
                screen.bottom(),
                along_x,
                (drawing.bottom() - p.y).abs(),
            ),
        ];
        for (dock, at, window_edge, along, dist) in beside {
            if along && (at - window_edge).abs() > 1.0 {
                spots.push((dock, true, dist));
            }
        }
    }
    spots.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.1.cmp(&b.1)));
    match spots.first() {
        Some(&(dock, beside, dist)) if dist <= EDGE_ZONE => (dock, beside),
        _ => (Dock::Floating, false),
    }
}

/// The band the palette would take beside the drawing at `dock`.
fn beside_zone(app: &App, dock: Dock, drawing: Rect) -> Rect {
    let thick = crate::ui::palette::thickness(app, crate::ui::palette::Row::Main);
    match dock {
        Dock::Left => {
            Rect::from_min_max(drawing.min, pos2(drawing.left() + thick, drawing.bottom()))
        }
        Dock::Right => {
            Rect::from_min_max(pos2(drawing.right() - thick, drawing.top()), drawing.max)
        }
        Dock::Top => Rect::from_min_max(drawing.min, pos2(drawing.right(), drawing.top() + thick)),
        Dock::Bottom => {
            Rect::from_min_max(pos2(drawing.left(), drawing.bottom() - thick), drawing.max)
        }
        Dock::Floating => drawing,
    }
}

/// The band (or, floating, the window) a dragged bar would take at `dock`.
fn drop_zone(app: &App, drag: BarDrag, dock: Dock, screen: Rect, menu_h: f32, p: Pos2) -> Rect {
    let thick = match drag.bar {
        Bar::Toolbox => crate::ui::toolbox::column_width(app),
        Bar::Palette => crate::ui::palette::thickness(app, crate::ui::palette::Row::Main),
        Bar::Status => crate::ui::status::BAR_H,
        b => b.row_height(),
    };
    // Bars dock inside the status bar, which keeps the outer band.
    let status = app.show_status_bar && drag.bar != Bar::Status;
    let status_dock = app.settings.bars.dock(Bar::Status);
    let top = screen.top()
        + menu_h
        + if status && status_dock == Dock::Top {
            crate::ui::status::BAR_H
        } else {
            0.0
        };
    let bottom = screen.bottom()
        - if status && status_dock == Dock::Bottom {
            crate::ui::status::BAR_H
        } else {
            0.0
        };
    match dock {
        Dock::Left => Rect::from_min_max(
            pos2(screen.left(), top),
            pos2(screen.left() + thick, bottom),
        ),
        Dock::Right => Rect::from_min_max(
            pos2(screen.right() - thick, top),
            pos2(screen.right(), bottom),
        ),
        Dock::Top => {
            Rect::from_min_max(pos2(screen.left(), top), pos2(screen.right(), top + thick))
        }
        Dock::Bottom => Rect::from_min_max(
            pos2(screen.left(), bottom - thick),
            pos2(screen.right(), bottom),
        ),
        Dock::Floating => Rect::from_min_size(p - drag.grab, drag.bar.floating_size(app)),
    }
}

/// The toolbar's buttons and boxes, in a row that slides when it is
/// longer than the room it gets; the length they take.
fn toolbar_body(app: &mut App, ui: &mut Ui, bar: Bar) -> f32 {
    use crate::ui::{overflow, time_tracker, toolbar};
    match bar {
        Bar::Standard => overflow::row(ui, "standard", |ui| toolbar::standard_toolbar(app, ui)),
        Bar::Property => overflow::row(ui, "property", |ui| toolbar::property_bar(app, ui)),
        Bar::Text => overflow::row(ui, "text", |ui| toolbar::text_toolbar(app, ui)),
        Bar::Zoom => overflow::row(ui, "zoom", |ui| toolbar::zoom_toolbar(app, ui)),
        Bar::Transform => overflow::row(ui, "transform", |ui| toolbar::transform_toolbar(app, ui)),
        Bar::TimeTracker => overflow::row(ui, "time_tracker", |ui| time_tracker::toolbar(app, ui)),
        Bar::Toolbox | Bar::Palette | Bar::Status => 0.0,
    }
}

/// The rows of toolbars docked at `dock` (the top or the bottom), each a
/// band as high as its highest toolbar, the toolbars side by side in it.
/// Each toolbar gets the length its buttons take while they all fit; when
/// they do not, the room is shared by those lengths and each slides.
/// Where the rows and toolbars were drawn goes to `app.bar_rows` for the
/// drops of a dragged toolbar.
pub fn docked_rows(app: &mut App, ui: &mut Ui, dock: Dock) {
    let rows = app.settings.bars.rows_at(dock);
    let mut order: Vec<(usize, Vec<Bar>)> = rows
        .into_iter()
        .enumerate()
        .map(|(i, r)| (i, r.into_iter().filter(|b| b.visible(app)).collect()))
        .filter(|(_, r): &(usize, Vec<Bar>)| !r.is_empty())
        .collect();
    // Bottom panels stack upwards: the lowest row first.
    if dock == Dock::Bottom {
        order.reverse();
    }
    for (index, bars) in order {
        let height = bars.iter().map(|b| b.row_height()).fold(0.0, f32::max);
        let panel = if dock == Dock::Top {
            egui::Panel::top(egui::Id::new(("bar_row_top", index)))
        } else {
            egui::Panel::bottom(egui::Id::new(("bar_row_bottom", index)))
        };
        panel
            .exact_size(height)
            .resizable(false)
            .show_separator_line(dock == Dock::Top)
            .frame(egui::Frame::new().fill(Tokens::panel()))
            .show(ui, |ui| {
                let row = ui.max_rect();
                let widths: Vec<f32> = bars
                    .iter()
                    .map(|b| {
                        ui.data(|d| d.get_temp::<f32>(egui::Id::new(("bar_natural", *b as u8))))
                            .unwrap_or(row.width())
                    })
                    .collect();
                let slots = share_row(row.width(), &widths);
                let mut x = row.left();
                let mut spots = Vec::new();
                for (bar, w) in bars.iter().zip(slots) {
                    let slot = Rect::from_min_max(pos2(x, row.top()), pos2(x + w, row.bottom()));
                    x += w;
                    spots.push((*bar, slot));
                    let inner = bar.inner_height().min(slot.height());
                    let body = Rect::from_center_size(slot.center(), vec2(slot.width(), inner));
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(slot)
                            .layout(egui::Layout::left_to_right(egui::Align::Center))
                            .id_salt(("bar_slot", *bar as u8)),
                    );
                    child.set_clip_rect(slot.intersect(ui.clip_rect()));
                    let back = background(app, &mut child, *bar);
                    let mut body_ui = child.new_child(
                        egui::UiBuilder::new()
                            .max_rect(
                                body.with_min_x(body.min.x + 2.0)
                                    .with_max_x(body.max.x - 4.0),
                            )
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    grip(app, &mut body_ui, *bar, true);
                    let gap = body_ui.spacing().item_spacing.x;
                    let len = toolbar_body(app, &mut body_ui, *bar);
                    // The length the toolbar takes whole: its margins, grip
                    // and buttons.
                    let natural = (2.0 + 9.0 + gap + len + 4.0).ceil();
                    let key = egui::Id::new(("bar_natural", *bar as u8));
                    if ui.data(|d| d.get_temp::<f32>(key)) != Some(natural) {
                        ui.data_mut(|d| d.insert_temp(key, natural));
                        ui.ctx().request_repaint();
                    }
                    menu_anywhere(app, &mut child, *bar, slot, back.secondary_clicked());
                }
                app.bar_rows.push(RowSpot {
                    dock,
                    row: index,
                    rect: row,
                    slots: spots,
                });
            });
    }
}

/// The lengths of toolbars side by side in a row `room` long, wanting
/// `want` each, left to right: each the length it wants while the room
/// holds it and leaves [`MIN_SLOT`] for each toolbar after it, the last
/// one taking the rest. A toolbar cut short slides (see `overflow.rs`).
pub fn share_row(room: f32, want: &[f32]) -> Vec<f32> {
    let n = want.len();
    let floor = if n == 0 {
        0.0
    } else {
        MIN_SLOT.min(room / n as f32)
    };
    let mut left = room.max(0.0);
    let mut out = Vec::with_capacity(n);
    for (i, w) in want.iter().enumerate() {
        let after = (n - 1 - i) as f32 * floor;
        let w = if i + 1 == n {
            left
        } else {
            let w = if w.is_finite() { w.max(1.0) } else { left };
            w.min(left - after).max(floor.min(left))
        };
        out.push(w);
        left = (left - w).max(0.0);
    }
    out
}

/// The shortest part of a row a toolbar gets while there is room.
pub const MIN_SLOT: f32 = 90.0;

/// The floating bars, each in a small window with its name, at its place.
pub fn floating_bars(app: &mut App, ctx: &egui::Context) {
    for bar in [
        Bar::Standard,
        Bar::Property,
        Bar::Text,
        Bar::Zoom,
        Bar::Transform,
        Bar::TimeTracker,
        Bar::Toolbox,
        Bar::Palette,
    ] {
        if !bar.visible(app) || app.settings.bars.dock(bar) != Dock::Floating {
            continue;
        }
        let screen = ctx.content_rect();
        let size = bar.floating_size(app);
        let mut pos = app.settings.bars.pos(bar);
        // Keep the title on the screen.
        pos.x = pos
            .x
            .clamp(screen.left(), (screen.right() - 60.0).max(screen.left()));
        pos.y = pos
            .y
            .clamp(screen.top(), (screen.bottom() - TITLE_H).max(screen.top()));
        egui::Area::new(egui::Id::new(("floating_bar", bar as u8)))
            .fixed_pos(pos)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Tokens::panel())
                    .stroke(Stroke::new(1.0, crate::theme::Tokens::window_border()))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 3],
                        blur: 10,
                        spread: 0,
                        color: Color32::from_black_alpha(50),
                    })
                    .show(ui, |ui| {
                        ui.set_width(size.x);
                        title_strip(app, ui, bar, size.x);
                        let body = ui.scope(|ui| floating_body(app, ui, bar, size)).response;
                        // The toolbox's buttons and the palette's colors
                        // open menus of their own.
                        if !matches!(bar, Bar::Toolbox | Bar::Palette) {
                            menu_anywhere(app, ui, bar, body.rect, false);
                        }
                    });
            });
    }
}

/// A floating bar's title: its name and a close button. Dragging it moves
/// the bar (docking it near an edge); a double click docks it back.
fn title_strip(app: &mut App, ui: &mut Ui, bar: Bar, width: f32) {
    let (rect, resp) = ui.allocate_exact_size(vec2(width, TITLE_H), Sense::click_and_drag());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, crate::theme::Tokens::tab_strip());
    p.text(
        pos2(rect.left() + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        tr(bar.key()),
        egui::FontId::proportional(11.0),
        Tokens::text(),
    );
    let close = Rect::from_center_size(
        pos2(rect.right() - TITLE_H / 2.0, rect.center().y),
        vec2(TITLE_H, TITLE_H),
    );
    let close_resp = ui.interact(
        close,
        egui::Id::new(("bar_close", bar as u8)),
        Sense::click(),
    );
    let c = close.center();
    let s = Stroke::new(
        1.2,
        if close_resp.hovered() {
            Color32::from_rgb(0xE8, 0x11, 0x23)
        } else {
            Tokens::text_dim()
        },
    );
    p.line_segment([c + vec2(-3.5, -3.5), c + vec2(3.5, 3.5)], s);
    p.line_segment([c + vec2(-3.5, 3.5), c + vec2(3.5, -3.5)], s);
    if close_resp.clicked() {
        bar.set_visible(app, false);
        return;
    }
    if resp.double_clicked() {
        move_bar(app, bar, bar.default_dock(), None);
        return;
    }
    if resp.drag_started() && !app.settings.bars.locked {
        let grab = resp
            .interact_pointer_pos()
            .map(|q| q - rect.min)
            .unwrap_or(vec2(12.0, TITLE_H / 2.0));
        app.bar_drag = Some(BarDrag {
            bar,
            grab,
            last: resp.interact_pointer_pos(),
        });
    }
    bar_menu(app, &resp, bar);
}

/// The bar's controls inside its floating window.
fn floating_body(app: &mut App, ui: &mut Ui, bar: Bar, size: Vec2) {
    match bar {
        Bar::Toolbox => {
            // As tall as its buttons: the size above is for the preview.
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(4, 3))
                .show(ui, |ui| {
                    crate::ui::toolbox::toolbox(app, ui, Dock::Floating)
                });
        }
        Bar::Palette => {
            ui.set_width(size.x);
            crate::ui::palette::palette_row(app, ui);
            // The further palettes shown, under it (#1164).
            for which in crate::ui::palette::extra_rows(app) {
                crate::ui::palette::extra_palette_row(app, ui, which);
            }
        }
        Bar::Standard => {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(4, 2))
                .show(ui, |ui| {
                    ui.set_height(32.0);
                    crate::ui::toolbar::standard_toolbar(app, ui);
                });
        }
        Bar::Property => {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(4, 8))
                .show(ui, |ui| {
                    ui.set_height(37.0);
                    crate::ui::toolbar::property_bar(app, ui);
                });
        }
        Bar::Text | Bar::Zoom | Bar::Transform | Bar::TimeTracker => {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(4, 2))
                .show(ui, |ui| {
                    ui.set_height(32.0);
                    ui.set_width(size.x - 8.0);
                    toolbar_body(app, ui, bar);
                });
        }
        Bar::Status => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1000.0))
    }

    #[test]
    fn the_palette_docks_beside_the_drawing_or_at_the_window_edge() {
        let s = screen();
        let drawing = Rect::from_min_max(pos2(60.0, 120.0), pos2(1200.0, 900.0));
        let at = |x: f32, y: f32| palette_target(s, 24.0, drawing, pos2(x, y));
        assert_eq!(at(1190.0, 500.0), (Dock::Right, true));
        assert_eq!(at(1230.0, 500.0), (Dock::Right, true));
        assert_eq!(at(1590.0, 500.0), (Dock::Right, false));
        assert_eq!(at(75.0, 500.0), (Dock::Left, true));
        assert_eq!(at(10.0, 500.0), (Dock::Left, false));
        assert_eq!(at(600.0, 890.0), (Dock::Bottom, true));
        assert_eq!(at(600.0, 995.0), (Dock::Bottom, false));
        assert_eq!(at(600.0, 125.0), (Dock::Top, true));
        assert_eq!(at(600.0, 30.0), (Dock::Top, false));
        assert_eq!(at(600.0, 500.0), (Dock::Floating, false));
        // An edge of the drawing on the window's edge is the window's.
        let wide = Rect::from_min_max(pos2(0.0, 120.0), pos2(1600.0, 900.0));
        assert_eq!(
            palette_target(s, 24.0, wide, pos2(1590.0, 500.0)),
            (Dock::Right, false)
        );
        // The place is kept in the preferences; older preferences have
        // none and keep the places they had.
        let mut l = BarLayout::default();
        assert_eq!(l.palette_place(), (Dock::Bottom, true));
        l.set_dock(Bar::Palette, Dock::Right);
        assert_eq!(l.palette_place(), (Dock::Right, false));
        l.palette_beside = Some(true);
        let back: BarLayout =
            serde_json::from_str(&serde_json::to_string(&l).expect("json")).expect("layout");
        assert_eq!(back.palette_place(), (Dock::Right, true));
        let old: BarLayout = serde_json::from_str(r#"{"palette":"bottom"}"#).expect("old");
        assert_eq!(old.palette_place(), (Dock::Bottom, true));
    }

    /// The palette dragged near the drawing's right edge shows a band there
    /// and, dropped, lies between the drawing and the docker column; at the
    /// window's right edge it lies outside the dockers, as before.
    #[test]
    fn the_palette_dropped_beside_the_drawing_lies_between_it_and_the_dockers() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.show_dockers = true;
        let run = |app: &mut App, events: Vec<egui::Event>| -> Vec<egui::Shape> {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1000.0))),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
            out.textures_delta.clear();
            out.shapes.into_iter().map(|c| c.shape).collect()
        };
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let canvas = app.canvas_rect;
        assert!(canvas.is_positive());
        let palette_x = || {
            ctx.data(|d| d.get_temp::<Rect>(egui::Id::new(("palette_expand_rect", 0u8))))
                .expect("the palette")
                .center()
                .x
        };
        app.bar_drag = Some(BarDrag {
            bar: Bar::Palette,
            grab: vec2(5.0, 5.0),
            last: None,
        });
        let at = pos2(canvas.right() - 10.0, canvas.center().y);
        let press = egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        let shapes = run(&mut app, vec![egui::Event::PointerMoved(at), press]);
        let blue = Color32::from_rgb(0x1E, 0x78, 0xD7);
        let band = shapes.iter().find_map(|s| match s {
            egui::Shape::Rect(r) if r.stroke.color == blue => Some(r.rect),
            _ => None,
        });
        let band = band.expect("the drop band");
        assert!(
            band.right() < 1500.0 && (band.right() - at.x).abs() < 40.0,
            "the band lies beside the drawing: {band:?} {at:?}"
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
        assert!(app.bar_drag.is_none());
        assert_eq!(app.settings.bars.palette_place(), (Dock::Right, true));
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let beside = palette_x();
        assert!(
            beside > app.canvas_rect.right() && beside < 1600.0 - 300.0,
            "between the drawing and the dockers: {beside} {:?}",
            app.canvas_rect
        );
        // From the menu, at the window's right edge: outside the dockers.
        move_palette(&mut app, Dock::Right, false, None);
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        assert!(palette_x() > 1550.0, "at the edge: {}", palette_x());
        assert_eq!(app.settings.bars.palette_place(), (Dock::Right, false));
    }

    #[test]
    fn a_dropped_bar_docks_at_the_nearest_edge_or_floats() {
        let s = screen();
        assert_eq!(
            target_dock(Bar::Palette, s, 24.0, pos2(1590.0, 500.0)),
            Dock::Right
        );
        assert_eq!(
            target_dock(Bar::Palette, s, 24.0, pos2(10.0, 500.0)),
            Dock::Left
        );
        assert_eq!(
            target_dock(Bar::Palette, s, 24.0, pos2(800.0, 40.0)),
            Dock::Top
        );
        assert_eq!(
            target_dock(Bar::Palette, s, 24.0, pos2(800.0, 990.0)),
            Dock::Bottom
        );
        assert_eq!(
            target_dock(Bar::Palette, s, 24.0, pos2(800.0, 500.0)),
            Dock::Floating
        );
        // The standard toolbar cannot dock on a side: it floats there.
        assert_eq!(
            target_dock(Bar::Standard, s, 24.0, pos2(5.0, 500.0)),
            Dock::Floating
        );
        // The status bar cannot float: the nearest edge it can take.
        assert_eq!(
            target_dock(Bar::Status, s, 24.0, pos2(800.0, 300.0)),
            Dock::Top
        );
        assert_eq!(
            target_dock(Bar::Status, s, 24.0, pos2(800.0, 900.0)),
            Dock::Bottom
        );
    }

    #[test]
    fn bars_only_go_where_they_can() {
        let mut l = BarLayout::default();
        assert_eq!(l.dock(Bar::Toolbox), Dock::Left);
        assert!(l.set_dock(Bar::Toolbox, Dock::Right));
        assert_eq!(l.dock(Bar::Toolbox), Dock::Right);
        assert!(!l.set_dock(Bar::Status, Dock::Floating));
        assert_eq!(l.dock(Bar::Status), Dock::Bottom);
        // A stored dock a bar cannot take reads as its default.
        l.standard = Dock::Left;
        assert_eq!(l.dock(Bar::Standard), Dock::Top);
        l.set_pos(Bar::Palette, pos2(10.0, 20.0));
        assert_eq!(l.pos(Bar::Palette), pos2(10.0, 20.0));
    }

    #[test]
    fn the_drop_zone_leaves_the_status_bar_outside() {
        let mut app = App::headless();
        app.show_status_bar = true;
        let s = screen();
        let drag = BarDrag {
            bar: Bar::Property,
            grab: vec2(12.0, 9.0),
            last: None,
        };
        let status = crate::ui::status::BAR_H;
        let z = drop_zone(&app, drag, Dock::Bottom, s, 24.0, pos2(800.0, 990.0));
        assert_eq!(z.bottom(), 1000.0 - status);
        assert_eq!(z.height(), 53.0);
        let z = drop_zone(&app, drag, Dock::Top, s, 24.0, pos2(800.0, 30.0));
        assert_eq!(z.top(), 24.0);
        // The status bar itself takes the outer band.
        let drag = BarDrag {
            bar: Bar::Status,
            ..drag
        };
        let z = drop_zone(&app, drag, Dock::Bottom, s, 24.0, pos2(800.0, 990.0));
        assert_eq!(z.bottom(), 1000.0);
        // Floating: the bar's window, held where it was grabbed.
        let drag = BarDrag {
            bar: Bar::Palette,
            ..drag
        };
        let z = drop_zone(&app, drag, Dock::Floating, s, 24.0, pos2(500.0, 500.0));
        assert_eq!(z.min, pos2(488.0, 491.0));
        assert_eq!(z.size(), Bar::Palette.floating_size(&app));
    }

    #[test]
    fn reset_brings_every_bar_back_and_keeps_the_lock() {
        let mut app = App::headless();
        move_bar(
            &mut app,
            Bar::Palette,
            Dock::Floating,
            Some(pos2(300.0, 300.0)),
        );
        move_bar(&mut app, Bar::Toolbox, Dock::Right, None);
        Bar::Standard.set_visible(&mut app, false);
        app.settings.bars.locked = true;
        reset(&mut app);
        assert_eq!(app.settings.bars.dock(Bar::Palette), Dock::Bottom);
        assert_eq!(app.settings.bars.dock(Bar::Toolbox), Dock::Left);
        assert!(Bar::ALL.iter().all(|b| b.visible(&app)));
        assert!(app.settings.bars.locked);
        // A move puts the bar there and shows it.
        Bar::Property.set_visible(&mut app, false);
        move_bar(&mut app, Bar::Property, Dock::Bottom, None);
        assert!(Bar::Property.visible(&app));
        assert_eq!(app.settings.bars.dock(Bar::Property), Dock::Bottom);
    }

    #[test]
    fn the_layout_survives_the_preferences() {
        let mut l = BarLayout::default();
        l.palette = Dock::Right;
        l.locked = true;
        let json = serde_json::to_string(&l).expect("json");
        let back: BarLayout = serde_json::from_str(&json).expect("back");
        assert_eq!(back, l);
        // Older preferences without bars read as the default layout, once
        // brought up to date.
        let mut empty: BarLayout = serde_json::from_str("{}").expect("empty");
        empty.migrate();
        assert_eq!(empty, BarLayout::default());
    }

    /// The toolbars along the top start each in a row of its own; one
    /// dropped into a row joins it where it was dropped, one dropped on
    /// the line between rows starts a row there, and the places survive
    /// the preferences (#837, #855).
    #[test]
    fn toolbars_share_rows_and_keep_their_places() {
        use Bar::*;
        let mut l = BarLayout::default();
        let first = vec![
            vec![Standard],
            vec![Property],
            vec![Text],
            vec![Zoom],
            vec![Transform],
            vec![TimeTracker],
        ];
        assert_eq!(l.rows_at(Dock::Top), first);
        assert!(l.rows_at(Dock::Bottom).is_empty());
        // The property bar beside the standard toolbar.
        l.place_in_row(
            Property,
            RowTarget::Into {
                dock: Dock::Top,
                row: 0,
                before: None,
            },
        );
        assert_eq!(l.rows_at(Dock::Top)[0], vec![Standard, Property]);
        assert_eq!(l.rows_at(Dock::Top).len(), 5);
        // The Zoom toolbar in front of the standard toolbar.
        l.place_in_row(
            Zoom,
            RowTarget::Into {
                dock: Dock::Top,
                row: 0,
                before: Some(Standard),
            },
        );
        assert_eq!(l.rows_at(Dock::Top)[0], vec![Zoom, Standard, Property]);
        // The property bar back on a row of its own under the first.
        l.place_in_row(
            Property,
            RowTarget::NewRow {
                dock: Dock::Top,
                at: 1,
            },
        );
        assert_eq!(
            l.rows_at(Dock::Top)[..2],
            [vec![Zoom, Standard], vec![Property]]
        );
        // Kept in the preferences.
        let json = serde_json::to_string(&l).expect("json");
        let back: BarLayout = serde_json::from_str(&json).expect("back");
        assert_eq!(back.rows_at(Dock::Top), l.rows_at(Dock::Top));
        // A toolbar sent to the bottom takes a row at the window's edge,
        // and back at the top the first row.
        let mut app = App::headless();
        app.settings.bars = l;
        move_bar(&mut app, Text, Dock::Bottom, None);
        assert_eq!(app.settings.bars.rows_at(Dock::Bottom), vec![vec![Text]]);
        move_bar(&mut app, Transform, Dock::Bottom, None);
        assert_eq!(
            app.settings.bars.rows_at(Dock::Bottom),
            vec![vec![Text], vec![Transform]]
        );
        move_bar(&mut app, Text, Dock::Top, None);
        assert_eq!(app.settings.bars.rows_at(Dock::Top)[0], vec![Text]);
        assert!(Text.visible(&app));
        // The toolbox and the palette have no rows.
        app.settings.bars.place_in_row(
            Toolbox,
            RowTarget::Into {
                dock: Dock::Top,
                row: 0,
                before: None,
            },
        );
        assert_eq!(app.settings.bars.dock(Toolbox), Dock::Left);
        // Reset puts every toolbar back in its first row.
        reset(&mut app);
        assert_eq!(app.settings.bars.rows_at(Dock::Top), first);
    }

    /// Where a dragged toolbar goes over the rows drawn: into a row by
    /// the middles of its toolbars, near a row's line a new row (#855).
    #[test]
    fn a_toolbar_dropped_on_a_row_joins_it_or_starts_one() {
        let spot = |row: usize, top: f32, slots: Vec<(Bar, f32, f32)>| RowSpot {
            dock: Dock::Top,
            row,
            rect: Rect::from_min_max(pos2(0.0, top), pos2(1600.0, top + 36.0)),
            slots: slots
                .into_iter()
                .map(|(b, a, z)| (b, Rect::from_min_max(pos2(a, top), pos2(z, top + 36.0))))
                .collect(),
        };
        let spots = vec![
            spot(
                0,
                24.0,
                vec![(Bar::Standard, 0.0, 900.0), (Bar::Zoom, 900.0, 1600.0)],
            ),
            spot(1, 60.0, vec![(Bar::Property, 0.0, 1600.0)]),
        ];
        let into = |row, before| RowTarget::Into {
            dock: Dock::Top,
            row,
            before,
        };
        assert_eq!(
            row_target(&spots, Bar::Property, pos2(300.0, 40.0)),
            Some(into(0, Some(Bar::Standard)))
        );
        assert_eq!(
            row_target(&spots, Bar::Property, pos2(1000.0, 40.0)),
            Some(into(0, Some(Bar::Zoom)))
        );
        assert_eq!(
            row_target(&spots, Bar::Property, pos2(1300.0, 40.0)),
            Some(into(0, None))
        );
        // Its own row: it stays.
        assert_eq!(
            row_target(&spots, Bar::Property, pos2(800.0, 78.0)),
            Some(into(1, None))
        );
        // Near the lines: a row of its own above or below.
        assert_eq!(
            row_target(&spots, Bar::Property, pos2(800.0, 26.0)),
            Some(RowTarget::NewRow {
                dock: Dock::Top,
                at: 0
            })
        );
        assert_eq!(
            row_target(&spots, Bar::Zoom, pos2(800.0, 94.0)),
            Some(RowTarget::NewRow {
                dock: Dock::Top,
                at: 2
            })
        );
        // Off the rows, or a bar without rows: the edges decide.
        assert_eq!(row_target(&spots, Bar::Property, pos2(800.0, 500.0)), None);
        assert_eq!(row_target(&spots, Bar::Toolbox, pos2(300.0, 40.0)), None);
        // The marks: a line before the Zoom toolbar, a band between rows.
        let m = row_mark(&spots, into(0, Some(Bar::Zoom)), Bar::Property).expect("mark");
        assert!((m.center().x - 900.0).abs() < 1e-3 && m.height() == 36.0);
        let m = row_mark(
            &spots,
            RowTarget::NewRow {
                dock: Dock::Top,
                at: 1,
            },
            Bar::Zoom,
        )
        .expect("band");
        assert!((m.center().y - 60.0).abs() < 1e-3 && m.width() == 1600.0);
    }

    /// Toolbars side by side keep their lengths while they fit, the last
    /// taking the rest; when they do not, the first ones keep theirs and
    /// the later ones get what is left, never under the floor (#855).
    #[test]
    fn a_row_shares_its_length() {
        assert_eq!(share_row(1600.0, &[900.0, 500.0]), vec![900.0, 700.0]);
        assert_eq!(share_row(1600.0, &[980.0, 940.0]), vec![980.0, 620.0]);
        assert_eq!(
            share_row(1000.0, &[980.0, 940.0]),
            vec![1000.0 - MIN_SLOT, MIN_SLOT]
        );
        assert_eq!(share_row(1600.0, &[f32::INFINITY]), vec![1600.0]);
        assert_eq!(share_row(100.0, &[300.0, 300.0]), vec![50.0, 50.0]);
        assert!(share_row(500.0, &[]).is_empty());
    }

    #[test]
    fn the_palette_starts_at_the_bottom_and_stays_where_it_is_put() {
        let layout = |palette: &str, version: Option<u32>| -> BarLayout {
            let version = version.map(|v| format!(r#", "version": {v}"#));
            let json = format!(
                r#"{{"toolbox": "left", "palette": "{palette}", "standard": "top",
                    "property": "top", "status": "bottom"{}}}"#,
                version.unwrap_or_default()
            );
            let mut l: BarLayout = serde_json::from_str(&json).expect("layout");
            l.migrate();
            assert_eq!(l.version, LAYOUT_VERSION);
            l
        };
        // A fresh install: the bottom.
        assert_eq!(BarLayout::default().palette, Dock::Bottom);
        assert_eq!(Bar::Palette.default_dock(), Dock::Bottom);
        // Version 1 put the palette at the right by default: back to the
        // bottom; anywhere else it stays.
        assert_eq!(layout("right", Some(1)).palette, Dock::Bottom);
        assert_eq!(layout("left", Some(1)).palette, Dock::Left);
        assert_eq!(layout("floating", Some(1)).palette, Dock::Floating);
        // Before that the bottom was the first place: a palette at the
        // right was the user's choice.
        assert_eq!(layout("right", None).palette, Dock::Right);
        assert_eq!(layout("bottom", None).palette, Dock::Bottom);
        // Put at the right now, it stays there across restarts.
        let mut l = BarLayout::default();
        assert!(l.set_dock(Bar::Palette, Dock::Right));
        let mut back: BarLayout =
            serde_json::from_str(&serde_json::to_string(&l).expect("json")).expect("back");
        back.migrate();
        assert_eq!(back.palette, Dock::Right);
    }
}
