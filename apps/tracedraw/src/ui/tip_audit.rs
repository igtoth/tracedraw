//! Tests: every control of the bars, the toolbox, the dockers, the status
//! bar and the dialogs shows a tip when the pointer rests on it.
//!
//! The window is drawn in many states (each tool with nothing, one and
//! every object selected, each docker tab, each dialog). In each, the
//! pointer rests on the middle of every enabled control not seen before,
//! and a tip has to open.

use super::root;
use crate::app::{App, DockerTab};
use crate::tools::Tool;
use egui::{Context, Event, Id, Order, Pos2, RawInput, Rect, WidgetType};
use std::collections::BTreeSet;
use tracedraw_core::geometry::{Point, Rect as MmRect};
use tracedraw_core::ShapeKind;

const SCREEN: egui::Vec2 = egui::vec2(1600.0, 1000.0);

/// The bottom of the menu bar, in points from the top of the window.
const MENU_BAR_BOTTOM: f32 = 26.0;

struct Driver {
    ctx: Context,
    time: f64,
}

impl Driver {
    fn new() -> Self {
        let ctx = crate::theme::ui_context();
        ctx.all_styles_mut(|s| {
            s.interaction.tooltip_delay = 0.0;
            s.interaction.tooltip_grace_time = 0.0;
            // Keeps the widgets' names (labels, kinds) for the report.
            s.debug.show_interactive_widgets = true;
        });
        Self { ctx, time: 0.0 }
    }

    fn frame(&mut self, app: &mut App, events: Vec<Event>) {
        self.time += 0.3;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let mut out = self.ctx.run_ui(input, |ui| root(app, ui));
        out.textures_delta.clear();
    }

    /// Whether a tip was shown in the last frame.
    fn tip_shown(&self) -> bool {
        self.ctx
            .viewport(|v| !v.prev_pass.tooltips.widget_tooltips.is_empty())
    }
}

/// A control left without a tip.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Missing {
    pub state: String,
    pub kind: String,
    pub label: String,
    /// The nearest text to its left on its row, to find it.
    pub near: String,
    pub at: (i32, i32),
}

fn mixed_app() -> App {
    let mut app = App::headless();
    app.settings.autocorrect.enabled = false;
    let r = app
        .new_shape(ShapeKind::Rect {
            rect: MmRect::new(10.0, 10.0, 60.0, 40.0),
            radius: 2.0,
            corners: None,
        })
        .expect("rect");
    app.new_shape(ShapeKind::Ellipse {
        rect: MmRect::new(70.0, 10.0, 120.0, 40.0),
        arc: None,
    });
    app.start_text(Point::new(20.0, 80.0), None);
    app.text_insert("Tip test");
    app.finish_text();
    app.select(vec![r]);
    app.duplicate();
    app.convert_to_bitmap(50.0, true);
    app.create_table(MmRect::new(10.0, 120.0, 80.0, 160.0));
    app.select_all();
    app
}

/// The controls of the last frame that a pointer can use, with the
/// texts drawn around them.
fn controls(
    ctx: &Context,
    dialogs: bool,
) -> (Vec<(Id, Rect, String, String)>, Vec<(Rect, String)>) {
    ctx.viewport(|v| {
        let w = &v.prev_pass.widgets;
        let mut out = Vec::new();
        let mut texts = Vec::new();
        for (layer, rects) in w.layers() {
            if matches!(layer.order, Order::Tooltip | Order::Debug)
                || (layer.order == Order::Foreground && !dialogs)
            {
                continue;
            }
            for r in rects {
                let info = w.info(r.id);
                let label = info.and_then(|i| i.label.clone()).unwrap_or_default();
                let kind = info.map(|i| i.typ).unwrap_or(WidgetType::Other);
                if !label.is_empty() {
                    texts.push((r.rect, label.clone()));
                }
                if !r.sense.interactive() {
                    continue;
                }
                if !r.enabled || r.interact_rect.width() < 2.0 || r.interact_rect.height() < 2.0 {
                    continue;
                }
                // The canvas, lists and scroll areas: no tip is wanted.
                if r.interact_rect.width() > 420.0 || r.interact_rect.height() > 120.0 {
                    continue;
                }
                // A link or other text that reads as what it does, and
                // the menu bar's titles, name themselves.
                if kind == WidgetType::Label && !label.is_empty() {
                    continue;
                }
                // A window's title bar and resize corner are not controls,
                // and its close cross is the toolkit's own.
                if matches!(kind, WidgetType::Window | WidgetType::ResizeHandle)
                    || (kind == WidgetType::Button && label == "Close window")
                {
                    continue;
                }
                if r.interact_rect.max.y <= MENU_BAR_BOTTOM {
                    continue;
                }
                out.push((r.id, r.interact_rect, format!("{kind:?}"), label));
            }
        }
        (out, texts)
    })
}

fn near_text(texts: &[(Rect, String)], at: Rect) -> String {
    texts
        .iter()
        .filter(|(r, _)| {
            r.center().x < at.left()
                && r.right() > at.left() - 90.0
                && (r.center().y - at.center().y).abs() < at.height().max(12.0)
        })
        .min_by(|a, b| {
            (at.left() - a.0.right())
                .partial_cmp(&(at.left() - b.0.right()))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, t)| t.clone())
        .unwrap_or_default()
}

/// Rest the pointer at `at` for a few frames: whether a tip showed.
fn hover(d: &mut Driver, app: &mut App, prepare: &mut dyn FnMut(&mut App), at: Pos2) -> bool {
    prepare(app);
    d.frame(app, vec![Event::PointerMoved(at)]);
    for _ in 0..3 {
        prepare(app);
        d.frame(app, vec![]);
    }
    d.tip_shown()
}

/// Rest the pointer on every new control of the current state.
fn check_state(
    d: &mut Driver,
    app: &mut App,
    state: &str,
    seen: &mut BTreeSet<(String, String, i32, i32)>,
    missing: &mut Vec<Missing>,
    prepare: &mut dyn FnMut(&mut App),
) {
    prepare(app);
    d.frame(app, vec![Event::PointerGone]);
    prepare(app);
    d.frame(app, vec![]);
    let in_dialog = !matches!(app.dialog, crate::ui::dialogs::Dialog::None);
    let (list, texts) = controls(&d.ctx, in_dialog);
    for (_, rect, kind, label) in list {
        let key = (
            kind.clone(),
            label.clone(),
            rect.center().x.round() as i32,
            rect.center().y.round() as i32,
        );
        if !seen.insert(key) {
            continue;
        }
        let at = rect.center();
        let mut shown = hover(d, app, prepare, at);
        if !shown {
            // The layout can still move on the first frames (fonts load
            // in the background): find the control again and retry.
            prepare(app);
            d.frame(app, vec![Event::PointerGone]);
            let (again, _) = controls(&d.ctx, in_dialog);
            if let Some((_, r, _, _)) = again
                .iter()
                .filter(|(_, _, k, l)| *k == kind && *l == label)
                .min_by(|a, b| {
                    a.1.center()
                        .distance(at)
                        .total_cmp(&b.1.center().distance(at))
                })
            {
                shown = hover(d, app, prepare, r.center());
            }
        }
        if !shown {
            missing.push(Missing {
                state: state.to_string(),
                kind,
                label,
                near: near_text(&texts, rect),
                at: (at.x.round() as i32, at.y.round() as i32),
            });
        }
        prepare(app);
        d.frame(app, vec![Event::PointerGone]);
    }
}

/// Every control in every state checked, the ones without a tip: the
/// bars, tabs, rulers, palette and status bar for every tool and
/// selection; with `dockers` every docker tab's contents too, with
/// `dialogs` every dialog.
pub fn audit(dockers: bool, dialogs: bool) -> Vec<Missing> {
    let mut d = Driver::new();
    let mut app = mixed_app();
    app.show_welcome = false;
    app.show_dockers = false;
    let mut seen = BTreeSet::new();
    let mut missing = Vec::new();
    let ids = app.selection.clone();
    let mut none = |_: &mut App| {};
    for tool in Tool::ALL {
        let mut sels = vec![Vec::new(), ids.clone()];
        sels.extend(ids.iter().map(|i| vec![*i]));
        for (n, sel) in sels.into_iter().enumerate() {
            app.set_tool(tool);
            app.select(sel);
            check_state(
                &mut d,
                &mut app,
                &format!("{tool:?} sel{n}"),
                &mut seen,
                &mut missing,
                &mut none,
            );
        }
    }
    app.set_tool(Tool::Pick);
    if dockers {
        app.show_dockers = true;
        for tab in DockerTab::ALL {
            app.docker_tab = tab;
            check_state(
                &mut d,
                &mut app,
                &format!("docker {tab:?}"),
                &mut seen,
                &mut missing,
                &mut none,
            );
        }
        app.docker_tab = DockerTab::Properties;
        app.show_dockers = false;
    }
    if dialogs {
        for dialog in crate::ui::dialogs::tests::all_dialogs(&app) {
            let name = format!("{dialog:?}");
            let name = name.split(['(', ' ', '{']).next().unwrap_or("").to_string();
            app.dialog = dialog;
            check_state(
                &mut d,
                &mut app,
                &format!("dialog {name}"),
                &mut seen,
                &mut missing,
                &mut none,
            );
            app.dialog = crate::ui::dialogs::Dialog::None;
            app.select_all();
        }
    }
    missing
}

/// Every control of the bars, tabs, rulers, palette and status bar shows
/// a tip, for every tool with nothing, one and every object selected, and
/// every control in every docker tab and every dialog.
#[test]
fn every_bar_docker_and_dialog_control_has_a_tip() {
    crate::i18n::set_language("en");
    let missing = audit(true, true);
    assert!(missing.is_empty(), "controls without a tip: {missing:#?}");
}

/// Writes every control without a tip to `$TIPS_AUDIT`.
#[test]
#[ignore]
fn report_controls_without_tips() {
    let Some(out) = std::env::var_os("TIPS_AUDIT") else {
        return;
    };
    let missing = audit(true, true);
    let lines: Vec<String> = missing
        .iter()
        .map(|m| {
            format!(
                "{}\t{}\t{:?}\tnear {:?}\t@{},{}",
                m.state, m.kind, m.label, m.near, m.at.0, m.at.1
            )
        })
        .collect();
    std::fs::write(out, lines.join("\n")).unwrap();
}
