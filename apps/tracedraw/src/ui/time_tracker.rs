//! The Time Tracker's toolbar (Window > Toolbars > Time Tracker) and its
//! dialog (Tools > Time Tracker). The records and the clock are in
//! `time_tracker.rs`.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::time_tracker::{self as tt, hms};
use crate::ui::dialogs::Dialog;
use crate::ui::tips::RichTip;
use egui::{pos2, vec2, Color32, Painter, Rect, RichText, Sense, Stroke, Ui};

/// What the clock is doing, for its light and tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockState {
    Paused,
    Recording,
    /// Running, but nobody has been at work for the idle time.
    Idle,
    /// Running, but the drawing has no active task.
    NoTask,
}

impl ClockState {
    pub fn of(app: &App, now: f64) -> Self {
        if !app.tracker.running {
            ClockState::Paused
        } else if !app.can_track() {
            ClockState::NoTask
        } else if app
            .tracker
            .idle(now, app.settings.time_tracker.idle_seconds())
        {
            ClockState::Idle
        } else {
            ClockState::Recording
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            ClockState::Paused => "tracker.state_paused",
            ClockState::Recording => "tracker.state_recording",
            ClockState::Idle => "tracker.state_idle",
            ClockState::NoTask => "tracker.no_task",
        }
    }

    fn color(self) -> Color32 {
        match self {
            ClockState::Recording => Color32::from_rgb(0xD8, 0x2C, 0x2C),
            ClockState::Idle => Color32::from_rgb(0xE0, 0x9A, 0x1A),
            ClockState::Paused | ClockState::NoTask => Color32::from_gray(0xA8),
        }
    }
}

/// A stopwatch: a ring, the crown on top and a hand.
pub fn paint_stopwatch(p: &Painter, r: Rect, color: Color32) {
    let s = r.width().min(r.height());
    let c = r.center() + vec2(0.0, s * 0.06);
    let radius = s * 0.36;
    let stroke = Stroke::new((s / 12.0).max(1.2), color);
    p.circle_stroke(c, radius, stroke);
    let top = c - vec2(0.0, radius);
    p.line_segment([top, top - vec2(0.0, s * 0.1)], stroke);
    p.line_segment(
        [
            top - vec2(s * 0.1, s * 0.12),
            top - vec2(-s * 0.1, s * 0.12),
        ],
        stroke,
    );
    p.line_segment([c, c + vec2(radius * 0.45, -radius * 0.55)], stroke);
}

/// The record button's picture: a triangle to start, two bars to pause.
fn paint_play_pause(p: &Painter, r: Rect, running: bool, color: Color32) {
    let c = r.center();
    if running {
        for dx in [-3.5, 3.5] {
            p.rect_filled(
                Rect::from_center_size(c + vec2(dx, 0.0), vec2(4.0, 12.0)),
                0.5,
                color,
            );
        }
    } else {
        p.add(egui::Shape::convex_polygon(
            vec![
                c + vec2(-4.5, -6.5),
                c + vec2(6.5, 0.0),
                c + vec2(-4.5, 6.5),
            ],
            color,
            Stroke::NONE,
        ));
    }
}

/// Window > Toolbars > Time Tracker: the stopwatch, the task list, the
/// task's time, the record button with its light, the total and the
/// Tasks button.
pub fn toolbar(app: &mut App, ui: &mut Ui) {
    let doc = app.has_document() && !app.show_welcome;
    let now = ui.input(|i| i.time);
    ui.horizontal(|ui| {
        ui.set_height(28.0);
        let (icon, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
        paint_stopwatch(
            ui.painter(),
            icon,
            if doc {
                Tokens::icon()
            } else {
                Tokens::icon_off()
            },
        );
        ui.add_enabled_ui(doc, |ui| {
            let (names, active) = if doc {
                let t = &app.doc().time_tracker;
                (
                    t.tasks.iter().map(|t| t.name.clone()).collect::<Vec<_>>(),
                    t.active.filter(|&i| i < t.tasks.len()),
                )
            } else {
                (Vec::new(), None)
            };
            let shown = active
                .and_then(|i| names.get(i).cloned())
                .unwrap_or_else(|| tr("tracker.no_task"));
            let mut pick = None;
            let mut add = false;
            egui::ComboBox::from_id_salt("time_tracker_task")
                .selected_text(shown)
                .width(180.0)
                .show_ui(ui, |ui| {
                    for (i, name) in names.iter().enumerate() {
                        if ui.selectable_label(active == Some(i), name).clicked() {
                            pick = Some(i);
                        }
                    }
                    if !names.is_empty() {
                        ui.separator();
                    }
                    if ui.selectable_label(false, tr("tracker.new_task")).clicked() {
                        add = true;
                    }
                })
                .response
                .rich_tip(tr("tracker.task"));
            if let Some(i) = pick {
                app.edit_time_tracker(|t| t.active = Some(i));
            }
            if add {
                app.add_tracked_task();
            }
            let time = app
                .active_task_seconds()
                .map(hms)
                .unwrap_or_else(|| "-:--:--".into());
            ui.add_sized(
                [70.0, 22.0],
                egui::Label::new(RichText::new(time).monospace().size(13.0)),
            );
            // The record button and its light.
            let running = app.tracker.running;
            let (r, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect(
                    r,
                    0.0,
                    crate::theme::Tokens::hovered_row(),
                    Stroke::new(1.0, crate::theme::Tokens::selected_row()),
                    egui::StrokeKind::Inside,
                );
            }
            let color = if doc {
                Tokens::icon()
            } else {
                Tokens::icon_off()
            };
            paint_play_pause(ui.painter(), r, running, color);
            let resp = resp.rich_tip(tr(if running {
                "tracker.pause"
            } else {
                "tracker.record"
            }));
            if resp.clicked() {
                app.toggle_time_clock();
            }
            let state = ClockState::of(app, now);
            let (dot, dot_resp) = ui.allocate_exact_size(vec2(12.0, 22.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 4.0, state.color());
            dot_resp.rich_tip(tr(state.key()));
            ui.add(egui::Separator::default().vertical().spacing(10.0));
            let total = if doc {
                let t = &app.doc().time_tracker;
                let extra = if t.active_task().is_some() {
                    app.tracker.pending
                } else {
                    0.0
                };
                t.total_seconds() + extra
            } else {
                0.0
            };
            ui.label(trf("tracker.total_is", &[("t", &hms(total))]));
            ui.add(egui::Separator::default().vertical().spacing(10.0));
            if ui.button(tr("tracker.tasks")).clicked() {
                app.dialog = Dialog::TimeTracker(TrackerDialog::default());
            }
        });
    });
}

/// The Time Tracker dialog: the tasks in a table, the selected task's
/// fields, the idle time and the exports.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackerDialog {
    pub selected: Option<usize>,
    /// The task the fields below were filled from.
    for_task: Option<usize>,
    pub name: String,
    pub time: String,
    pub start: String,
    pub finish: String,
    /// Fields typed into since they were last applied (name, time, start,
    /// finish).
    edited: [bool; 4],
}

const ROW_H: f32 = 22.0;
/// Column widths: the active mark, task, time, start, finish.
const COLS: [f32; 5] = [20.0, 206.0, 84.0, 134.0, 134.0];

fn table_width() -> f32 {
    COLS.iter().sum()
}

impl TrackerDialog {
    /// The fields' texts for task `i` as the drawing has it now.
    fn values(app: &App, i: Option<usize>) -> [String; 4] {
        let tz = tt::local_zone();
        let t = &app.doc().time_tracker;
        match i.and_then(|i| t.tasks.get(i).map(|task| (i, task))) {
            Some((i, task)) => [
                task.name.clone(),
                hms(tt::task_seconds(t, i, app.tracker.pending)),
                task.start
                    .map(|s| tt::date_time(s, &tz))
                    .unwrap_or_default(),
                task.finish
                    .map(|s| tt::date_time(s, &tz))
                    .unwrap_or_default(),
            ],
            None => Default::default(),
        }
    }

    /// Apply the typed fields to the task they belong to; unreadable
    /// values are dropped (the field shows the kept value again).
    pub fn commit(&mut self, app: &mut App) {
        let Some(i) = self.for_task else {
            self.edited = [false; 4];
            return;
        };
        let tz = tt::local_zone();
        let edited = std::mem::take(&mut self.edited);
        let name = self.name.trim().to_string();
        let time = tt::parse_hms(&self.time);
        let start = tt::parse_date_time(&self.start, &tz);
        let finish = tt::parse_date_time(&self.finish, &tz);
        app.edit_time_tracker(|t| {
            let Some(task) = t.tasks.get_mut(i) else {
                return;
            };
            if edited[0] && !name.is_empty() {
                task.name = name;
            }
            if let (true, Some(s)) = (edited[1], time) {
                task.seconds = s;
            }
            if let (true, Some(s)) = (edited[2], start) {
                task.start = s;
            }
            if let (true, Some(s)) = (edited[3], finish) {
                task.finish = s;
            }
        });
    }

    /// Select task `i`, applying what was typed for the previous one.
    pub fn select(&mut self, app: &mut App, i: Option<usize>) {
        if self.edited.iter().any(|e| *e) {
            self.commit(app);
        }
        self.selected = i;
    }
}

/// Show the dialog; `close` is set when it should close.
pub fn dialog(app: &mut App, ctx: &egui::Context, st: &mut TrackerDialog, close: &mut bool) {
    if !app.has_document() {
        *close = true;
        return;
    }
    let now = ctx.input(|i| i.time);
    crate::ui::dialogs::window(ctx, tr("tracker.title")).show(ctx, |ui| {
        let count = app.doc().time_tracker.tasks.len();
        if st.selected.is_some_and(|i| i >= count) {
            st.selected = count.checked_sub(1);
        }
        if st.selected.is_none() && count > 0 {
            st.selected = app
                .doc()
                .time_tracker
                .active
                .filter(|&i| i < count)
                .or(Some(0));
        }
        table(app, ui, st);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button(tr("tracker.new_task")).clicked() {
                st.select(app, None);
                app.add_tracked_task();
                st.selected = app.doc().time_tracker.tasks.len().checked_sub(1);
            }
            let has = st.selected.is_some();
            if ui
                .add_enabled(has, egui::Button::new(tr("tracker.delete")))
                .clicked()
            {
                if let Some(i) = st.selected {
                    st.edited = [false; 4];
                    app.edit_time_tracker(|t| {
                        if i < t.tasks.len() {
                            t.tasks.remove(i);
                            t.active = match t.active {
                                Some(a) if a == i => None,
                                Some(a) if a > i => Some(a - 1),
                                a => a,
                            };
                        }
                    });
                    st.selected = None;
                    st.for_task = None;
                }
            }
            let active = app.doc().time_tracker.active;
            if ui
                .add_enabled(
                    has && active != st.selected,
                    egui::Button::new(tr("tracker.make_active")),
                )
                .clicked()
            {
                let sel = st.selected;
                app.edit_time_tracker(|t| t.active = sel);
            }
            if ui
                .add_enabled(has, egui::Button::new(tr("tracker.reset")))
                .clicked()
            {
                if let Some(i) = st.selected {
                    st.edited = [false; 4];
                    app.edit_time_tracker(|t| {
                        if let Some(task) = t.tasks.get_mut(i) {
                            task.seconds = 0.0;
                            task.start = None;
                            task.finish = None;
                        }
                    });
                    // Time counted for it so far goes too.
                    if app.doc().time_tracker.active == Some(i) {
                        app.tracker.pending = 0.0;
                    }
                }
            }
            ui.add_space(8.0);
            let state = ClockState::of(app, now);
            let label = tr(if app.tracker.running {
                "tracker.pause"
            } else {
                "tracker.record"
            });
            if ui.button(label).clicked() {
                st.select(app, st.selected);
                app.toggle_time_clock();
            }
            let (dot, _) = ui.allocate_exact_size(vec2(12.0, 18.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 4.0, state.color());
            ui.label(RichText::new(tr(state.key())).color(Tokens::text_dim()));
        });
        ui.add_space(6.0);
        fields(app, ui, st);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(tr("tracker.idle_before"));
            let mut minutes = app.settings.time_tracker.idle_minutes;
            if ui
                .add(crate::ui::field::NumField::new(&mut minutes).range(1..=120))
                .changed()
            {
                app.settings.time_tracker.idle_minutes = minutes;
            }
            ui.label(tr("tracker.idle_after"));
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button(tr("tracker.export_csv")).clicked() {
                st.select(app, st.selected);
                export(app, Export::Csv);
            }
            if ui.button(tr("tracker.export_text")).clicked() {
                st.select(app, st.selected);
                export(app, Export::Text);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.close")))
                    .rich_tip(crate::ui::tips::keyed("dialog.close"))
                    .clicked()
                {
                    *close = true;
                }
            });
        });
    });
    if *close {
        st.select(app, st.selected);
    }
}

/// The tasks: a header, one row per task (a dot at the active one), the
/// total. A click selects a row, a double click makes its task active.
fn table(app: &mut App, ui: &mut Ui, st: &mut TrackerDialog) {
    let tz = tt::local_zone();
    let width = table_width();
    let tracker = app.doc().time_tracker.clone();
    let pending = app.tracker.pending;
    let font = egui::FontId::proportional(13.0);
    let cell = |p: &Painter, r: Rect, col: usize, text: &str, color: Color32, bold: bool| {
        let x0 = r.left() + COLS[..col].iter().sum::<f32>();
        // The time sits a little away from the dates after it.
        let right_pad = if col == 2 { 14.0 } else { 4.0 };
        let clip = Rect::from_min_max(
            pos2(x0 + 4.0, r.top()),
            pos2(x0 + COLS[col] - right_pad, r.bottom()),
        );
        let font = if bold {
            crate::theme::bold(13.0)
        } else {
            font.clone()
        };
        let (anchor, align) = if col == 2 {
            (pos2(clip.right(), r.center().y), egui::Align2::RIGHT_CENTER)
        } else {
            (pos2(clip.left(), r.center().y), egui::Align2::LEFT_CENTER)
        };
        p.with_clip_rect(clip)
            .text(anchor, align, text, font, color);
    };
    // Header.
    let (head, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
    let p = ui.painter();
    p.rect_filled(head, 0.0, crate::theme::Tokens::tab_strip());
    for (col, key) in [
        "",
        "tracker.task",
        "tracker.time",
        "tracker.start",
        "tracker.finish",
    ]
    .iter()
    .enumerate()
    {
        if !key.is_empty() {
            cell(p, head, col, &tr(key), Tokens::text(), false);
        }
    }
    let mut clicked = None;
    let mut activate = None;
    let list_h = (ROW_H * 8.0).min(ROW_H * tracker.tasks.len().max(3) as f32);
    let frame = egui::Frame::new()
        .fill(crate::theme::Tokens::field())
        .stroke(Stroke::new(1.0, crate::theme::Tokens::control_border()));
    frame.show(ui, |ui| {
        ui.set_width(width - 2.0);
        egui::ScrollArea::vertical()
            .max_height(list_h)
            .min_scrolled_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if tracker.tasks.is_empty() {
                    let (r, _) =
                        ui.allocate_exact_size(vec2(width - 2.0, ROW_H * 2.0), Sense::hover());
                    ui.painter().text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        tr("tracker.empty"),
                        font.clone(),
                        Tokens::text_dim(),
                    );
                }
                for (i, task) in tracker.tasks.iter().enumerate() {
                    let (r, resp) =
                        ui.allocate_exact_size(vec2(width - 2.0, ROW_H), Sense::click());
                    let p = ui.painter();
                    if st.selected == Some(i) {
                        p.rect_filled(r, 0.0, Tokens::tool_active());
                    } else if resp.hovered() {
                        p.rect_filled(r, 0.0, Tokens::tool_hover());
                    }
                    if tracker.active == Some(i) {
                        let c = pos2(r.left() + COLS[0] / 2.0 + 1.0, r.center().y);
                        p.circle_filled(c, 3.5, Tokens::ACCENT);
                    }
                    cell(p, r, 1, &task.name, Tokens::text(), false);
                    cell(
                        p,
                        r,
                        2,
                        &hms(tt::task_seconds(&tracker, i, pending)),
                        Tokens::text(),
                        false,
                    );
                    let date =
                        |s: Option<i64>| s.map(|s| tt::date_time(s, &tz)).unwrap_or_default();
                    cell(p, r, 3, &date(task.start), Tokens::text(), false);
                    cell(p, r, 4, &date(task.finish), Tokens::text(), false);
                    if resp.clicked() {
                        clicked = Some(i);
                    }
                    if resp.double_clicked() {
                        activate = Some(i);
                    }
                }
            });
    });
    // The total under the list.
    let (r, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
    let extra = if tracker.active_task().is_some() {
        pending
    } else {
        0.0
    };
    cell(
        ui.painter(),
        r,
        1,
        &tr("tracker.total"),
        Tokens::text(),
        true,
    );
    cell(
        ui.painter(),
        r,
        2,
        &hms(tracker.total_seconds() + extra),
        Tokens::text(),
        true,
    );
    if let Some(i) = clicked {
        if st.selected != Some(i) {
            st.select(app, Some(i));
        }
    }
    if let Some(i) = activate {
        app.edit_time_tracker(|t| t.active = Some(i));
    }
}

/// The selected task's name, time, start and finish. Typed values apply
/// when the field is left (or Enter is pressed).
fn fields(app: &mut App, ui: &mut Ui, st: &mut TrackerDialog) {
    let has = st.selected.is_some();
    if st.for_task != st.selected {
        st.for_task = st.selected;
        st.edited = [false; 4];
    }
    let values = TrackerDialog::values(app, st.selected);
    let mut edited = st.edited;
    let mut bufs = [
        std::mem::take(&mut st.name),
        std::mem::take(&mut st.time),
        std::mem::take(&mut st.start),
        std::mem::take(&mut st.finish),
    ];
    let mut lost = false;
    let mut field = |ui: &mut Ui, k: usize, label: &str, width: f32, hint: &str| {
        ui.allocate_ui_with_layout(
            vec2(54.0, 24.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| ui.label(label),
        );
        if !edited[k] {
            bufs[k] = values[k].clone();
        }
        let resp = ui
            .add_enabled_ui(has, |ui| {
                ui.add_sized(
                    [width, 24.0],
                    egui::TextEdit::singleline(&mut bufs[k])
                        .desired_width(width)
                        .hint_text(hint),
                )
            })
            .inner;
        if resp.changed() {
            edited[k] = true;
        }
        if resp.lost_focus() {
            lost = true;
        }
    };
    ui.horizontal(|ui| {
        field(ui, 0, &tr("tracker.name"), 230.0, "");
        ui.add_space(12.0);
        field(ui, 1, &tr("tracker.time"), 150.0, "h:mm:ss");
    });
    ui.horizontal(|ui| {
        field(ui, 2, &tr("tracker.start"), 230.0, "YYYY-MM-DD HH:MM");
        ui.add_space(12.0);
        field(ui, 3, &tr("tracker.finish"), 150.0, "YYYY-MM-DD HH:MM");
    });
    let [name, time, start, finish] = bufs;
    st.name = name;
    st.time = time;
    st.start = start;
    st.finish = finish;
    st.edited = edited;
    if lost && st.edited.iter().any(|e| *e) {
        st.commit(app);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Export {
    Csv,
    Text,
}

/// Save the records as CSV or as a text report, where the user chooses.
fn export(app: &mut App, kind: Export) {
    let (ext, filter) = match kind {
        Export::Csv => ("csv", "CSV (*.csv)"),
        Export::Text => ("txt", "Text (*.txt)"),
    };
    let name = format!(
        "{} {}.{ext}",
        app.document_name(),
        tr("tracker.file_suffix")
    );
    crate::files::Dialog::new()
        .add_filter(filter, &[ext])
        .set_file_name(name)
        .save_file(app, move |app, path| {
            let tz = tt::local_zone();
            let tracker = &app.doc().time_tracker;
            let data = match kind {
                Export::Csv => tt::csv(tracker, app.tracker.pending, &tz),
                Export::Text => tt::text_report(
                    &app.document_name(),
                    tracker,
                    app.tracker.pending,
                    &tz,
                    tt::unix_now(),
                ),
            };
            app.status = match crate::files::write(&path, data) {
                Ok(()) => trf("tracker.exported", &[("p", &path.display().to_string())]),
                Err(e) => trf("tracker.export_failed", &[("e", &e.to_string())]),
            };
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{geometry::Size, Document};

    fn app_with_tasks() -> App {
        let mut app = App::headless();
        app.open_document(Document::new("t", Size::new(100.0, 100.0)), None);
        app.add_tracked_task();
        app.add_tracked_task();
        app
    }

    #[test]
    fn typed_fields_apply_to_their_task_and_bad_ones_are_dropped() {
        let mut app = app_with_tasks();
        let mut st = TrackerDialog {
            selected: Some(0),
            for_task: Some(0),
            ..Default::default()
        };
        st.name = "Layout".into();
        st.time = "1:30".into();
        st.start = "not a date".into();
        st.finish = String::new();
        st.edited = [true, true, true, true];
        st.commit(&mut app);
        let t = &app.doc().time_tracker.tasks[0];
        assert_eq!(t.name, "Layout");
        assert_eq!(t.seconds, 5400.0);
        assert_eq!(t.start, None);
        assert_eq!(t.finish, None);
        assert_eq!(st.edited, [false; 4]);
        // Selecting another task applies what was typed first.
        st.name = "Ink".into();
        st.edited[0] = true;
        st.select(&mut app, Some(1));
        assert_eq!(app.doc().time_tracker.tasks[0].name, "Ink");
        assert_eq!(st.selected, Some(1));
        // An empty name keeps the old one.
        st.for_task = Some(1);
        st.name = "  ".into();
        st.edited[0] = true;
        st.commit(&mut app);
        assert_eq!(app.doc().time_tracker.tasks[1].name, "Task 2");
    }

    #[test]
    fn the_light_tells_what_the_clock_does() {
        let mut app = app_with_tasks();
        assert_eq!(ClockState::of(&app, 0.0), ClockState::Paused);
        app.toggle_time_clock();
        app.time_tracker_frame(0.0, true);
        assert_eq!(ClockState::of(&app, 1.0), ClockState::Recording);
        let idle = app.settings.time_tracker.idle_seconds();
        assert_eq!(ClockState::of(&app, idle + 1.0), ClockState::Idle);
        app.edit_time_tracker(|t| t.active = None);
        assert_eq!(ClockState::of(&app, 1.0), ClockState::NoTask);
    }

    #[test]
    fn the_toolbar_and_dialog_draw() {
        let mut app = app_with_tasks();
        app.toggle_time_clock();
        let ctx = crate::theme::ui_context();
        let mut st = TrackerDialog::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(1600.0, 1000.0))),
            ..Default::default()
        };
        for _ in 0..2 {
            let mut out = ctx.run_ui(input.clone(), |ui| {
                toolbar(&mut app, ui);
                let mut close = false;
                dialog(&mut app, ui.ctx(), &mut st, &mut close);
                assert!(!close);
            });
            out.textures_delta.clear();
        }
        // The dialog picked the active task.
        assert_eq!(st.selected, Some(1));
    }
}
