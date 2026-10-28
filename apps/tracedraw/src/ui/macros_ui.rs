//! What VBA macros show: message and input boxes, their user forms, the
//! error dialog, Run Macro (Alt+Shift+F8), the warning before a project's first
//! run, the "still running" notice with Stop, the Macro Manager docker and
//! the Macro Editor (Alt+F11). See docs/behavior/vba.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::macro_forms::{LiveControl, LiveForm};
use crate::macros::Prompt;
use crate::scripting::vba_run::Input;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, Context, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use serde_json::{json, Value};
use tracedraw_vba_project::ControlKind;

/// Points to screen pixels (forms are designed at 96 dots per inch).
const PX: f32 = 4.0 / 3.0;

/// VBA's MsgBox answers.
pub const VB_OK: i32 = 1;
pub const VB_CANCEL: i32 = 2;
pub const VB_ABORT: i32 = 3;
pub const VB_RETRY: i32 = 4;
pub const VB_IGNORE: i32 = 5;
pub const VB_YES: i32 = 6;
pub const VB_NO: i32 = 7;

/// The buttons of a MsgBox style (its low four bits) with their answers.
pub fn msgbox_buttons(style: i32) -> Vec<(&'static str, i32)> {
    match style & 0xF {
        1 => vec![("dialog.ok", VB_OK), ("dialog.cancel", VB_CANCEL)],
        2 => vec![
            ("macros.abort", VB_ABORT),
            ("macros.retry", VB_RETRY),
            ("macros.ignore", VB_IGNORE),
        ],
        3 => vec![
            ("dialog.yes", VB_YES),
            ("dialog.no", VB_NO),
            ("dialog.cancel", VB_CANCEL),
        ],
        4 => vec![("dialog.yes", VB_YES), ("dialog.no", VB_NO)],
        5 => vec![("macros.retry", VB_RETRY), ("dialog.cancel", VB_CANCEL)],
        _ => vec![("dialog.ok", VB_OK)],
    }
}

/// What Esc and the close box answer: Cancel when there is one, OK for
/// a lone OK; `None` when the box must be answered with a button.
pub fn msgbox_escape(style: i32) -> Option<i32> {
    let b = msgbox_buttons(style);
    if b.iter().any(|(_, v)| *v == VB_CANCEL) {
        Some(VB_CANCEL)
    } else if b.len() == 1 {
        Some(VB_OK)
    } else {
        None
    }
}

/// The button Enter picks (vbDefaultButton2 = 256, 3 = 512, 4 = 768).
pub fn msgbox_default(style: i32) -> usize {
    let n = msgbox_buttons(style).len();
    (((style >> 8) & 0xF) as usize).min(n.saturating_sub(1))
}

/// Each frame: the running macro's requests, then what it shows.
pub fn frame(app: &mut App, ctx: &Context) {
    if app.macro_tick() {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
    }
    if app.macros.modal() {
        crate::ui::chrome::modal_blocker(ctx);
    }
    forms(app, ctx);
    prompt(app, ctx);
    confirm(app, ctx);
    run_dialog(app, ctx);
    error_dialog(app, ctx);
    running_notice(app, ctx);
    editor(app, ctx);
}

// ---------------------------------------------------------------- icons

/// MsgBox icons: vbCritical 16, vbQuestion 32, vbExclamation 48,
/// vbInformation 64.
fn paint_icon(ui: &Ui, rect: Rect, style: i32) {
    let p = ui.painter();
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0;
    match style & 0x70 {
        16 => {
            p.circle_filled(c, r, Color32::from_rgb(0xD3, 0x2F, 0x2F));
            let d = r * 0.42;
            let s = Stroke::new(3.0, Color32::WHITE);
            p.line_segment([c + Vec2::new(-d, -d), c + Vec2::new(d, d)], s);
            p.line_segment([c + Vec2::new(d, -d), c + Vec2::new(-d, d)], s);
        }
        48 => {
            let pts = vec![
                c + Vec2::new(0.0, -r),
                c + Vec2::new(r, r * 0.85),
                c + Vec2::new(-r, r * 0.85),
            ];
            p.add(egui::epaint::PathShape::convex_polygon(
                pts,
                Color32::from_rgb(0xF5, 0xB7, 0x00),
                Stroke::new(1.0, Color32::from_rgb(0xB0, 0x80, 0x00)),
            ));
            p.text(
                c + Vec2::new(0.0, r * 0.15),
                egui::Align2::CENTER_CENTER,
                "!",
                crate::theme::bold(r * 1.2),
                Color32::BLACK,
            );
        }
        32 | 64 => {
            p.circle_filled(c, r, Color32::from_rgb(0x1F, 0x6F, 0xC5));
            p.text(
                c + Vec2::new(0.0, 0.5),
                egui::Align2::CENTER_CENTER,
                if style & 0x70 == 32 { "?" } else { "i" },
                crate::theme::bold(r * 1.25),
                Color32::WHITE,
            );
        }
        _ => {}
    }
}

/// A padlock: protected projects and modules saved without code.
pub fn paint_lock(ui: &Ui, rect: Rect, color: Color32) {
    let p = ui.painter();
    let body = Rect::from_center_size(rect.center() + Vec2::new(0.0, 2.0), Vec2::new(9.0, 7.0));
    p.rect_filled(body, 1.0, color);
    let top = body.center_top();
    p.circle_stroke(top, 3.0, Stroke::new(1.5, color));
}

/// A row of buttons; `default` gets the accent. Returns the index picked.
fn buttons(ui: &mut Ui, labels: &[String], default: usize) -> Option<usize> {
    let mut picked = None;
    ui.add_space(8.0);
    let row = Vec2::new(ui.available_width().max(200.0), 28.0);
    ui.allocate_ui_with_layout(
        row,
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            for i in (0..labels.len()).rev() {
                let r = ui
                    .scope(|ui| {
                        if i == default {
                            let w = &mut ui.visuals_mut().widgets;
                            w.inactive.bg_stroke = Stroke::new(1.0, Tokens::ACCENT);
                        }
                        ui.add_sized([88.0, 26.0], egui::Button::new(&labels[i]))
                    })
                    .inner;
                if r.clicked() {
                    picked = Some(i);
                }
            }
        },
    );
    picked
}

// ------------------------------------------------------- message boxes

fn prompt(app: &mut App, ctx: &Context) {
    let Some(p) = &mut app.macros.prompt else {
        return;
    };
    let mut answer = None;
    match p {
        Prompt::MsgBox {
            prompt,
            buttons: style,
            title,
        } => {
            let style = *style;
            let list = msgbox_buttons(style);
            let labels: Vec<String> = list.iter().map(|(k, _)| tr(k)).collect();
            let default = msgbox_default(style);
            let title = if title.is_empty() {
                "TraceDraw".to_string()
            } else {
                title.clone()
            };
            let text = prompt.clone();
            crate::ui::chrome::window(title).show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.horizontal_top(|ui| {
                    if style & 0x70 != 0 {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                        paint_icon(ui, r, style);
                        ui.add_space(8.0);
                    }
                    ui.add(egui::Label::new(text).wrap());
                });
                if let Some(i) = buttons(ui, &labels, default) {
                    answer = Some(list[i].1);
                }
            });
            let (enter, esc) = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Enter),
                    i.key_pressed(egui::Key::Escape),
                )
            });
            if answer.is_none() && enter {
                answer = Some(list[default].1);
            }
            let close = crate::ui::chrome::take_close_request(ctx);
            if answer.is_none() && (esc || close) {
                answer = msgbox_escape(style);
            }
            if let Some(a) = answer {
                app.answer_macro(Input::Answer(a));
            }
        }
        Prompt::InputBox {
            prompt,
            title,
            text,
        } => {
            let title = if title.is_empty() {
                "TraceDraw".to_string()
            } else {
                title.clone()
            };
            let mut done: Option<Option<String>> = None;
            crate::ui::chrome::window(title).show(ctx, |ui| {
                ui.set_max_width(420.0);
                ui.add(egui::Label::new(prompt.as_str()).wrap());
                let r = ui.add(egui::TextEdit::singleline(text).desired_width(380.0));
                if !r.has_focus() && !r.lost_focus() && ui.memory(|m| m.focused().is_none()) {
                    r.request_focus();
                }
                let labels = [tr("dialog.ok"), tr("dialog.cancel")];
                match buttons(ui, &labels, 0) {
                    Some(0) => done = Some(Some(text.clone())),
                    Some(_) => done = Some(None),
                    None => {}
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    done = Some(Some(text.clone()));
                }
            });
            let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            if done.is_none() && (esc || crate::ui::chrome::take_close_request(ctx)) {
                done = Some(None);
            }
            if let Some(d) = done {
                app.answer_macro(Input::Text(d));
            }
        }
    }
}

// ------------------------------------------------------------- errors

fn error_dialog(app: &mut App, ctx: &Context) {
    let Some((e, project)) = app.macros.error.clone() else {
        return;
    };
    let mut pick = None;
    crate::ui::chrome::window(tr("macros.error_title")).show(ctx, |ui| {
        ui.set_max_width(460.0);
        ui.horizontal_top(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
            paint_icon(ui, r, 16);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                if e.number > 0 {
                    ui.strong(trf("macros.runtime_error", &[("n", &e.number.to_string())]));
                } else {
                    ui.strong(tr("macros.error_heading"));
                }
                ui.add(egui::Label::new(e.description.as_str()).wrap());
                if !e.module.is_empty() || e.line > 0 {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(trf(
                            "macros.error_where",
                            &[
                                ("m", &e.module),
                                (
                                    "p",
                                    if e.procedure.is_empty() {
                                        "-"
                                    } else {
                                        &e.procedure
                                    },
                                ),
                                ("l", &e.line.to_string()),
                            ],
                        ))
                        .color(Tokens::text_dim()),
                    );
                }
            });
        });
        let can_debug = project < app.macros.projects.len() && !e.module.is_empty();
        let labels = if can_debug {
            vec![tr("macros.debug"), tr("macros.end")]
        } else {
            vec![tr("macros.end")]
        };
        if let Some(i) = buttons(ui, &labels, labels.len() - 1) {
            pick = Some(if can_debug && i == 0 { 0 } else { 1 });
        }
    });
    let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter));
    if pick.is_none() && (esc || crate::ui::chrome::take_close_request(ctx)) {
        pick = Some(1);
    }
    match pick {
        Some(0) => {
            app.macros.error = None;
            app.edit_macro(project, &e.module, Some(e.line));
        }
        Some(_) => app.macros.error = None,
        None => {}
    }
}

// ---------------------------------------------- warnings before a run

fn confirm(app: &mut App, ctx: &Context) {
    let Some((p, m, n)) = app.macros.confirm.clone() else {
        return;
    };
    let Some(project) = app.macros.projects.get(p) else {
        app.macros.confirm = None;
        return;
    };
    let warnings = project.warnings.clone();
    let key = project.file.display().to_string();
    let mut pick = None;
    crate::ui::chrome::window(tr("macros.warnings_title")).show(ctx, |ui| {
        ui.set_max_width(480.0);
        ui.horizontal_top(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
            paint_icon(ui, r, 48);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(trf("macros.warnings_intro", &[("name", project.name())]))
                        .wrap(),
                );
                for w in &warnings {
                    ui.add(egui::Label::new(format!("- {w}")).wrap());
                }
            });
        });
        pick = buttons(ui, &[tr("macros.run_anyway"), tr("dialog.cancel")], 0);
    });
    if pick.is_none()
        && (ctx.input(|i| i.key_pressed(egui::Key::Escape))
            || crate::ui::chrome::take_close_request(ctx))
    {
        pick = Some(1);
    }
    match pick {
        Some(0) => {
            app.macros.confirm = None;
            app.macros.warned.insert(key);
            app.run_macro(p, &m, &n);
        }
        Some(_) => app.macros.confirm = None,
        None => {}
    }
}

// ----------------------------------------------------------- Run Macro

fn run_dialog(app: &mut App, ctx: &Context) {
    if app.macros.run_dialog.is_none() {
        return;
    }
    let all = crate::macros::all_macros(&app.macros);
    let names: Vec<String> = app
        .macros
        .projects
        .iter()
        .map(|p| p.name().to_string())
        .collect();
    let running = app.macros.run.is_some();
    let mut action = None;
    let st = app.macros.run_dialog.as_mut().expect("checked");
    crate::ui::chrome::window(tr("macros.run_title")).show(ctx, |ui| {
        ui.set_min_width(440.0);
        ui.horizontal(|ui| {
            ui.label(tr("macros.find"));
            let r = ui.add(egui::TextEdit::singleline(&mut st.filter).desired_width(300.0));
            if !st.focused {
                r.request_focus();
                st.focused = true;
            }
        });
        let f = st.filter.to_lowercase();
        let shown: Vec<&(usize, String, String)> = all
            .iter()
            .filter(|(p, m, n)| {
                f.is_empty() || format!("{} {m}.{n}", names[*p]).to_lowercase().contains(&f)
            })
            .collect();
        if st.chosen.as_ref().is_none_or(|c| !shown.contains(&c)) {
            st.chosen = shown.first().map(|s| (*s).clone());
        }
        egui::Frame::new()
            .fill(Tokens::field())
            .stroke(Stroke::new(1.0, Tokens::border()))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(420.0, 260.0));
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        ui.set_min_width(410.0);
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            if shown.is_empty() {
                                ui.label(
                                    RichText::new(tr("macros.none")).color(Tokens::text_dim()),
                                );
                            }
                            for s in &shown {
                                let on = st.chosen.as_ref() == Some(*s);
                                let label = format!("{}  >  {}.{}", names[s.0], s.1, s.2);
                                let r = ui.selectable_label(on, label);
                                if r.clicked() {
                                    st.chosen = Some((*s).clone());
                                }
                                if r.double_clicked() {
                                    action = Some(("run", (*s).clone()));
                                }
                            }
                        });
                    });
            });
        let labels = [tr("macros.run"), tr("macros.edit"), tr("dialog.cancel")];
        match buttons(ui, &labels, 0) {
            Some(0) if !running => action = st.chosen.clone().map(|c| ("run", c)),
            Some(1) => action = st.chosen.clone().map(|c| ("edit", c)),
            Some(2) => action = Some(("cancel", (0, String::new(), String::new()))),
            _ => {}
        }
        if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !running {
            if let Some(c) = st.chosen.clone() {
                action = Some(("run", c));
            }
        }
    });
    if action.is_none()
        && (ctx.input(|i| i.key_pressed(egui::Key::Escape))
            || crate::ui::chrome::take_close_request(ctx))
    {
        action = Some(("cancel", (0, String::new(), String::new())));
    }
    if let Some((what, (p, m, n))) = action {
        app.macros.run_dialog = None;
        match what {
            "run" => app.request_macro(p, &m, &n),
            "edit" => {
                app.edit_macro(p, &m, None);
                if let Some(ed) = &mut app.macros.editor {
                    ed.goto_line = crate::macros::sub_line(&ed.source, &n);
                }
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------- still running

fn running_notice(app: &mut App, ctx: &Context) {
    let Some(run) = &app.macros.run else {
        return;
    };
    if run.started.elapsed() < std::time::Duration::from_millis(800) || app.macros.prompt.is_some()
    {
        return;
    }
    let forms_up = run.forms.lock().map(|f| f.any_visible()).unwrap_or(false);
    let name = run.name.clone();
    let mut stop = false;
    egui::Area::new(egui::Id::new("macro_running"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -40.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if !forms_up {
                        ui.spinner();
                    }
                    ui.label(trf("macros.still_running", &[("name", &name)]));
                    if ui.button(tr("macros.stop")).clicked() {
                        stop = true;
                    }
                });
            });
        });
    if stop {
        app.stop_macro();
    }
}

// --------------------------------------------------------------- forms

/// What the user did on a form this frame: properties to set, events to
/// send, focus moves.
#[derive(Default)]
struct FormActions {
    sets: Vec<(u32, String, &'static str, Value)>,
    events: Vec<(u32, String, &'static str, Vec<Value>)>,
    focus: Vec<(u32, Option<String>)>,
    calls: Vec<(u32, String, &'static str, Vec<Value>)>,
}

impl FormActions {
    fn event(&mut self, form: u32, ctl: &str, ev: &'static str, args: Vec<Value>) {
        self.events.push((form, ctl.to_string(), ev, args));
    }
}

fn color(c: Option<(u8, u8, u8)>, default: Color32) -> Color32 {
    c.map(|(r, g, b)| Color32::from_rgb(r, g, b))
        .unwrap_or(default)
}

/// The VBA key code of an egui key (KeyDown, KeyUp).
fn key_code(k: egui::Key) -> Option<i32> {
    use egui::Key::*;
    let name = k.name();
    Some(match k {
        Enter => 13,
        Escape => 27,
        Tab => 9,
        Backspace => 8,
        Delete => 46,
        Insert => 45,
        Home => 36,
        End => 35,
        PageUp => 33,
        PageDown => 34,
        ArrowLeft => 37,
        ArrowUp => 38,
        ArrowRight => 39,
        ArrowDown => 40,
        Space => 32,
        F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12 => {
            111 + name.trim_start_matches('F').parse::<i32>().ok()?
        }
        _ => {
            let c = name.chars().next()?;
            if name.len() == 1 && (c.is_ascii_alphanumeric()) {
                c.to_ascii_uppercase() as i32
            } else {
                return None;
            }
        }
    })
}

fn shift_mask(m: egui::Modifiers) -> i32 {
    (m.shift as i32) | ((m.ctrl || m.command) as i32) << 1 | (m.alt as i32) << 2
}

/// Where control `i` sits on screen: its container's place plus its own.
fn control_rect(form: &LiveForm, i: usize, origin: Pos2) -> Rect {
    let c = &form.controls[i];
    let mut at = origin;
    let mut p = c.parent;
    let mut own = Vec2::new(c.left, c.top);
    if c.kind == ControlKind::Page {
        own = Vec2::ZERO;
    }
    while let Some(j) = p {
        let pc = &form.controls[j];
        match pc.kind {
            // Pages fill their multi-page under the tabs.
            ControlKind::MultiPage => own += Vec2::new(pc.left, pc.top + 18.0),
            ControlKind::Page => {}
            ControlKind::Frame => own += Vec2::new(pc.left, pc.top + 8.0),
            _ => own += Vec2::new(pc.left, pc.top),
        }
        p = pc.parent;
    }
    at += own * PX;
    let size = if c.kind == ControlKind::Page {
        let mp = c.parent.map(|j| &form.controls[j]);
        mp.map(|m| Vec2::new(m.width, m.height - 18.0))
            .unwrap_or(Vec2::ZERO)
    } else {
        Vec2::new(c.width, c.height)
    };
    Rect::from_min_size(at, size * PX)
}

fn font(c: &LiveControl) -> egui::FontId {
    if c.font.bold {
        crate::theme::bold(c.font.size * PX)
    } else {
        egui::FontId::proportional(c.font.size * PX)
    }
}

fn rich(c: &LiveControl, text: &str) -> RichText {
    let mut t = RichText::new(text).font(font(c));
    if c.font.italic {
        t = t.italics();
    }
    if c.font.underline {
        t = t.underline();
    }
    if let Some((r, g, b)) = c.fore {
        // Dark text on light controls is usual; a system colour reads as
        // black here.
        t = t.color(Color32::from_rgb(r, g, b));
    }
    t
}

fn picture(ui: &Ui, key: String, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let id = egui::Id::new(("macro_picture", &key));
    if let Some(t) = ui.ctx().data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let tex = ui.ctx().load_texture(
        key,
        egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    ui.ctx().data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

fn forms(app: &mut App, ctx: &Context) {
    let Some(run) = &app.macros.run else {
        return;
    };
    let shown: Vec<LiveForm> = match run.forms.lock() {
        Ok(f) => f.list.iter().filter(|f| f.visible).cloned().collect(),
        Err(_) => return,
    };
    let forms = run.forms.clone();
    let blocked = app.macros.prompt.is_some();
    let mut acts = FormActions::default();
    // The last modal form takes the keys; one under it waits.
    let top = shown.iter().rev().find(|f| f.modal).map(|f| f.id);
    for form in &shown {
        let mut open = true;
        let size = Vec2::new(form.width, form.height) * PX;
        let back = color(form.back, Color32::from_rgb(0xF0, 0xF0, 0xF0));
        let enabled = form.enabled && !blocked && top.is_none_or(|t| t == form.id || !form.modal);
        let r = egui::Window::new(RichText::new(&form.caption).color(Color32::BLACK))
            .id(egui::Id::new(("macro_form", form.id)))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .fixed_size(size)
            .default_pos(ctx.content_rect().center() - size / 2.0)
            .open(&mut open)
            .frame(
                egui::Frame::window(&ctx.global_style())
                    .fill(back)
                    .corner_radius(2.0)
                    .inner_margin(egui::Margin::ZERO),
            )
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(color(form.fore, Color32::BLACK));
                ui.visuals_mut().widgets.inactive.weak_bg_fill =
                    Color32::from_rgb(0xE1, 0xE1, 0xE1);
                ui.visuals_mut().extreme_bg_color = Color32::WHITE;
                let (area, bg) = ui.allocate_exact_size(size, Sense::click());
                if bg.clicked() {
                    acts.event(form.id, "", "Click", Vec::new());
                }
                if bg.double_clicked() {
                    acts.event(form.id, "", "DblClick", Vec::new());
                }
                if let Some(b) = &form.picture {
                    if let Some(t) = picture(ui, format!("form{}", form.id), b) {
                        ui.painter().image(
                            t.id(),
                            area,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }
                ui.add_enabled_ui(enabled, |ui| {
                    draw_controls(ui, form, area.min, &mut acts);
                });
                enabled
            });
        let _ = r;
        if !open {
            // The close box asks the macro (QueryClose, vbFormControlMenu).
            acts.event(form.id, "", "QueryClose", vec![json!(0)]);
        }
        // Default and Cancel buttons: Enter and Esc on the top form.
        if Some(form.id) == top || (top.is_none() && enabled) {
            let typing_multi = form.focus.as_ref().is_some_and(|f| {
                form.control(f)
                    .is_some_and(|c| c.kind == ControlKind::TextBox && c.multiline)
            });
            let (enter, esc) = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Enter),
                    i.key_pressed(egui::Key::Escape),
                )
            });
            if enter && !typing_multi && !blocked {
                if let Some(c) = form
                    .controls
                    .iter()
                    .find(|c| c.default && c.enabled && c.visible)
                {
                    acts.event(form.id, &c.name, "Click", Vec::new());
                }
            }
            if esc && !blocked {
                if let Some(c) = form
                    .controls
                    .iter()
                    .find(|c| c.cancel && c.enabled && c.visible)
                {
                    acts.event(form.id, &c.name, "Click", Vec::new());
                }
            }
        }
    }
    if acts.sets.is_empty()
        && acts.events.is_empty()
        && acts.focus.is_empty()
        && acts.calls.is_empty()
    {
        return;
    }
    // Apply what changed, then let the macro handle the events (with the
    // ones the changes fire), in order.
    let mut events: Vec<(u32, String, &'static str, Vec<Value>)> = Vec::new();
    if let Ok(mut f) = forms.lock() {
        for (id, ctl) in acts.focus {
            if let Some(form) = f.get_mut(id) {
                form.focus = ctl;
            }
        }
        for (id, ctl, prop, v) in acts.sets {
            if let Ok(o) = f.set_prop(id, &ctl, prop, &v) {
                for off in o.off {
                    events.push((id, off, "Change", Vec::new()));
                }
                if o.changed {
                    events.push((id, ctl.clone(), "Change", Vec::new()));
                }
                if o.click {
                    events.push((id, ctl.clone(), "Click", Vec::new()));
                }
            }
        }
        for (id, ctl, method, args) in acts.calls {
            let _ = f.call(id, &ctl, method, &args);
            if method == "SetSelected" {
                events.push((id, ctl.clone(), "Change", Vec::new()));
                events.push((id, ctl, "Click", Vec::new()));
            }
        }
    }
    events.extend(acts.events);
    for (id, ctl, ev, args) in events {
        app.macro_form_event(id, &ctl, ev, args);
    }
}

fn draw_controls(ui: &mut Ui, form: &LiveForm, origin: Pos2, acts: &mut FormActions) {
    let id = form.id;
    // Containers first so what sits inside draws over them.
    let mut order: Vec<usize> = (0..form.controls.len()).collect();
    let depth = |mut i: usize| {
        let mut d = 0;
        while let Some(p) = form.controls[i].parent {
            d += 1;
            i = p;
        }
        d
    };
    order.sort_by_key(|i| depth(*i));
    for i in order {
        if !form.shows(i) {
            continue;
        }
        let c = &form.controls[i];
        let rect = control_rect(form, i, origin);
        let name = c.name.as_str();
        let enabled = c.enabled;
        let focus_wanted = form
            .focus
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case(name));
        let resp: Option<egui::Response> = ui
            .add_enabled_ui(enabled, |ui| -> Option<egui::Response> {
                match c.kind {
                    ControlKind::Label => {
                        if let Some(b) = c.back {
                            ui.painter().rect_filled(
                                rect,
                                0.0,
                                color(Some(b), Color32::TRANSPARENT),
                            );
                        }
                        let r = ui.put(
                            rect,
                            egui::Label::new(rich(c, &c.caption))
                                .wrap()
                                .sense(Sense::click()),
                        );
                        if r.clicked() {
                            acts.event(id, name, "Click", Vec::new());
                        }
                        if r.double_clicked() {
                            acts.event(id, name, "DblClick", Vec::new());
                        }
                        Some(r)
                    }
                    ControlKind::TextBox => Some(text_box(ui, form, c, rect, acts)),
                    ControlKind::CommandButton => {
                        let mut b = egui::Button::new(rich(c, &c.caption));
                        if let Some(bg) = c.back {
                            b = b.fill(color(Some(bg), Color32::LIGHT_GRAY));
                        }
                        let r = ui.put(rect, b);
                        if c.default {
                            ui.painter().rect_stroke(
                                rect,
                                2.0,
                                Stroke::new(1.0, Tokens::ACCENT),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if r.clicked() {
                            acts.event(id, name, "Click", Vec::new());
                        }
                        if r.double_clicked() {
                            acts.event(id, name, "DblClick", Vec::new());
                        }
                        Some(r)
                    }
                    ControlKind::CheckBox => {
                        let mut v = c.checked;
                        let r = ui.put(rect, egui::Checkbox::new(&mut v, rich(c, &c.caption)));
                        if r.changed() && !c.locked {
                            acts.sets.push((id, name.into(), "Value", json!(v)));
                        }
                        Some(r)
                    }
                    ControlKind::OptionButton => {
                        let r =
                            ui.put(rect, egui::RadioButton::new(c.checked, rich(c, &c.caption)));
                        if r.clicked() && !c.checked && !c.locked {
                            acts.sets.push((id, name.into(), "Value", json!(true)));
                        } else if r.clicked() {
                            acts.event(id, name, "Click", Vec::new());
                        }
                        Some(r)
                    }
                    ControlKind::ToggleButton => {
                        let r = ui.put(
                            rect,
                            egui::Button::selectable(c.checked, rich(c, &c.caption)),
                        );
                        if r.clicked() && !c.locked {
                            acts.sets
                                .push((id, name.into(), "Value", json!(!c.checked)));
                        }
                        Some(r)
                    }
                    ControlKind::ComboBox => Some(combo_box(ui, form, c, rect, acts)),
                    ControlKind::ListBox => Some(list_box(ui, form, c, rect, acts)),
                    ControlKind::Frame => {
                        let r = rect.shrink2(Vec2::new(0.0, 0.0));
                        let top = r.top() + 6.0 * PX;
                        let frame = Rect::from_min_max(Pos2::new(r.left(), top), r.max);
                        if let Some(b) = c.back {
                            ui.painter().rect_filled(
                                frame,
                                0.0,
                                color(Some(b), Color32::TRANSPARENT),
                            );
                        }
                        ui.painter().rect_stroke(
                            frame,
                            2.0,
                            Stroke::new(1.0, Color32::from_gray(0xA0)),
                            egui::StrokeKind::Inside,
                        );
                        if !c.caption.is_empty() {
                            let g = ui.painter().layout_no_wrap(
                                c.caption.clone(),
                                font(c),
                                color(c.fore, Color32::BLACK),
                            );
                            let at = Pos2::new(r.left() + 8.0, top - g.size().y / 2.0);
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    at - Vec2::new(2.0, 0.0),
                                    g.size() + Vec2::new(4.0, 0.0),
                                ),
                                0.0,
                                color(form.back, Color32::from_rgb(0xF0, 0xF0, 0xF0)),
                            );
                            ui.painter().galley(at, g, Color32::BLACK);
                        }
                        None
                    }
                    ControlKind::MultiPage => {
                        let pages = form.pages_of(i);
                        let tabs: Vec<String> = pages
                            .iter()
                            .map(|j| form.controls[*j].caption.clone())
                            .collect();
                        Some(tabs_row(ui, c, rect, &tabs, acts, id))
                    }
                    ControlKind::TabStrip => Some(tabs_row(ui, c, rect, &c.items, acts, id)),
                    ControlKind::Page => {
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(1.0, Color32::from_gray(0xB0)),
                            egui::StrokeKind::Inside,
                        );
                        None
                    }
                    ControlKind::Image => {
                        if let Some(b) = c.back {
                            ui.painter().rect_filled(
                                rect,
                                0.0,
                                color(Some(b), Color32::TRANSPARENT),
                            );
                        }
                        if let Some(bytes) = &c.picture {
                            if let Some(t) = picture(ui, format!("ctl{id}{name}"), bytes) {
                                let s = t.size_vec2();
                                let fit = (rect.width() / s.x).min(rect.height() / s.y).min(1.0);
                                let r = Rect::from_center_size(rect.center(), s * fit);
                                ui.painter().image(
                                    t.id(),
                                    r,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }
                        }
                        let r = ui.interact(rect, ui.id().with(("img", id, name)), Sense::click());
                        if r.clicked() {
                            acts.event(id, name, "Click", Vec::new());
                        }
                        Some(r)
                    }
                    ControlKind::SpinButton => {
                        let vertical = rect.height() >= rect.width();
                        let (a, b) = if vertical {
                            let (t, bt) = rect.split_top_bottom_at_fraction(0.5);
                            (t, bt)
                        } else {
                            let (l, r) = rect.split_left_right_at_fraction(0.5);
                            (r, l)
                        };
                        let up = ui.put(
                            a,
                            egui::Button::new(if vertical { "▲" } else { "▶" }).small(),
                        );
                        let down = ui.put(
                            b,
                            egui::Button::new(if vertical { "▼" } else { "◀" }).small(),
                        );
                        let step = c.small_change.max(1);
                        if up.clicked() {
                            acts.sets.push((
                                id,
                                name.into(),
                                "Value",
                                json!((c.number + step).min(c.max)),
                            ));
                            acts.event(id, name, "SpinUp", Vec::new());
                        }
                        if down.clicked() {
                            acts.sets.push((
                                id,
                                name.into(),
                                "Value",
                                json!((c.number - step).max(c.min)),
                            ));
                            acts.event(id, name, "SpinDown", Vec::new());
                        }
                        Some(up.union(down))
                    }
                    ControlKind::ScrollBar => {
                        let mut v = c.number;
                        let (lo, hi) = (c.min.min(c.max), c.min.max(c.max));
                        let vertical = rect.height() > rect.width();
                        let mut s = egui::Slider::new(&mut v, lo..=hi).show_value(false);
                        if vertical {
                            s = s.vertical();
                        }
                        let r = ui.put(rect, s);
                        if r.changed() {
                            acts.sets.push((id, name.into(), "Value", json!(v)));
                            acts.event(id, name, "Scroll", Vec::new());
                        }
                        Some(r)
                    }
                    ControlKind::Other(_) => {
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(1.0, Color32::GRAY),
                            egui::StrokeKind::Inside,
                        );
                        None
                    }
                }
            })
            .inner;
        let Some(r) = resp else { continue };
        if !c.tip.is_empty() {
            r.clone().rich_tip(c.tip.as_str());
        }
        // Focus: Enter and Exit, and SetFocus from the macro.
        if r.gained_focus() {
            acts.focus.push((id, Some(name.to_string())));
            acts.event(id, name, "Enter", Vec::new());
        }
        if r.lost_focus() {
            acts.event(id, name, "Exit", Vec::new());
            if form
                .focus
                .as_deref()
                .is_some_and(|f| f.eq_ignore_ascii_case(name))
            {
                acts.focus.push((id, None));
            }
        }
        let asked = egui::Id::new(("macro_focus_asked", id));
        let last: Option<String> = ui.ctx().data(|d| d.get_temp(asked));
        if focus_wanted && !r.has_focus() && last.as_deref() != Some(name) {
            r.request_focus();
            ui.ctx()
                .data_mut(|d| d.insert_temp(asked, name.to_string()));
        }
    }
}

fn text_box(
    ui: &mut Ui,
    form: &LiveForm,
    c: &LiveControl,
    rect: Rect,
    acts: &mut FormActions,
) -> egui::Response {
    let id = form.id;
    let name = c.name.as_str();
    let mut text = c.text.clone();
    let mut te = if c.multiline {
        egui::TextEdit::multiline(&mut text)
    } else {
        egui::TextEdit::singleline(&mut text)
    }
    .font(font(c))
    .desired_width(rect.width() - 6.0)
    .interactive(!c.locked)
    .background_color(color(c.back, Color32::WHITE));
    if let Some(p) = c.password_char {
        let _ = p;
        te = te.password(true);
    }
    if c.max_length > 0 {
        te = te.char_limit(c.max_length as usize);
    }
    let r = ui.put(rect, te);
    if r.has_focus() {
        // KeyDown for keys, KeyPress for the characters typed.
        let events = ui.input(|i| i.events.clone());
        for e in events {
            match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if let Some(k) = key_code(key) {
                        acts.event(
                            id,
                            name,
                            "KeyDown",
                            vec![json!(k), json!(shift_mask(modifiers))],
                        );
                        if key == egui::Key::Enter {
                            acts.event(id, name, "KeyPress", vec![json!(13)]);
                        }
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: false,
                    modifiers,
                    ..
                } => {
                    if let Some(k) = key_code(key) {
                        acts.event(
                            id,
                            name,
                            "KeyUp",
                            vec![json!(k), json!(shift_mask(modifiers))],
                        );
                    }
                }
                egui::Event::Text(t) => {
                    for ch in t.chars() {
                        acts.event(id, name, "KeyPress", vec![json!(ch as u32)]);
                    }
                }
                _ => {}
            }
        }
    }
    let typed = egui::Id::new(("macro_tb_edited", id, name));
    if r.changed() {
        acts.sets.push((id, name.into(), "Text", json!(text)));
        ui.ctx().data_mut(|d| d.insert_temp(typed, true));
    }
    if r.lost_focus()
        && ui
            .ctx()
            .data_mut(|d| d.remove_temp::<bool>(typed))
            .unwrap_or(false)
    {
        acts.event(id, name, "AfterUpdate", Vec::new());
    }
    if r.double_clicked() {
        acts.event(id, name, "DblClick", Vec::new());
    }
    r
}

fn row_text(c: &LiveControl, row: usize) -> String {
    let n = c.column_count.max(1) as usize;
    (0..n)
        .map(|k| c.cell(row, k))
        .collect::<Vec<_>>()
        .join("    ")
}

fn combo_box(
    ui: &mut Ui,
    form: &LiveForm,
    c: &LiveControl,
    rect: Rect,
    acts: &mut FormActions,
) -> egui::Response {
    let id = form.id;
    let name = c.name.as_str();
    let mut pick = None;
    let r = if c.style == 2 {
        // A drop-down list: the rows only.
        let cb = egui::ComboBox::from_id_salt(("macro_combo", id, name))
            .width(rect.width() - 8.0)
            .selected_text(rich(c, &c.text))
            .show_ui(ui, |ui| {
                for row in 0..c.list.len() {
                    if ui
                        .selectable_label(row as i32 == c.list_index, row_text(c, row))
                        .clicked()
                    {
                        pick = Some(row);
                    }
                }
            });
        // Placed by its rect: the combo is drawn where the cursor is, so
        // put it in a child at the rect.
        cb.response
    } else {
        let edit_rect = Rect::from_min_max(rect.min, Pos2::new(rect.right() - 18.0, rect.bottom()));
        let r = text_box(ui, form, c, edit_rect, acts);
        let button = Rect::from_min_max(Pos2::new(rect.right() - 18.0, rect.top()), rect.max);
        let b = ui.put(button, egui::Button::new("▾").small());
        let popup = egui::Id::new(("macro_combo_list", id, name));
        if b.clicked() {
            ui.ctx().data_mut(|d| {
                let open = d.get_temp::<bool>(popup).unwrap_or(false);
                d.insert_temp(popup, !open)
            });
        }
        if ui
            .ctx()
            .data(|d| d.get_temp::<bool>(popup))
            .unwrap_or(false)
        {
            egui::Area::new(popup)
                .order(egui::Order::Tooltip)
                .fixed_pos(Pos2::new(rect.left(), rect.bottom()))
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(rect.width() - 12.0);
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                for row in 0..c.list.len() {
                                    if ui
                                        .selectable_label(
                                            row as i32 == c.list_index,
                                            row_text(c, row),
                                        )
                                        .clicked()
                                    {
                                        pick = Some(row);
                                    }
                                }
                            });
                    });
                });
            if pick.is_some() {
                ui.ctx().data_mut(|d| d.insert_temp(popup, false));
            }
        }
        r.union(b)
    };
    if let Some(row) = pick {
        acts.sets
            .push((id, name.into(), "ListIndex", json!(row as i32)));
    }
    r
}

fn list_box(
    ui: &mut Ui,
    form: &LiveForm,
    c: &LiveControl,
    rect: Rect,
    acts: &mut FormActions,
) -> egui::Response {
    let id = form.id;
    let name = c.name.as_str();
    ui.painter()
        .rect_filled(rect, 0.0, color(c.back, Color32::WHITE));
    ui.painter().rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, Color32::from_gray(0x80)),
        egui::StrokeKind::Inside,
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(2.0)));
    let mut any: Option<egui::Response> = None;
    egui::ScrollArea::vertical()
        .id_salt(("macro_list", id, name))
        .max_height(rect.height() - 4.0)
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in 0..c.list.len() {
                let on = if c.multi_select > 0 {
                    c.selected.get(row).copied().unwrap_or(false)
                } else {
                    row as i32 == c.list_index
                };
                let label = row_text(c, row);
                let text = if c.multi_select > 0 && c.style == 1 {
                    format!("{} {label}", if on { "[x]" } else { "[ ]" })
                } else {
                    label
                };
                let r = ui.add_sized(
                    [rect.width() - 16.0, c.font.size * PX + 4.0],
                    egui::Button::selectable(on, rich(c, &text)),
                );
                if r.clicked() && !c.locked {
                    if c.multi_select > 0 {
                        acts.calls.push((
                            id,
                            name.into(),
                            "SetSelected",
                            vec![json!(row), json!(!on)],
                        ));
                        acts.sets
                            .push((id, name.into(), "ListIndex", json!(row as i32)));
                    } else if !on {
                        acts.sets
                            .push((id, name.into(), "ListIndex", json!(row as i32)));
                    }
                }
                if r.double_clicked() {
                    acts.event(id, name, "DblClick", Vec::new());
                }
                any = Some(match any.take() {
                    Some(a) => a.union(r),
                    None => r,
                });
            }
        });
    any.unwrap_or_else(|| ui.interact(rect, ui.id().with(("lb", id, name)), Sense::click()))
}

fn tabs_row(
    ui: &mut Ui,
    c: &LiveControl,
    rect: Rect,
    tabs: &[String],
    acts: &mut FormActions,
    id: u32,
) -> egui::Response {
    let bar = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 18.0 * PX));
    ui.painter().rect_stroke(
        Rect::from_min_max(Pos2::new(rect.left(), bar.bottom()), rect.max),
        0.0,
        Stroke::new(1.0, Color32::from_gray(0xA0)),
        egui::StrokeKind::Inside,
    );
    let mut x = bar.left();
    let mut resp: Option<egui::Response> = None;
    for (k, t) in tabs.iter().enumerate() {
        let w = ui
            .painter()
            .layout_no_wrap(t.clone(), font(c), Color32::BLACK)
            .size()
            .x
            + 16.0;
        let r = Rect::from_min_size(Pos2::new(x, bar.top()), Vec2::new(w, bar.height()));
        let b = ui.put(
            r,
            egui::Button::selectable(k as i32 == c.number, rich(c, t)),
        );
        if b.clicked() && k as i32 != c.number {
            acts.sets
                .push((id, c.name.clone(), "Value", json!(k as i32)));
        }
        x += w + 2.0;
        resp = Some(match resp.take() {
            Some(a) => a.union(b),
            None => b,
        });
    }
    resp.unwrap_or_else(|| ui.interact(bar, ui.id().with(("tabs", id, &c.name)), Sense::hover()))
}

// ------------------------------------------------------ Macro Manager

/// The Macro Manager docker: the projects as a tree (project, modules,
/// macros) with Run, Edit, Rename, Unload and Export.
pub fn manager(app: &mut App, ui: &mut Ui) {
    app.macros.ensure_loaded();
    let running = app.macros.run.is_some();
    let chosen = app.macros.chosen.clone();
    let chosen_macro = chosen
        .as_ref()
        .and_then(|(p, m, n)| Some((*p, m.clone()?, n.clone()?)));
    let mut act: Option<&str> = None;
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr("macros.load"))
            .rich_tip(tr("macros.load_hint"))
            .clicked()
        {
            act = Some("load");
        }
        if ui
            .add_enabled(
                chosen_macro.is_some() && !running,
                egui::Button::new(tr("macros.run")),
            )
            .clicked()
        {
            act = Some("run");
        }
        if running && ui.button(tr("macros.stop")).clicked() {
            act = Some("stop");
        }
        if ui
            .add_enabled(chosen.is_some(), egui::Button::new(tr("macros.edit")))
            .clicked()
        {
            act = Some("edit");
        }
        if ui
            .add_enabled(chosen.is_some(), egui::Button::new(tr("macros.rename")))
            .clicked()
        {
            act = Some("rename");
        }
        if ui
            .add_enabled(
                chosen.is_some() && !running,
                egui::Button::new(tr("macros.unload")),
            )
            .clicked()
        {
            act = Some("unload");
        }
        if ui
            .add_enabled(chosen.is_some(), egui::Button::new(tr("macros.export")))
            .clicked()
        {
            act = Some("export");
        }
    });
    if let Some((i, text)) = &mut app.macros.renaming {
        let i = *i;
        let mut done = None;
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(text).desired_width(160.0));
            if ui.button(tr("dialog.ok")).clicked()
                || (r.lost_focus() && ui.input(|k| k.key_pressed(egui::Key::Enter)))
            {
                done = Some(true);
            }
            if ui.button(tr("dialog.cancel")).clicked() {
                done = Some(false);
            }
        });
        match done {
            Some(true) => {
                let name = text.clone();
                app.macros.renaming = None;
                if let Err(e) = app.macros.rename(i, &name) {
                    app.status = e;
                }
                app.shortcut_editor.entries.clear();
            }
            Some(false) => app.macros.renaming = None,
            None => {}
        }
    }
    ui.separator();
    if app.macros.projects.is_empty() {
        ui.label(RichText::new(tr("macros.empty")).color(Tokens::text_dim()));
    }
    let mut pick: Option<(usize, Option<String>, Option<String>)> = None;
    let mut run = None;
    egui::ScrollArea::vertical()
        .id_salt("macro_tree")
        .show(ui, |ui| {
            for (pi, p) in app.macros.projects.iter().enumerate() {
                let macros = p.macros();
                let head = egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    ui.id().with(("macro_project", pi)),
                    true,
                );
                head.show_header(ui, |ui| {
                    let on = chosen.as_ref().is_some_and(|c| c.0 == pi && c.1.is_none());
                    if p.project.protected {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        paint_lock(ui, r, Tokens::text_dim());
                    }
                    if ui
                        .add(egui::Button::selectable(
                            on,
                            RichText::new(p.name()).strong(),
                        ))
                        .clicked()
                    {
                        pick = Some((pi, None, None));
                    }
                    if !p.warnings.is_empty() {
                        ui.label(
                            RichText::new("!")
                                .color(Color32::from_rgb(0xC0, 0x80, 0x00))
                                .strong(),
                        )
                        .rich_tip(p.warnings.join("\n"));
                    }
                })
                .body(|ui| {
                    for m in &p.project.modules {
                        let kind = match m.kind {
                            tracedraw_vba_project::ModuleKind::Standard => tr("macros.kind_module"),
                            tracedraw_vba_project::ModuleKind::Class => tr("macros.kind_class"),
                            tracedraw_vba_project::ModuleKind::Form => tr("macros.kind_form"),
                            tracedraw_vba_project::ModuleKind::Document => {
                                tr("macros.kind_document")
                            }
                        };
                        ui.horizontal(|ui| {
                            let on = chosen.as_ref().is_some_and(|c| {
                                c.0 == pi
                                    && c.1.as_deref() == Some(m.name.as_str())
                                    && c.2.is_none()
                            });
                            if !m.has_source {
                                let (r, _) =
                                    ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                paint_lock(ui, r, Tokens::text_dim());
                            }
                            let r = ui
                                .add(egui::Button::selectable(
                                    on,
                                    format!("{}  ({kind})", m.name),
                                ))
                                .rich_tip(if m.has_source {
                                    String::new()
                                } else {
                                    tr("macros.no_source")
                                });
                            if r.clicked() {
                                pick = Some((pi, Some(m.name.clone()), None));
                            }
                        });
                        for (_, n) in macros
                            .iter()
                            .filter(|(mm, _)| mm.eq_ignore_ascii_case(&m.name))
                        {
                            ui.horizontal(|ui| {
                                ui.add_space(18.0);
                                let on = chosen.as_ref().is_some_and(|c| {
                                    c.0 == pi
                                        && c.1.as_deref() == Some(m.name.as_str())
                                        && c.2.as_deref() == Some(n.as_str())
                                });
                                let r = ui.add(egui::Button::selectable(on, format!("▸ {n}")));
                                if r.clicked() {
                                    pick = Some((pi, Some(m.name.clone()), Some(n.clone())));
                                }
                                if r.double_clicked() && !running {
                                    run = Some((pi, m.name.clone(), n.clone()));
                                }
                            });
                        }
                    }
                    for w in &p.warnings {
                        ui.add(
                            egui::Label::new(RichText::new(w).small().color(Tokens::text_dim()))
                                .wrap(),
                        );
                    }
                });
            }
        });
    if let Some(p) = pick {
        app.macros.chosen = Some(p);
    }
    if let Some((p, m, n)) = run {
        app.request_macro(p, &m, &n);
    }
    let Some(a) = act else { return };
    match a {
        "load" => {
            app.load_macro_project();
            app.shortcut_editor.entries.clear();
        }
        "stop" => app.stop_macro(),
        "run" => {
            if let Some((p, m, n)) = chosen_macro {
                app.request_macro(p, &m, &n);
            }
        }
        "edit" => {
            if let Some((p, m, n)) = chosen {
                app.edit_macro(p, m.as_deref().unwrap_or(""), None);
                if let (Some(n), Some(ed)) = (n, &mut app.macros.editor) {
                    ed.goto_line = crate::macros::sub_line(&ed.source, &n);
                }
            }
        }
        "rename" => {
            if let Some((p, _, _)) = chosen {
                let name = app
                    .macros
                    .projects
                    .get(p)
                    .map(|x| x.name().to_string())
                    .unwrap_or_default();
                app.macros.renaming = Some((p, name));
            }
        }
        "unload" => {
            if let Some((p, _, _)) = chosen {
                app.macros.remove(p);
                app.macros.chosen = None;
                if app.macros.editor.as_ref().is_some_and(|e| e.project == p) {
                    app.macros.editor = None;
                }
                app.shortcut_editor.entries.clear();
            }
        }
        "export" => {
            if let Some((p, m, _)) = chosen {
                app.export_macros(p, m.as_deref());
            }
        }
        _ => {}
    }
}

// --------------------------------------------------------- the editor

const KEYWORDS: &[&str] = &[
    "and",
    "as",
    "boolean",
    "byref",
    "byte",
    "byval",
    "call",
    "case",
    "const",
    "currency",
    "date",
    "declare",
    "dim",
    "do",
    "double",
    "each",
    "else",
    "elseif",
    "end",
    "enum",
    "erase",
    "error",
    "exit",
    "explicit",
    "false",
    "for",
    "friend",
    "function",
    "get",
    "global",
    "gosub",
    "goto",
    "if",
    "implements",
    "in",
    "integer",
    "is",
    "let",
    "lib",
    "like",
    "long",
    "longptr",
    "loop",
    "me",
    "mod",
    "new",
    "next",
    "not",
    "nothing",
    "object",
    "on",
    "option",
    "optional",
    "or",
    "paramarray",
    "preserve",
    "private",
    "property",
    "public",
    "redim",
    "resume",
    "return",
    "select",
    "set",
    "single",
    "static",
    "step",
    "stop",
    "string",
    "sub",
    "then",
    "to",
    "true",
    "type",
    "typeof",
    "until",
    "variant",
    "wend",
    "while",
    "with",
    "withevents",
    "xor",
];

/// VBA colouring: keywords blue, comments green, strings dark red.
pub fn highlight(text: &str, size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::monospace(size);
    let fmt = |c: Color32| egui::TextFormat {
        font_id: font.clone(),
        color: c,
        ..Default::default()
    };
    let plain = Tokens::text();
    let kw = Color32::from_rgb(0x1E, 0x5A, 0xC8);
    let comment = Color32::from_rgb(0x2E, 0x8B, 0x3A);
    let string = Color32::from_rgb(0xA3, 0x2A, 0x2A);
    for line in text.split_inclusive('\n') {
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            let ch = b[i] as char;
            if ch == '\'' {
                job.append(&line[i..], 0.0, fmt(comment));
                break;
            }
            if ch == '"' {
                let mut j = i + 1;
                while j < b.len() && b[j] != b'"' && b[j] != b'\n' {
                    j += 1;
                }
                let end = (j + 1).min(b.len());
                job.append(&line[i..end], 0.0, fmt(string));
                i = end;
                continue;
            }
            if ch.is_ascii_alphabetic() || ch == '_' {
                let mut j = i;
                while j < b.len() && ((b[j] as char).is_ascii_alphanumeric() || b[j] == b'_') {
                    j += 1;
                }
                let word = &line[i..j];
                let low = word.to_ascii_lowercase();
                if low == "rem" && line[..i].trim().is_empty() {
                    job.append(&line[i..], 0.0, fmt(comment));
                    break;
                }
                let c = if KEYWORDS.binary_search(&low.as_str()).is_ok() {
                    kw
                } else {
                    plain
                };
                job.append(word, 0.0, fmt(c));
                i = j;
                continue;
            }
            // Anything else up to the next interesting character.
            let mut j = i + ch.len_utf8();
            while j < b.len() {
                let c = b[j] as char;
                if c == '\''
                    || c == '"'
                    || c.is_ascii_alphabetic()
                    || c == '_'
                    || !line.is_char_boundary(j)
                {
                    break;
                }
                j += 1;
            }
            while j < b.len() && !line.is_char_boundary(j) {
                j += 1;
            }
            job.append(&line[i..j], 0.0, fmt(plain));
            i = j;
        }
    }
    job
}

/// Line (1-based) of character index `at` in `text`.
fn line_of(text: &str, at: usize) -> u32 {
    text.chars().take(at).filter(|c| *c == '\n').count() as u32 + 1
}

/// The character index of the start of line `line` (1-based).
fn line_start(text: &str, line: u32) -> usize {
    let mut n = 1;
    for (i, c) in text.chars().enumerate() {
        if n >= line {
            return i;
        }
        if c == '\n' {
            n += 1;
        }
    }
    text.chars().count()
}

fn editor(app: &mut App, ctx: &Context) {
    let Some(ed) = &app.macros.editor else {
        return;
    };
    let pi = ed.project;
    let Some(p) = app.macros.projects.get(pi) else {
        app.macros.editor = None;
        return;
    };
    let title = trf("macros.editor_title", &[("name", p.name())]);
    let modules: Vec<(String, bool)> = p
        .project
        .modules
        .iter()
        .map(|m| (m.name.clone(), m.has_source))
        .collect();
    let protected = p.project.protected;
    let running = app.macros.run.is_some();
    let mut open = true;
    let mut act: Option<&str> = None;
    let mut switch_to: Option<String> = None;
    let ed = app.macros.editor.as_mut().expect("checked");
    egui::Window::new(title)
        .id(egui::Id::new("macro_editor"))
        .open(&mut open)
        .default_size(Vec2::new(860.0, 560.0))
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(ed.dirty, egui::Button::new(tr("macros.save")))
                    .rich_tip("Ctrl+S")
                    .clicked()
                {
                    act = Some("save");
                }
                if ui
                    .add_enabled(!running, egui::Button::new(tr("macros.run_sub")))
                    .rich_tip(tr("macros.run_sub_hint"))
                    .clicked()
                {
                    act = Some("run");
                }
                if running && ui.button(tr("macros.stop")).clicked() {
                    act = Some("stop");
                }
                ui.separator();
                ui.label(tr("macros.find"));
                let r = ui.add(egui::TextEdit::singleline(&mut ed.find).desired_width(160.0));
                if ui.button(tr("macros.find_next")).clicked()
                    || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    act = Some("find");
                }
                if protected {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    paint_lock(ui, r, Tokens::text_dim());
                    ui.label(RichText::new(tr("macros.protected")).color(Tokens::text_dim()));
                }
            });
            if let Some(m) = &ed.message {
                ui.label(RichText::new(m).color(Tokens::text_dim()));
            }
            ui.separator();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    egui::ScrollArea::vertical()
                        .id_salt("macro_modules")
                        .show(ui, |ui| {
                            for (m, has) in &modules {
                                let on = m.eq_ignore_ascii_case(&ed.module);
                                ui.horizontal(|ui| {
                                    if !has {
                                        let (r, _) = ui
                                            .allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                        paint_lock(ui, r, Tokens::text_dim());
                                    }
                                    if ui.add(egui::Button::selectable(on, m)).clicked() && !on {
                                        switch_to = Some(m.clone());
                                    }
                                });
                            }
                        });
                });
                ui.separator();
                code_area(ui, ed);
            });
        });
    // The keys work while the code has the focus.
    let in_code = ctx.memory(|m| m.focused()) == Some(egui::Id::new("macro_code_edit"));
    let (save_keys, f5) = ctx.input(|i| {
        (
            in_code && i.key_pressed(egui::Key::S) && i.modifiers.command,
            in_code && i.key_pressed(egui::Key::F5),
        )
    });
    if save_keys && ed.dirty {
        act = Some("save");
    }
    if f5 && !running {
        act = Some("run");
    }
    if let Some(m) = switch_to {
        if ed.dirty {
            app.save_macro_editor();
        }
        app.edit_macro(pi, &m, None);
        return;
    }
    if !open {
        if ed.dirty {
            app.save_macro_editor();
        }
        app.macros.editor = None;
        return;
    }
    match act {
        Some("save") => app.save_macro_editor(),
        Some("stop") => app.stop_macro(),
        Some("find") => {
            let ed = app.macros.editor.as_mut().expect("open");
            let hay = ed.source.to_lowercase();
            let needle = ed.find.to_lowercase();
            if needle.is_empty() {
                return;
            }
            let start_char = line_start(&ed.source, ed.caret_line + 1);
            let start = hay
                .char_indices()
                .nth(start_char)
                .map(|(b, _)| b)
                .unwrap_or(hay.len());
            let found = hay[start..]
                .find(&needle)
                .map(|b| b + start)
                .or_else(|| hay.find(&needle));
            match found {
                Some(b) => {
                    let at = hay[..b].chars().count();
                    ed.goto_line = Some(line_of(&ed.source, at));
                    ed.message = None;
                }
                None => ed.message = Some(trf("macros.not_found", &[("t", &ed.find)])),
            }
        }
        Some("run") => {
            if app.macros.editor.as_ref().is_some_and(|e| e.dirty) {
                app.save_macro_editor();
            }
            let Some(ed) = &app.macros.editor else { return };
            let (module, line) = (ed.module.clone(), ed.caret_line);
            match crate::macros::sub_at(&ed.source, line) {
                Some(name) => app.request_macro(pi, &module, &name),
                None => {
                    if let Some(ed) = &mut app.macros.editor {
                        ed.message = Some(tr("macros.no_sub_here"));
                    }
                }
            }
        }
        _ => {}
    }
}

const EDITOR_GUTTER: f32 = 44.0;

fn code_area(ui: &mut Ui, ed: &mut crate::macros::Editor) {
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap: f32| {
        let mut job = highlight(text.as_str(), 12.0);
        job.wrap.max_width = wrap;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let goto = ed.goto_line.take();
    egui::ScrollArea::both()
        .id_salt("macro_code")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let id = egui::Id::new("macro_code_edit");
            let out = egui::TextEdit::multiline(&mut ed.source)
                .id(id)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(30)
                .lock_focus(true)
                .layouter(&mut layouter)
                .margin(egui::Margin {
                    left: EDITOR_GUTTER as i8,
                    right: 4,
                    top: 2,
                    bottom: 2,
                })
                .show(ui);
            if out.response.changed() {
                ed.dirty = true;
                ed.message = None;
            }
            if let Some(c) = out.cursor_range {
                ed.caret_line = line_of(&ed.source, c.primary.index.0);
            }
            // Line numbers.
            let rect = out.response.rect;
            let painter = ui.painter_at(rect);
            painter.rect_filled(
                Rect::from_min_max(
                    rect.min,
                    Pos2::new(rect.left() + EDITOR_GUTTER - 6.0, rect.bottom()),
                ),
                0.0,
                Tokens::panel(),
            );
            let mut line = 1u32;
            let mut new_line = true;
            let mut row_of_line = Vec::new();
            for row in &out.galley.rows {
                let r = row.rect().translate(out.galley_pos.to_vec2());
                if new_line {
                    painter.text(
                        Pos2::new(rect.left() + EDITOR_GUTTER - 10.0, r.center().y),
                        egui::Align2::RIGHT_CENTER,
                        line.to_string(),
                        egui::FontId::monospace(10.0),
                        Tokens::text_dim(),
                    );
                    row_of_line.push(r);
                    line += 1;
                }
                new_line = row.ends_with_newline;
            }
            if let Some(l) = goto {
                let at = line_start(&ed.source, l);
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                    let end = at
                        + ed.source
                            .chars()
                            .skip(at)
                            .take_while(|c| *c != '\n' && *c != '\r')
                            .count();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(at),
                            egui::text::CCursor::new(end),
                        )));
                    state.store(ui.ctx(), id);
                }
                ui.memory_mut(|m| m.request_focus(id));
                if let Some(r) = row_of_line.get(l.saturating_sub(1) as usize) {
                    ui.scroll_to_rect(*r, Some(egui::Align::Center));
                    painter.rect_filled(*r, 0.0, Color32::from_rgba_unmultiplied(255, 220, 0, 50));
                }
                ed.caret_line = l;
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_box_buttons_answer_vba_values() {
        // vbYesNoCancel + vbDefaultButton2.
        let b = msgbox_buttons(3 + 256);
        assert_eq!(
            b.iter().map(|x| x.1).collect::<Vec<_>>(),
            vec![VB_YES, VB_NO, VB_CANCEL]
        );
        assert_eq!(msgbox_default(3 + 256), 1);
        assert_eq!(msgbox_escape(3), Some(VB_CANCEL));
        assert_eq!(msgbox_escape(4), None);
        assert_eq!(msgbox_escape(0), Some(VB_OK));
        assert_eq!(msgbox_buttons(2).len(), 3);
        // A default past the last button is the last.
        assert_eq!(msgbox_default(768), 0);
    }

    #[test]
    fn keywords_are_sorted_for_the_search() {
        let mut k = KEYWORDS.to_vec();
        k.sort();
        assert_eq!(k, KEYWORDS);
    }

    #[test]
    fn colouring_keeps_the_text() {
        let src = "Sub A() ' hi\r\n  s = \"x\" & 1 ' é\r\nEnd Sub\r\n";
        let job = highlight(src, 12.0);
        assert_eq!(job.text, src);
    }
}
