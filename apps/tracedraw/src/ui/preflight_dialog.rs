//! File > Preflight: the issues the preflight finds (`preflight.rs`), the
//! hint for the one picked, and the checks to run with their thresholds.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::preflight::{self, Check, Issue, PreflightPrefs, Severity};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, RichText, Sense, Stroke, Ui};

/// The dialog's state: what the last run found and the row picked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreflightState {
    pub issues: Vec<Issue>,
    /// The row picked: a kind of issue, or one issue (its index).
    pub selected: Option<Row>,
    /// The kinds of issue opened to show their objects.
    pub open: Vec<Check>,
    /// The preferences the issues were found with; `None` before the first
    /// run.
    ran_with: Option<PreflightPrefs>,
    pub settings_open: bool,
}

/// A row of the list: the heading of a kind of issue, or one issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Group(Check),
    Issue(usize),
}

/// Kinds with at most this many issues start opened.
const OPEN_UP_TO: usize = 3;

impl PreflightState {
    /// Run the checks again.
    pub fn refresh(&mut self, app: &App) {
        let first = self.ran_with.is_none();
        self.issues = issues_of(app);
        self.ran_with = Some(app.settings.preflight.clone());
        let n = self.issues.len();
        if let Some(Row::Issue(i)) = self.selected {
            if i >= n {
                self.selected = None;
            }
        }
        if first {
            self.open = Check::ALL
                .into_iter()
                .filter(|c| {
                    let k = self.issues.iter().filter(|i| i.check == *c).count();
                    k > 0 && k <= OPEN_UP_TO
                })
                .collect();
        }
    }

    /// The rows in order: each kind found, then its issues when opened.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for c in Check::ALL {
            let mut of_kind = self
                .issues
                .iter()
                .enumerate()
                .filter(|(_, i)| i.check == c)
                .map(|(n, _)| Row::Issue(n))
                .peekable();
            if of_kind.peek().is_none() {
                continue;
            }
            rows.push(Row::Group(c));
            if self.open.contains(&c) {
                rows.extend(of_kind);
            }
        }
        rows
    }

    /// Open or close a kind of issue.
    pub fn toggle(&mut self, c: Check) {
        if let Some(i) = self.open.iter().position(|o| *o == c) {
            self.open.remove(i);
        } else {
            self.open.push(c);
        }
    }
}

/// The issues of the active drawing.
pub fn issues_of(app: &App) -> Vec<Issue> {
    if !app.has_document() {
        return Vec::new();
    }
    preflight::run(app.doc(), &app.missing_fonts, &app.settings.preflight)
}

/// The one-line summary the Print dialog shows: the counts, or that all is
/// well.
pub fn summary(issues: &[Issue]) -> String {
    let (warnings, notes) = preflight::counts(issues);
    if issues.is_empty() {
        tr("preflight.none")
    } else {
        trf(
            "preflight.summary",
            &[("w", &warnings.to_string()), ("n", &notes.to_string())],
        )
    }
}

/// A warning triangle or a note's circle.
pub fn paint_icon(p: &Painter, c: Pos2, severity: Severity) {
    match severity {
        Severity::Warning => {
            let pts = vec![c + vec2(0.0, -6.5), c + vec2(7.0, 5.5), c + vec2(-7.0, 5.5)];
            p.add(egui::Shape::convex_polygon(
                pts,
                Color32::from_rgb(0xF2, 0xB7, 0x05),
                Stroke::new(1.0, Color32::from_rgb(0xB0, 0x82, 0x00)),
            ));
            p.text(
                c + vec2(0.0, 1.5),
                egui::Align2::CENTER_CENTER,
                "!",
                crate::theme::bold(10.0),
                Color32::BLACK,
            );
        }
        Severity::Info => {
            p.circle_filled(c, 6.5, Color32::from_rgb(0x1F, 0x6F, 0xC5));
            p.text(
                c + vec2(0.0, 0.5),
                egui::Align2::CENTER_CENTER,
                "i",
                crate::theme::bold(10.0),
                Color32::WHITE,
            );
        }
    }
}

fn paint_ok(p: &Painter, c: Pos2) {
    p.circle_filled(c, 7.0, Color32::from_rgb(0x2E, 0x9E, 0x45));
    let s = Stroke::new(1.8, Color32::WHITE);
    p.line_segment([c + vec2(-3.5, 0.0), c + vec2(-1.0, 2.8)], s);
    p.line_segment([c + vec2(-1.0, 2.8), c + vec2(3.8, -2.8)], s);
}

const ROW_H: f32 = 22.0;
/// Column widths: icon, check, page, object, detail.
const COLS: [f32; 5] = [24.0, 220.0, 56.0, 190.0, 110.0];

/// Show the dialog; `close` is set when it should close.
pub fn dialog(app: &mut App, ctx: &egui::Context, st: &mut PreflightState, close: &mut bool) {
    if !app.has_document() {
        *close = true;
        return;
    }
    if st.ran_with.as_ref() != Some(&app.settings.preflight) {
        st.refresh(app);
    }
    crate::ui::dialogs::window(ctx, tr("preflight.title")).show(ctx, |ui| {
        // The verdict.
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            if st.issues.is_empty() {
                paint_ok(ui.painter(), r.center());
            } else {
                let (warnings, _) = preflight::counts(&st.issues);
                let sev = if warnings > 0 {
                    Severity::Warning
                } else {
                    Severity::Info
                };
                paint_icon(ui.painter(), r.center(), sev);
            }
            ui.label(RichText::new(summary(&st.issues)).strong());
        });
        ui.add_space(2.0);
        issue_table(app, ui, st);
        // The hint for the picked issue.
        let check = match st.selected {
            Some(Row::Group(c)) => Some(c),
            Some(Row::Issue(i)) => st.issues.get(i).map(|i| i.check),
            None => None,
        };
        let hint = check
            .map(|c| tr(&format!("preflight.hint.{}", c.id())))
            .unwrap_or_else(|| tr("preflight.pick_hint"));
        ui.allocate_ui_with_layout(
            vec2(COLS.iter().sum::<f32>(), 34.0),
            egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true),
            |ui| {
                ui.set_min_height(34.0);
                ui.add(
                    egui::Label::new(RichText::new(hint).color(Tokens::text_dim()).size(12.0))
                        .wrap(),
                );
            },
        );
        if st.settings_open {
            crate::ui::chrome::section(ui, &tr("preflight.settings"));
            settings(app, ui);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(tr("preflight.refresh")).clicked() {
                st.refresh(app);
            }
            let label = if st.settings_open {
                tr("preflight.hide_settings")
            } else {
                tr("preflight.show_settings")
            };
            if ui.button(label).clicked() {
                st.settings_open = !st.settings_open;
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
}

/// The issues by kind: a heading row for each kind with its count (a
/// click opens or closes it), then one row per issue with its page,
/// object and detail (a click goes to the page and selects the object).
fn issue_table(app: &mut App, ui: &mut Ui, st: &mut PreflightState) {
    let width: f32 = COLS.iter().sum();
    let font = egui::FontId::proportional(13.0);
    let cell = |p: &Painter, r: Rect, x: f32, w: f32, text: &str, color: Color32, bold: bool| {
        let clip = Rect::from_min_max(
            pos2(r.left() + x + 4.0, r.top()),
            pos2(r.left() + x + w - 4.0, r.bottom()),
        );
        let f = if bold {
            crate::theme::bold(13.0)
        } else {
            font.clone()
        };
        p.with_clip_rect(clip).text(
            pos2(clip.left(), r.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            f,
            color,
        );
    };
    let col_x = |col: usize| COLS[..col].iter().sum::<f32>();
    let (head, _) = ui.allocate_exact_size(vec2(width, ROW_H), Sense::hover());
    ui.painter()
        .rect_filled(head, 0.0, crate::theme::Tokens::tab_strip());
    for (col, key) in [
        "",
        "preflight.issue",
        "preflight.page",
        "preflight.object",
        "preflight.detail",
    ]
    .iter()
    .enumerate()
    {
        if !key.is_empty() {
            cell(
                ui.painter(),
                head,
                col_x(col),
                COLS[col],
                &tr(key),
                Tokens::text(),
                false,
            );
        }
    }
    let rows = st.rows();
    let list_h = ROW_H * 10.0;
    let mut clicked = None;
    egui::Frame::new()
        .fill(crate::theme::Tokens::field())
        .stroke(Stroke::new(1.0, crate::theme::Tokens::control_border()))
        .show(ui, |ui| {
            ui.set_width(width - 2.0);
            egui::ScrollArea::vertical()
                .id_salt("preflight_issues")
                .max_height(list_h)
                .min_scrolled_height(list_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if rows.is_empty() {
                        let (r, _) =
                            ui.allocate_exact_size(vec2(width - 2.0, ROW_H * 2.0), Sense::hover());
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            tr("preflight.empty"),
                            font.clone(),
                            Tokens::text_dim(),
                        );
                    }
                    for row in &rows {
                        let (r, resp) =
                            ui.allocate_exact_size(vec2(width - 2.0, ROW_H), Sense::click());
                        let p = ui.painter();
                        if st.selected == Some(*row) {
                            p.rect_filled(r, 0.0, Tokens::tool_active());
                        } else if resp.hovered() {
                            p.rect_filled(r, 0.0, Tokens::tool_hover());
                        }
                        match *row {
                            Row::Group(c) => {
                                let n = st.issues.iter().filter(|i| i.check == c).count();
                                let open = st.open.contains(&c);
                                // The open/closed arrow, then the icon.
                                let a = pos2(r.left() + 8.0, r.center().y);
                                let pts = if open {
                                    vec![
                                        a + vec2(-3.5, -2.0),
                                        a + vec2(3.5, -2.0),
                                        a + vec2(0.0, 2.5),
                                    ]
                                } else {
                                    vec![
                                        a + vec2(-2.0, -3.5),
                                        a + vec2(2.5, 0.0),
                                        a + vec2(-2.0, 3.5),
                                    ]
                                };
                                p.add(egui::Shape::convex_polygon(
                                    pts,
                                    Tokens::text_dim(),
                                    Stroke::NONE,
                                ));
                                paint_icon(p, pos2(r.left() + 24.0, r.center().y), c.severity());
                                let name = format!("{}  ({n})", tr(&c.key()));
                                cell(p, r, 32.0, width - 36.0, &name, Tokens::text(), true);
                            }
                            Row::Issue(i) => {
                                let Some(issue) = st.issues.get(i) else {
                                    continue;
                                };
                                let page = issue
                                    .page
                                    .map(|n| (n + 1).to_string())
                                    .unwrap_or_else(|| tr("preflight.master"));
                                let object = issue
                                    .shape
                                    .and_then(|id| app.doc().find_shape(id))
                                    .map(crate::ui::objects_docker::object_label)
                                    .unwrap_or_default();
                                cell(
                                    p,
                                    r,
                                    col_x(1) + 14.0,
                                    COLS[1] - 14.0,
                                    &tr(&issue.check.key()),
                                    Tokens::text_dim(),
                                    false,
                                );
                                cell(p, r, col_x(2), COLS[2], &page, Tokens::text(), false);
                                cell(p, r, col_x(3), COLS[3], &object, Tokens::text(), false);
                                cell(
                                    p,
                                    r,
                                    col_x(4),
                                    COLS[4],
                                    &issue.detail,
                                    Tokens::text_dim(),
                                    false,
                                );
                            }
                        }
                        if resp.clicked() {
                            clicked = Some(*row);
                        }
                    }
                });
        });
    if let Some(row) = clicked {
        st.selected = Some(row);
        match row {
            Row::Group(c) => st.toggle(c),
            Row::Issue(i) => {
                if let Some(issue) = st.issues.get(i).cloned() {
                    show_issue(app, &issue);
                }
            }
        }
    }
}

/// Go to the issue's page and select its object.
pub fn show_issue(app: &mut App, issue: &Issue) {
    if let Some(p) = issue.page {
        if p != app.page_index() {
            app.goto_page(p);
        }
    }
    if let Some(id) = issue.shape {
        if app.doc().find_shape(id).is_some() {
            app.select(vec![id]);
        }
    }
}

/// The checks to run, in two columns, and the thresholds.
fn settings(app: &mut App, ui: &mut Ui) {
    let prefs = &mut app.settings.preflight;
    egui::Grid::new("preflight_checks")
        .num_columns(2)
        .spacing([18.0, 4.0])
        .show(ui, |ui| {
            for (n, c) in Check::ALL.iter().enumerate() {
                let mut on = prefs.runs(*c);
                if ui.checkbox(&mut on, tr(&c.key())).changed() {
                    prefs.set_runs(*c, on);
                }
                if n % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    ui.add_space(4.0);
    let number = |ui: &mut Ui,
                  before: &str,
                  v: &mut f64,
                  range: std::ops::RangeInclusive<f64>,
                  after: &str| {
        ui.horizontal(|ui| {
            ui.label(tr(before));
            ui.add(
                crate::ui::field::NumField::new(v)
                    .range(range)
                    .speed(0.5)
                    .max_decimals(2),
            );
            ui.label(after);
        });
    };
    number(
        ui,
        "preflight.min_dpi",
        &mut prefs.min_dpi,
        10.0..=2400.0,
        "dpi",
    );
    number(
        ui,
        "preflight.max_dpi",
        &mut prefs.max_dpi,
        100.0..=10000.0,
        "dpi",
    );
    number(
        ui,
        "preflight.min_text",
        &mut prefs.min_text_pt,
        1.0..=24.0,
        "pt",
    );
    number(
        ui,
        "preflight.rich_black_below",
        &mut prefs.rich_black_below_pt,
        1.0..=72.0,
        "pt",
    );
    number(
        ui,
        "preflight.min_outline",
        &mut prefs.min_outline_mm,
        0.0..=5.0,
        "mm",
    );
    if ui.button(tr("preflight.defaults")).clicked() {
        *prefs = PreflightPrefs::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{document::ShapeKind, geometry::Size, Document};

    #[test]
    fn picking_an_issue_selects_its_object_and_the_dialog_draws() {
        let mut app = App::headless();
        let mut doc = Document::new("t", Size::new(100.0, 100.0));
        let id = doc.ids_mut().shape();
        let mut s = tracedraw_core::Shape::new(
            id,
            ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(200.0, 10.0, 210.0, 20.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.stroke = None;
        doc.pages[0].layers[0].shapes.push(s);
        app.open_document(doc, None);
        let mut st = PreflightState::default();
        st.refresh(&app);
        assert_eq!(st.issues.len(), 1);
        assert_eq!(st.issues[0].check, Check::OffPage);
        assert!(summary(&st.issues).contains('1'));
        // One kind with one issue: opened at once.
        assert_eq!(st.rows(), vec![Row::Group(Check::OffPage), Row::Issue(0)]);
        st.toggle(Check::OffPage);
        assert_eq!(st.rows(), vec![Row::Group(Check::OffPage)]);
        let issue = st.issues[0].clone();
        show_issue(&mut app, &issue);
        assert_eq!(app.selection, vec![id]);
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1000.0))),
            ..Default::default()
        };
        st.settings_open = true;
        for _ in 0..2 {
            let mut out = ctx.run_ui(input.clone(), |ui| {
                let mut close = false;
                dialog(&mut app, ui.ctx(), &mut st, &mut close);
                assert!(!close);
            });
            out.textures_delta.clear();
        }
        assert_eq!(summary(&[]), tr("preflight.none"));
    }
}
