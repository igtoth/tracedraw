//! The Text docker (Ctrl+T): Character, Paragraph and Frame sections,
//! each under a header bar with a triangle that opens or closes it.
//!
//! - The three buttons at the top show the docker from that section on
//!   (Character shows all three, Frame only the Frame section).
//! - The two buttons at the right: "Apply to default text" gives new text
//!   the settings of the selected text (the rows ticked, when the boxes
//!   are shown), and the eye shows or hides the boxes at the right of the
//!   rows.
//! - Character: font, style and size; range kerning and underline; the
//!   text fill, the characters' background and the text outline, each a
//!   kind, a colour and "..." for its full settings; the expander shows
//!   strikethrough, the characters' offsets and angle and the OpenType
//!   features.
//! - Paragraph: the six alignments (none, left, centre, right, full and
//!   forced justification), line spacing in % of the character height,
//!   points or % of the font size, left, first-line and right indents,
//!   space before and after paragraphs; the expander shows character and
//!   word spacing. The gear opens the hyphenation settings.
//! - Frame (paragraph text): background colour, vertical alignment, fit
//!   text to frame, columns and the gap between them; the expander shows
//!   bullets, drop caps and hyphenation.
//!
//! Every field edits the selected text: while editing with characters
//! selected, those characters; otherwise every selected text object. With
//! no text selected the font, style, size, underline and alignment become
//! those of new text.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::propbar::{pic_button, Pic};
use egui::{Color32, Rect, Sense, Stroke, Ui, Vec2};
use tracedraw_core::{
    Color, Fill, LeadingUnit, ParagraphStyle, ShapeKind, TextAlign, TextSpan, VerticalAlign,
};

/// The sections, by the names kept in the preferences.
const SECTIONS: [(&str, &str); 3] = [
    ("character", "docker.character"),
    ("paragraph", "docker.paragraph"),
    ("frame", "docker.frame"),
];

/// Width of the row boxes column.
const CHECK_W: f32 = 18.0;

fn is_open(app: &App, id: &str) -> bool {
    !app.settings.text_sections_closed.iter().any(|c| c == id)
}

fn toggle(app: &mut App, id: &str) {
    let closed = &mut app.settings.text_sections_closed;
    if closed.iter().any(|c| c == id) {
        closed.retain(|c| c != id);
    } else {
        closed.push(id.to_string());
    }
}

/// Show the docker from section `i` on (and open it).
fn show_from(app: &mut App, i: usize) {
    app.settings.text_docker_from = i.min(SECTIONS.len() - 1) as u8;
    let id = SECTIONS[i.min(SECTIONS.len() - 1)].0;
    app.settings.text_sections_closed.retain(|c| c != id);
}

fn ticked(app: &App, key: &str) -> bool {
    app.settings.text_docker_ticked.iter().any(|k| k == key)
}

/// The box at the right of a row (when shown): ticked rows are what
/// "Apply to default text" takes.
fn row_check(app: &mut App, ui: &mut Ui, key: &str) {
    if !app.settings.text_docker_checks {
        return;
    }
    let mut on = ticked(app, key);
    if ui
        .add_sized([CHECK_W, 18.0], egui::Checkbox::without_text(&mut on))
        .on_hover_text(tr("docker.text_tick_tip"))
        .changed()
    {
        let list = &mut app.settings.text_docker_ticked;
        list.retain(|k| k != key);
        if on {
            list.push(key.to_string());
        }
    }
}

/// The picture of a section's button.
fn section_picture(p: &egui::Painter, r: Rect, id: &str) {
    match id {
        "character" => {
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "A",
                egui::FontId::proportional(19.0),
                Tokens::icon(),
            );
        }
        "paragraph" => {
            for k in 0..4 {
                let y = r.top() + 3.0 + k as f32 * 4.0;
                p.line_segment(
                    [
                        egui::pos2(r.left() + 2.0, y),
                        egui::pos2(r.right() - 2.0, y),
                    ],
                    Stroke::new(1.4, Tokens::icon()),
                );
            }
        }
        _ => {
            let f = r.shrink(2.5);
            p.rect_stroke(
                f,
                0.0,
                Stroke::new(1.2, Tokens::icon()),
                egui::StrokeKind::Inside,
            );
            for c in [
                f.left_top(),
                f.right_top(),
                f.left_bottom(),
                f.right_bottom(),
            ] {
                p.rect_filled(
                    Rect::from_center_size(c, Vec2::splat(3.5)),
                    0.0,
                    Tokens::icon(),
                );
            }
        }
    }
}

/// The "Apply to default text" picture: a box with an arrow into it.
fn apply_picture(p: &egui::Painter, r: Rect) {
    let s = Stroke::new(1.3, Tokens::icon());
    let b = Rect::from_min_max(
        egui::pos2(r.left() + 2.0, r.top() + 7.0),
        r.right_bottom() - Vec2::new(2.0, 1.0),
    );
    p.rect_stroke(b, 0.0, s, egui::StrokeKind::Inside);
    let x = r.center().x;
    p.line_segment(
        [egui::pos2(x, r.top() + 1.0), egui::pos2(x, r.top() + 11.0)],
        s,
    );
    p.line_segment(
        [
            egui::pos2(x - 3.0, r.top() + 8.0),
            egui::pos2(x, r.top() + 11.0),
        ],
        s,
    );
    p.line_segment(
        [
            egui::pos2(x + 3.0, r.top() + 8.0),
            egui::pos2(x, r.top() + 11.0),
        ],
        s,
    );
}

/// The eye picture of the boxes button.
fn eye_picture(p: &egui::Painter, r: Rect) {
    let s = Stroke::new(1.2, Tokens::icon());
    let page = r.shrink(1.5);
    p.rect_stroke(page, 0.0, s, egui::StrokeKind::Inside);
    let c = egui::pos2(page.right() - 4.0, page.top() + 4.0);
    p.circle_stroke(c, 3.0, s);
    p.circle_filled(c, 1.2, Tokens::icon());
}

/// A small square button with a picture; true when clicked.
fn picture_button(
    ui: &mut Ui,
    pressed: bool,
    tip: &str,
    draw: impl FnOnce(&egui::Painter, Rect),
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 26.0), Sense::click());
    if pressed {
        ui.painter().rect_filled(rect, 2.0, Tokens::tool_active());
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
    }
    draw(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
    );
    resp.on_hover_text(tip).clicked()
}

/// A section's header bar; a click opens or closes it. True when open.
/// `gear` draws a gear at the right that returns its click through the
/// second value.
fn header(app: &mut App, ui: &mut Ui, id: &str, title: &str, gear: bool) -> (bool, bool) {
    let open = is_open(app, id);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click());
    let fill = if resp.hovered() {
        Tokens::tool_hover()
    } else {
        crate::theme::Tokens::control_fill()
    };
    ui.painter().rect_filled(rect, 2.0, fill);
    let c = egui::pos2(rect.left() + 10.0, rect.center().y);
    let pts = if open {
        vec![
            c + Vec2::new(-4.0, 2.0),
            c + Vec2::new(4.0, 2.0),
            c + Vec2::new(0.0, -3.0),
        ]
    } else {
        vec![
            c + Vec2::new(-4.0, -2.0),
            c + Vec2::new(4.0, -2.0),
            c + Vec2::new(0.0, 3.0),
        ]
    };
    ui.painter().add(egui::epaint::PathShape::convex_polygon(
        pts,
        Tokens::icon(),
        Stroke::NONE,
    ));
    ui.painter().text(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        crate::theme::bold(12.5),
        Tokens::text(),
    );
    let mut gear_clicked = false;
    if gear {
        let g = Rect::from_center_size(
            egui::pos2(rect.right() - 14.0, rect.center().y),
            Vec2::splat(18.0),
        );
        let gr = ui.interact(g, ui.id().with((id, "gear")), Sense::click());
        if gr.hovered() {
            ui.painter().rect_filled(g, 2.0, Tokens::tool_active());
        }
        gear_picture(ui.painter(), g.shrink(2.0));
        gear_clicked = gr.on_hover_text(tr("docker.paragraph_settings")).clicked();
    }
    if resp.clicked() && !gear_clicked {
        toggle(app, id);
        return (!open, false);
    }
    (open, gear_clicked)
}

fn gear_picture(p: &egui::Painter, r: Rect) {
    let c = r.center();
    let s = Stroke::new(1.3, Tokens::icon());
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        let d = Vec2::new(a.cos(), a.sin());
        p.line_segment([c + d * 4.5, c + d * 7.0], Stroke::new(2.2, Tokens::icon()));
    }
    p.circle_stroke(c, 4.5, s);
    p.circle_stroke(c, 1.8, s);
}

/// The "more" bar under a section: a triangle that shows or hides its
/// further settings (kept per section for the session).
fn expander(ui: &mut Ui, id: &str) -> bool {
    let key = egui::Id::new(("text_docker_more", id));
    let mut open = ui.data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, Tokens::tool_hover());
    } else {
        ui.painter()
            .rect_filled(rect, 0.0, crate::theme::Tokens::control_fill());
    }
    let c = rect.center();
    let pts = if open {
        vec![
            c + Vec2::new(-4.0, 2.0),
            c + Vec2::new(4.0, 2.0),
            c + Vec2::new(0.0, -2.5),
        ]
    } else {
        vec![
            c + Vec2::new(-4.0, -2.0),
            c + Vec2::new(4.0, -2.0),
            c + Vec2::new(0.0, 2.5),
        ]
    };
    ui.painter().add(egui::epaint::PathShape::convex_polygon(
        pts,
        Tokens::icon(),
        Stroke::NONE,
    ));
    if resp
        .on_hover_text(tr(if open {
            "docker.show_less"
        } else {
            "docker.show_more"
        }))
        .clicked()
    {
        open = !open;
        ui.data_mut(|d| d.insert_temp(key, open));
    }
    open
}

/// A field's label in the grids, without the colon some texts end with,
/// so every label of a section reads alike.
fn label(ui: &mut Ui, key: &str) {
    let text = tr(key);
    ui.label(text.trim_end_matches([':', '\u{FF1A}', ' ']));
}

/// A distance field in the ruler unit, 90 points wide; true when changed.
pub(crate) fn length(ui: &mut Ui, app: &App, mm: &mut f64) -> bool {
    let u = app.units;
    let unit = u.mm();
    let mut v = *mm / unit;
    let d = app.settings.precision.min(6) as usize;
    let r = ui.add_sized(
        [90.0, 22.0],
        NumField::new(&mut v)
            .fixed_decimals(d.min(3))
            .max_decimals(d)
            .suffix(format!(" {}", u.short()))
            .unit_mm(u.mm()),
    );
    if r.changed() && (v * unit - *mm).abs() > 1e-12 {
        *mm = v * unit;
        true
    } else {
        false
    }
}

/// A number field 90 points wide with a suffix, kept in `range`; true when
/// changed.
fn number(
    ui: &mut Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
) -> bool {
    let before = *value;
    ui.add_sized(
        [90.0, 22.0],
        NumField::new(value)
            .range(range)
            .max_decimals(1)
            .suffix(suffix.to_string()),
    );
    (*value - before).abs() > 1e-12
}

/// The picture of a vertical alignment: a frame with three lines at its
/// top, middle or bottom.
fn paint_vertical_align(p: &egui::Painter, r: Rect, v: VerticalAlign, color: Color32) {
    let f = r.shrink(1.0);
    p.rect_stroke(f, 0.0, Stroke::new(1.0, color), egui::StrokeKind::Inside);
    let block = 7.0;
    let top = match v {
        VerticalAlign::Top => f.top() + 2.5,
        VerticalAlign::Center => f.center().y - block / 2.0,
        VerticalAlign::Bottom => f.bottom() - 2.5 - block,
    };
    for k in 0..3 {
        let y = top + k as f32 * 3.0 + 0.5;
        p.line_segment(
            [
                egui::pos2(f.left() + 3.0, y),
                egui::pos2(f.right() - 3.0, y),
            ],
            Stroke::new(1.3, color),
        );
    }
}

/// The style list's entries: regular, italic, bold, bold italic.
const STYLES: [(bool, bool, &str); 4] = [
    (false, false, "docker.style_normal"),
    (false, true, "docker.style_italic"),
    (true, false, "docker.style_bold"),
    (true, true, "docker.style_bold_italic"),
];

/// Outline widths offered, points; 0 = none, the smallest = hairline.
const OUTLINE_WIDTHS_PT: [f64; 9] = [0.0, 0.216, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0];

fn width_name(pt: f64) -> String {
    if pt <= 0.0 {
        tr("docker.no_outline")
    } else if (pt - 0.216).abs() < 1e-3 {
        tr("docker.hairline")
    } else {
        format!("{pt} pt")
    }
}

/// Edit the selected text's spans with `f`: the characters selected while
/// editing, else every span of every selected text object (one undo step).
pub(crate) fn edit_spans(app: &mut App, f: impl Fn(&mut TextSpan) + Clone) {
    // Table cells being edited take the change.
    if app.table_edit.is_some() {
        app.cells_apply_style(f);
        return;
    }
    if app.text_edit.as_ref().is_some_and(|te| te.has_selection()) {
        app.text_apply_style(f);
        return;
    }
    let cmds: Vec<tracedraw_core::Command> = app
        .selected_shapes()
        .iter()
        .filter_map(|s| match &s.kind {
            ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } => {
                let mut spans = spans.clone();
                spans.iter_mut().for_each(f.clone());
                Some(tracedraw_core::Command::SetShapeKind {
                    shape: s.id,
                    kind: ShapeKind::Text {
                        spans,
                        origin: *origin,
                        frame: *frame,
                        align: *align,
                        para: para.clone(),
                        on_path: on_path.clone(),
                    },
                })
            }
            _ => None,
        })
        .collect();
    if !cmds.is_empty() {
        if let Err(e) = app.engine.run_batch("Text", &cmds) {
            app.status = e.to_string();
        }
    }
}

/// Edit the paragraph style of every selected text object with `f`: the
/// paragraphs of the selection while a text is edited, else every
/// paragraph (only the settings `f` changes).
pub(crate) fn edit_paragraphs(app: &mut App, f: impl Fn(&mut ParagraphStyle)) {
    let ids: Vec<_> = app
        .selected_shapes()
        .iter()
        .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
        .map(|s| s.id)
        .collect();
    for id in ids {
        app.edit_paragraph_style(id, &f);
    }
}

/// Set the alignment of the selected text, and of new text.
fn set_align(app: &mut App, a: TextAlign) {
    app.apply_text_align(a);
}

/// "Apply to default text": new text takes the selected text's font,
/// style, size, underline and alignment (only the ticked rows while the
/// boxes are shown), and its fill and outline through the usual question
/// about new objects' defaults.
pub(crate) fn apply_to_default(app: &mut App) {
    let Some(t) = app
        .selected_shapes()
        .into_iter()
        .find(|s| matches!(s.kind, ShapeKind::Text { .. }))
    else {
        return;
    };
    let ShapeKind::Text { spans, .. } = &t.kind else {
        return;
    };
    let Some(sp) = spans.first() else {
        return;
    };
    let align = app
        .paragraph_style_of(&t)
        .map(|(a, _)| a)
        .unwrap_or_default();
    let all = !app.settings.text_docker_checks;
    let take = |key: &str| all || app.settings.text_docker_ticked.iter().any(|k| k == key);
    let (font, style, size, underline, alignment, fill, outline) = (
        take("font"),
        take("style"),
        take("size"),
        take("underline"),
        take("align"),
        take("fill"),
        take("outline"),
    );
    if font {
        app.text_font = sp.font_family.clone();
    }
    if style {
        app.text_bold = sp.bold;
        app.text_italic = sp.italic;
    }
    if size {
        // As seen: new text, not scaled, comes out the same size.
        app.text_size_pt = sp.size_pt * app.text_scale(t.id);
    }
    if underline {
        app.text_underline = sp.underline;
    }
    if alignment {
        app.text_align = align;
    }
    app.status = tr("docker.text_defaults_applied");
    if fill {
        let f = sp.fill.clone().unwrap_or_else(|| t.fill.clone());
        app.request_default_change(crate::defaults::DefaultChange::Fill(f));
    } else if outline {
        app.request_default_change(crate::defaults::DefaultChange::OutlineColor(
            t.stroke.as_ref().map(|s| s.color),
        ));
    }
}

/// Draw the docker.
pub fn text_docker(app: &mut App, ui: &mut Ui) {
    // The active table cell shows as the text it holds.
    let first = app.active_cell_text_shape().or_else(|| {
        app.selected_shapes()
            .into_iter()
            .find(|s| matches!(s.kind, ShapeKind::Text { .. }))
    });
    let from = (app.settings.text_docker_from as usize).min(SECTIONS.len() - 1);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, (id, key)) in SECTIONS.iter().enumerate() {
            if picture_button(ui, i == from, &tr(key), |p, r| section_picture(p, r, id)) {
                show_from(app, i);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let checks = app.settings.text_docker_checks;
            if picture_button(ui, checks, &tr("docker.text_show_ticks"), eye_picture) {
                app.settings.text_docker_checks = !checks;
            }
            if picture_button(ui, false, &tr("docker.text_apply_default"), apply_picture) {
                apply_to_default(app);
            }
        });
    });
    ui.add_space(2.0);
    for (i, (id, key)) in SECTIONS.iter().enumerate().skip(from) {
        let (open, gear) = header(app, ui, id, &tr(key), i == 1);
        if gear {
            app.dialog = crate::ui::dialogs::Dialog::CustomHyphens(Default::default());
        }
        if open {
            ui.add_space(4.0);
            match i {
                0 => character(app, ui, first.as_ref()),
                1 => paragraph(app, ui, first.as_ref()),
                _ => frame(app, ui, first.as_ref()),
            }
            ui.add_space(6.0);
        }
    }
    if first.is_none() {
        ui.label(
            egui::RichText::new(tr("docker.text_hint"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
    }
}

/// A fill row: an icon, the kind (none or uniform), the colour and "...".
/// Returns the fill chosen, when changed.
fn fill_row(
    ui: &mut Ui,
    id: &str,
    icon: &str,
    tip: &str,
    fill: Option<&Fill>,
    more: &mut bool,
) -> Option<Option<Fill>> {
    let mut out = None;
    let color = fill.and_then(|f| f.preview_color());
    ui.label(egui::RichText::new(icon).strong())
        .on_hover_text(tip);
    let uniform = color.is_some();
    let names = [tr("docker.no_fill"), tr("docker.uniform_fill")];
    egui::ComboBox::from_id_salt(id)
        .width(120.0)
        .selected_text(names[usize::from(uniform)].clone())
        .show_ui(ui, |ui| {
            if ui.selectable_label(!uniform, &names[0]).clicked() && uniform {
                out = Some(None);
            }
            if ui.selectable_label(uniform, &names[1]).clicked() && !uniform {
                out = Some(Some(Fill::Solid(Color::BLACK)));
            }
        });
    let mut c = color.unwrap_or(Color::BLACK);
    ui.add_enabled_ui(uniform, |ui| {
        if crate::ui::color_picker::color_button(ui, &mut c) {
            out = Some(Some(Fill::Solid(c)));
        }
    });
    if ui
        .small_button("\u{2022}\u{2022}\u{2022}")
        .on_hover_text(tr("docker.more_settings"))
        .clicked()
    {
        *more = true;
    }
    out
}

/// The style list: the faces the family has (Regular, SemiBold, Bold
/// ...), else regular, italic, bold and bold italic. A face of a weight
/// between is the family named with its weight, so the list shows
/// "SemiBold" for "Titillium Web SemiBold", never Bold (#892). True when
/// the style changed.
fn style_list(app: &mut App, ui: &mut Ui) -> bool {
    let font = app.text_font.clone();
    let base = tracedraw_text::split_weight(&font).map_or(font.as_str(), |(b, _)| b);
    let faces = app
        .font_previews
        .info(base)
        .map(|i| i.faces)
        .unwrap_or_default();
    let now = (font.clone(), app.text_bold, app.text_italic);
    let picks: Vec<(String, (String, bool, bool))> = faces
        .iter()
        .map(|f| (f.name.clone(), tracedraw_text::face_family(base, f)))
        .collect();
    let shown = style_name(&picks, &now);
    let mut changed = false;
    egui::ComboBox::from_id_salt("docker_text_style")
        .width(110.0)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            let mut chosen = None;
            if picks.is_empty() {
                for (b, i, key) in STYLES {
                    let on = app.text_bold == b && app.text_italic == i;
                    if ui.selectable_label(on, tr(key)).clicked() {
                        chosen = Some((font.clone(), b, i));
                    }
                }
            } else {
                for (name, pick) in &picks {
                    if ui.selectable_label(*pick == now, name).clicked() {
                        chosen = Some(pick.clone());
                    }
                }
            }
            if let Some((family, bold, italic)) = chosen {
                if family != app.text_font {
                    app.text_font = family;
                }
                app.text_bold = bold;
                app.text_italic = italic;
                changed = true;
            }
        });
    changed
}

/// The style the list shows for the text settings `now` (family, bold,
/// italic) among a family's faces and the names text gives them: the
/// face's own name, else the weight a family name ends in ("SemiBold"),
/// else regular, italic, bold or bold italic.
fn style_name(picks: &[(String, (String, bool, bool))], now: &(String, bool, bool)) -> String {
    if let Some((name, _)) = picks.iter().find(|(_, p)| p == now) {
        return name.clone();
    }
    let (family, bold, italic) = now;
    if let Some(weight) = tracedraw_text::weight_suffix(family).filter(|_| !*bold) {
        return if *italic {
            format!("{weight} {}", tr("docker.style_italic"))
        } else {
            weight.to_string()
        };
    }
    let idx = STYLES
        .iter()
        .position(|(b, i, _)| b == bold && i == italic)
        .unwrap_or(0);
    tr(STYLES[idx].2)
}

fn character(app: &mut App, ui: &mut Ui, first: Option<&tracedraw_core::Shape>) {
    let span = first.and_then(|t| match &t.kind {
        ShapeKind::Text { spans, .. } => spans.first().cloned(),
        _ => None,
    });
    let mut changed = false;
    ui.horizontal(|ui| {
        let w = ui.available_width()
            - if app.settings.text_docker_checks {
                CHECK_W + 6.0
            } else {
                0.0
            };
        changed |= crate::ui::font_list::text_font_list(app, ui, "docker_text_font", w.max(120.0));
        row_check(app, ui, "font");
    });
    ui.horizontal(|ui| {
        changed |= style_list(app, ui);
        row_check(app, ui, "style");
        changed |= crate::ui::toolbar::text_size_list(app, ui);
        row_check(app, ui, "size");
    });
    if changed {
        app.apply_text_style();
    }
    // Range kerning and underline.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("AV").strong().size(11.0))
            .on_hover_text(tr("docker.tracking"));
        let mut kern = span.as_ref().map_or(0.0, |s| s.tracking_pct);
        let enabled = span.is_some();
        let mut kc = false;
        ui.add_enabled_ui(enabled, |ui| {
            kc = number(ui, &mut kern, -50.0..=200.0, " %");
        });
        if kc {
            edit_spans(app, move |sp| sp.tracking_pct = kern);
        }
        row_check(app, ui, "kerning");
        let on = app.text_underline;
        if ui
            .add_sized(
                [26.0, 22.0],
                egui::Button::selectable(on, egui::RichText::new("U").underline().size(14.0)),
            )
            .on_hover_text(tr("toolbar.underline"))
            .clicked()
        {
            app.text_underline = !on;
            let u = !on;
            if span.is_some() {
                edit_spans(app, move |sp| sp.underline = u);
            }
        }
        row_check(app, ui, "underline");
    });
    // Text fill, background and outline.
    let mut more_fill = false;
    let text_fill = first.map(|t| {
        span.as_ref()
            .and_then(|s| s.fill.clone())
            .unwrap_or_else(|| t.fill.clone())
    });
    ui.horizontal(|ui| {
        if let Some(f) = fill_row(
            ui,
            "docker_text_fill",
            "A",
            &tr("docker.text_fill"),
            text_fill.as_ref(),
            &mut more_fill,
        ) {
            set_text_fill(app, f);
        }
        row_check(app, ui, "fill");
    });
    let mut more_bg = false;
    let bg = span.as_ref().and_then(|s| s.background.clone());
    ui.horizontal(|ui| {
        if let Some(f) = fill_row(
            ui,
            "docker_text_background",
            "ab",
            &tr("docker.text_background"),
            bg.as_ref(),
            &mut more_bg,
        ) {
            edit_spans(app, move |sp| sp.background = f.clone());
        }
        row_check(app, ui, "background");
    });
    if more_fill || more_bg {
        app.open_fill_editor();
    }
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("A").strong().color(Tokens::text_dim()))
            .on_hover_text(tr("docker.text_outline"));
        let stroke = first.and_then(|t| t.stroke.clone());
        let pt = stroke.as_ref().map_or(0.0, |s| s.width * 72.0 / 25.4);
        egui::ComboBox::from_id_salt("docker_text_outline")
            .width(120.0)
            .selected_text(if stroke.is_some() {
                width_name((pt * 1000.0).round() / 1000.0)
            } else {
                width_name(0.0)
            })
            .show_ui(ui, |ui| {
                for w in OUTLINE_WIDTHS_PT {
                    if ui
                        .selectable_label((w - pt).abs() < 1e-3, width_name(w))
                        .clicked()
                    {
                        if w <= 0.0 {
                            app.apply_outline_color(None);
                        } else {
                            app.apply_outline_width(w * 25.4 / 72.0);
                        }
                    }
                }
            });
        let mut c = stroke.as_ref().map_or(Color::BLACK, |s| s.color);
        ui.add_enabled_ui(stroke.is_some(), |ui| {
            if crate::ui::color_picker::color_button(ui, &mut c) {
                app.apply_outline_color(Some(c));
            }
        });
        if ui
            .small_button("\u{2022}\u{2022}\u{2022}")
            .on_hover_text(tr("docker.more_settings"))
            .clicked()
        {
            app.open_outline_editor();
        }
        row_check(app, ui, "outline");
    });
    if !expander(ui, "character") {
        return;
    }
    let Some(mut sp) = span else {
        return;
    };
    let mut sc = false;
    // Character offsets and angle: the selected characters while editing,
    // else every character.
    ui.horizontal_wrapped(|ui| {
        crate::ui::propbar::char_offset_part(app, ui);
    });
    sc |= ui
        .checkbox(&mut sp.strikethrough, tr("docker.strikethrough"))
        .changed();
    // Wrapped, so the feature buttons never widen the docker.
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("docker.opentype"));
        for (tag, k) in [
            ("liga", "docker.ligatures"),
            ("smcp", "docker.small_caps"),
            ("onum", "docker.oldstyle_nums"),
            ("frac", "docker.fractions"),
            ("swsh", "docker.swash"),
        ] {
            let on = sp.features.iter().any(|f| f == tag);
            if ui.selectable_label(on, tr(k)).clicked() {
                if on {
                    sp.features.retain(|f| f != tag);
                } else {
                    sp.features.push(tag.to_string());
                }
                sc = true;
            }
        }
    });
    if sc {
        let (strike, features) = (sp.strikethrough, sp.features.clone());
        edit_spans(app, move |s| {
            s.strikethrough = strike;
            s.features = features.clone();
        });
    }
}

/// The text fill: the selected characters' colour while editing, else the
/// text objects' fill (span colours give way to it).
fn set_text_fill(app: &mut App, fill: Option<Fill>) {
    if app.table_edit.is_some() {
        let f = Some(fill.unwrap_or(Fill::None));
        app.cells_apply_style(move |sp| sp.fill = f.clone());
        return;
    }
    if app.text_selection_active() {
        app.apply_fill(fill.unwrap_or(Fill::None));
        return;
    }
    let has_span_fills = app.selected_shapes().iter().any(|s| match &s.kind {
        ShapeKind::Text { spans, .. } => spans.iter().any(|sp| sp.fill.is_some()),
        _ => false,
    });
    if has_span_fills {
        edit_spans(app, |sp| sp.fill = None);
    }
    app.apply_fill(fill.unwrap_or(Fill::None));
}

fn paragraph(app: &mut App, ui: &mut Ui, first: Option<&tracedraw_core::Shape>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (a, pic, key) in [
            (TextAlign::None, Pic::TextNone, "docker.align_none"),
            (TextAlign::Left, Pic::TextLeft, "docker.align_left_short"),
            (
                TextAlign::Center,
                Pic::TextCenter,
                "docker.align_center_short",
            ),
            (TextAlign::Right, Pic::TextRight, "docker.align_right_short"),
            (
                TextAlign::Justify,
                Pic::TextJustify,
                "docker.align_justify_short",
            ),
            (
                TextAlign::ForceJustify,
                Pic::TextForceJustify,
                "docker.align_force_justify",
            ),
        ] {
            let current = first
                .and_then(|t| app.paragraph_style_of(t))
                .map_or(app.text_align, |(a, _)| a);
            if pic_button(ui, pic, &tr(key), true, current == a).clicked() {
                set_align(app, a);
            }
        }
        row_check(app, ui, "align");
    });
    let Some(t) = first else {
        return;
    };
    // The paragraph at the caret while editing, else the first one.
    let Some((_, mut p)) = app.paragraph_style_of(t) else {
        return;
    };
    let before = p.clone();
    let mut pc = false;
    let line_icon = |ui: &mut Ui, text: &str, key: &str| {
        ui.add_sized(
            [18.0, 22.0],
            egui::Label::new(egui::RichText::new(text).size(13.0)),
        )
        .on_hover_text(tr(key));
    };
    ui.horizontal(|ui| {
        line_icon(ui, "\u{2195}", "docker.leading");
        let (range, suffix) = match p.leading_unit {
            LeadingUnit::Points => (0.0..=2000.0, " pt"),
            _ => (0.0..=2000.0, " %"),
        };
        pc |= number(ui, &mut p.leading_pct, range, suffix);
        let unit_name = |u: LeadingUnit| {
            tr(match u {
                LeadingUnit::PercentCharHeight => "docker.leading_char_height",
                LeadingUnit::Points => "docker.leading_points",
                LeadingUnit::PercentPointSize => "docker.leading_point_size",
            })
        };
        egui::ComboBox::from_id_salt("docker_leading_unit")
            .width(120.0)
            .selected_text(unit_name(p.leading_unit))
            .show_ui(ui, |ui| {
                for u in LeadingUnit::ALL {
                    if ui
                        .selectable_label(p.leading_unit == u, unit_name(u))
                        .clicked()
                        && p.leading_unit != u
                    {
                        // Keep the spacing about the same in the new unit.
                        let size_pt = t_size_pt(t);
                        p.leading_pct = convert_leading(p.leading_pct, p.leading_unit, u, size_pt);
                        p.leading_unit = u;
                        pc = true;
                    }
                }
            });
        row_check(app, ui, "leading");
    });
    egui::Grid::new("text_paragraph")
        .num_columns(4)
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            line_icon(ui, "\u{21E5}", "docker.left_indent");
            pc |= length(ui, app, &mut p.left_indent);
            line_icon(ui, "\u{2912}", "docker.space_before");
            pc |= length(ui, app, &mut p.space_before);
            ui.end_row();
            line_icon(ui, "\u{21B3}", "docker.first_line_indent");
            pc |= length(ui, app, &mut p.first_line_indent);
            line_icon(ui, "\u{2913}", "docker.space_after");
            pc |= length(ui, app, &mut p.space_after);
            ui.end_row();
            line_icon(ui, "\u{21E4}", "docker.right_indent");
            pc |= length(ui, app, &mut p.right_indent);
            ui.end_row();
        });
    if expander(ui, "paragraph") {
        egui::Grid::new("text_paragraph_more")
            .num_columns(2)
            .spacing([6.0, 4.0])
            .show(ui, |ui| {
                label(ui, "docker.char_spacing");
                pc |= number(ui, &mut p.char_spacing_pct, -100.0..=2000.0, " %");
                ui.end_row();
                label(ui, "docker.word_spacing");
                pc |= number(ui, &mut p.word_spacing_pct, 0.0..=2000.0, " %");
                ui.end_row();
            });
    }
    if pc {
        // Only what changed reaches the paragraphs, each keeping the rest.
        let before = tracedraw_core::ParagraphFormat::of(TextAlign::Left, &before);
        let after = tracedraw_core::ParagraphFormat::of(TextAlign::Left, &p);
        edit_paragraphs(app, move |q| {
            let mut f = tracedraw_core::ParagraphFormat::of(TextAlign::Left, q);
            f.take_changes(&before, &after);
            f.apply_to(q);
        });
    }
}

/// The largest font size of a text, points.
fn t_size_pt(t: &tracedraw_core::Shape) -> f64 {
    match &t.kind {
        ShapeKind::Text { spans, .. } => {
            spans.iter().map(|s| s.size_pt).fold(0.0, f64::max).max(1.0)
        }
        _ => 12.0,
    }
}

/// A line spacing value in `from` given in `to`, for text of `size_pt`
/// (the character height taken as 1.2 times the size).
pub(crate) fn convert_leading(v: f64, from: LeadingUnit, to: LeadingUnit, size_pt: f64) -> f64 {
    let pts = match from {
        LeadingUnit::PercentCharHeight => v / 100.0 * size_pt * 1.2,
        LeadingUnit::Points => v,
        LeadingUnit::PercentPointSize => v / 100.0 * size_pt,
    };
    let out = match to {
        LeadingUnit::PercentCharHeight => pts / (size_pt * 1.2) * 100.0,
        LeadingUnit::Points => pts,
        LeadingUnit::PercentPointSize => pts / size_pt * 100.0,
    };
    (out * 10.0).round() / 10.0
}

fn frame(app: &mut App, ui: &mut Ui, first: Option<&tracedraw_core::Shape>) {
    let para_text = first.and_then(|t| match &t.kind {
        ShapeKind::Text { frame: Some(_), .. } => app.paragraph_style_of(t).map(|(_, p)| (t.id, p)),
        _ => None,
    });
    let Some((id, mut p)) = para_text else {
        ui.label(
            egui::RichText::new(tr("docker.frame_hint"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        return;
    };
    let mut pc = false;
    ui.horizontal(|ui| {
        // Background colour of the frame.
        let has = p.frame_fill.is_some();
        let mut c = p
            .frame_fill
            .as_ref()
            .and_then(|f| f.preview_color())
            .unwrap_or(Color::WHITE);
        if ui
            .selectable_label(has, egui::RichText::new("\u{25A3}").size(15.0))
            .on_hover_text(tr("docker.frame_background"))
            .clicked()
        {
            p.frame_fill = if has { None } else { Some(Fill::Solid(c)) };
            pc = true;
        }
        ui.add_enabled_ui(has, |ui| {
            if crate::ui::color_picker::color_button(ui, &mut c) {
                p.frame_fill = Some(Fill::Solid(c));
                pc = true;
            }
        });
        row_check(app, ui, "frame_fill");
        // Vertical alignment, a list.
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 26.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
        }
        paint_vertical_align(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
            p.vertical_align,
            Tokens::icon(),
        );
        let resp = resp.on_hover_text(tr("docker.vertical_align"));
        egui::Popup::menu(&resp)
            .id(egui::Id::new("docker_valign"))
            .show(|ui| {
                for (v, key) in [
                    (VerticalAlign::Top, "docker.valign_top"),
                    (VerticalAlign::Center, "docker.valign_center"),
                    (VerticalAlign::Bottom, "docker.valign_bottom"),
                ] {
                    if ui
                        .selectable_label(p.vertical_align == v, tr(key))
                        .clicked()
                    {
                        p.vertical_align = v;
                        pc = true;
                        ui.close();
                    }
                }
            });
        // Fit the text to the frame.
        if ui
            .selectable_label(p.fit_to_frame, egui::RichText::new("\u{2922}").size(14.0))
            .on_hover_text(tr("menu.text.frame_fit_text"))
            .clicked()
        {
            p.fit_to_frame = !p.fit_to_frame;
            pc = true;
        }
    });
    egui::Grid::new("text_frame")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            label(ui, "dialog.columns");
            let before = p.columns;
            ui.add_sized([90.0, 22.0], NumField::new(&mut p.columns).range(1..=8));
            pc |= p.columns != before;
            ui.end_row();
            label(ui, "docker.gutter");
            ui.add_enabled_ui(p.columns > 1, |ui| {
                pc |= length(ui, app, &mut p.gutter);
            });
            ui.end_row();
        });
    if expander(ui, "frame") {
        ui.horizontal(|ui| {
            if pic_button(ui, Pic::Bullets, &tr("menu.text.bullets"), true, p.bullets).clicked() {
                app.dialog = crate::ui::dialogs::Dialog::TextBullets;
            }
            if pic_button(
                ui,
                Pic::DropCap,
                &tr("menu.text.drop_cap"),
                true,
                p.drop_cap_lines > 1,
            )
            .clicked()
            {
                app.dialog = crate::ui::dialogs::Dialog::TextDropCap;
            }
            if ui.button(tr("menu.text.columns")).clicked() {
                app.dialog = crate::ui::dialogs::Dialog::TextColumns;
            }
        });
        pc |= ui
            .checkbox(&mut p.hyphenate, tr("docker.hyphenate"))
            .changed();
    }
    if pc {
        app.set_paragraph_style(id, p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Point, Size, TextSpan};

    fn draw(app: &mut App) {
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(360.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| text_docker(app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_sections_open_close_and_draw_for_any_text() {
        let mut app = App::headless();
        draw(&mut app);
        for frame in [None, Some(Size::new(60.0, 40.0))] {
            let id = app
                .new_shape(ShapeKind::Text {
                    spans: vec![TextSpan::new("Text", "Arial", 24.0)],
                    origin: Point::new(10.0, 10.0),
                    frame,
                    align: TextAlign::Left,
                    para: ParagraphStyle::default(),
                    on_path: None,
                })
                .expect("text");
            app.select(vec![id]);
            draw(&mut app);
        }
        for (id, _) in SECTIONS {
            assert!(is_open(&app, id));
            toggle(&mut app, id);
            assert!(!is_open(&app, id));
        }
        draw(&mut app);
        assert_eq!(app.settings.text_sections_closed.len(), 3);
    }

    /// The style list names a semibold face by its weight, never Bold
    /// (#892, #624): from the family's faces, and from the family name
    /// when the faces are not known.
    #[test]
    fn the_style_list_shows_a_semibold_face_as_semibold() {
        let face = |name: &str, weight: u16, italic: bool| tracedraw_text::FaceStyle {
            name: name.into(),
            weight,
            italic,
        };
        let faces = [
            face("Regular", 400, false),
            face("SemiBold", 600, false),
            face("Bold", 700, false),
        ];
        let picks: Vec<(String, (String, bool, bool))> = faces
            .iter()
            .map(|f| {
                (
                    f.name.clone(),
                    tracedraw_text::face_family("Titillium Web", f),
                )
            })
            .collect();
        let now = |f: &str, b: bool| (f.to_string(), b, false);
        assert_eq!(
            style_name(&picks, &now("Titillium Web SemiBold", false)),
            "SemiBold"
        );
        assert_eq!(style_name(&picks, &now("Titillium Web", true)), "Bold");
        assert_eq!(style_name(&picks, &now("Titillium Web", false)), "Regular");
        // Faces not known (the family is not installed).
        assert_eq!(
            style_name(&[], &now("Titillium Web SemiBold", false)),
            "SemiBold"
        );
        assert_eq!(
            style_name(&[], &now("Titillium Web", true)),
            tr("docker.style_bold")
        );
        // A .cdr run in the semibold face shows as that face in the docker,
        // with Bold off.
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Text", "Selawik SemiBold", 11.0)],
                origin: Point::new(10.0, 10.0),
                frame: None,
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        app.sync_text_defaults_from(id);
        draw(&mut app);
        assert_eq!(app.text_font, "Selawik SemiBold");
        assert!(!app.text_bold);
    }

    /// The vertical alignment a paragraph text takes is kept with it and
    /// moves its lines.
    #[test]
    fn vertical_alignment_is_kept_with_the_text() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Text", "Arial", 24.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(60.0, 80.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        let p = ParagraphStyle {
            vertical_align: VerticalAlign::Bottom,
            ..ParagraphStyle::default()
        };
        app.set_paragraph_style(id, p);
        let s = app.doc().find_shape(id).cloned().expect("text");
        assert!(matches!(
            s.kind,
            ShapeKind::Text { ref para, .. } if para.vertical_align == VerticalAlign::Bottom
        ));
        // Kept in the native format.
        let json = serde_json::to_string(&s).expect("json");
        assert!(json.contains("\"vertical_align\":\"bottom\""), "{json}");
    }

    /// The section buttons show the docker from that section on; the eye
    /// shows the row boxes; Apply to default text gives new text the
    /// selected text's settings (only the ticked rows with the boxes on).
    #[test]
    fn section_buttons_boxes_and_apply_to_default() {
        let mut app = App::headless();
        let mut sp = TextSpan::new("Text", "Selawik", 30.0);
        sp.bold = true;
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![sp],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(60.0, 40.0)),
                align: TextAlign::ForceJustify,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        for from in 0..3 {
            show_from(&mut app, from);
            app.settings.text_docker_checks = from == 1;
            draw(&mut app);
            assert_eq!(app.settings.text_docker_from as usize, from);
        }
        // Everything, with the boxes hidden.
        app.settings.text_docker_checks = false;
        app.text_font = "Other".into();
        app.text_size_pt = 9.0;
        apply_to_default(&mut app);
        assert_eq!(app.text_font, "Selawik");
        assert_eq!(app.text_size_pt, 30.0);
        assert!(app.text_bold);
        assert_eq!(app.text_align, TextAlign::ForceJustify);
        // Only the ticked rows with the boxes shown.
        app.dialog = crate::ui::dialogs::Dialog::None;
        app.text_font = "Other".into();
        app.text_size_pt = 9.0;
        app.settings.text_docker_checks = true;
        app.settings.text_docker_ticked = vec!["size".into()];
        apply_to_default(&mut app);
        assert_eq!((app.text_font.as_str(), app.text_size_pt), ("Other", 30.0));
    }

    /// The fields edit the selected text: background, alignment and line
    /// spacing in another unit.
    #[test]
    fn fields_edit_the_selected_text() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Text", "Arial", 20.0)],
                origin: Point::new(10.0, 10.0),
                frame: None,
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        let yellow = Fill::Solid(Color::rgb8(255, 255, 0));
        let f = yellow.clone();
        edit_spans(&mut app, move |sp| sp.background = Some(f.clone()));
        set_align(&mut app, TextAlign::None);
        edit_paragraphs(&mut app, |p| {
            p.leading_unit = LeadingUnit::Points;
            p.leading_pct = 24.0;
        });
        let s = app.doc().find_shape(id).cloned().expect("text");
        let ShapeKind::Text {
            spans, align, para, ..
        } = &s.kind
        else {
            panic!("text");
        };
        assert_eq!(spans[0].background, Some(yellow));
        assert_eq!(*align, TextAlign::None);
        assert_eq!(
            (para.leading_unit, para.leading_pct),
            (LeadingUnit::Points, 24.0)
        );
        // 120 % of the character height of 10 pt text is 14.4 pt, or
        // 144 % of the size.
        assert_eq!(
            convert_leading(
                120.0,
                LeadingUnit::PercentCharHeight,
                LeadingUnit::Points,
                10.0
            ),
            14.4
        );
        assert_eq!(
            convert_leading(
                14.4,
                LeadingUnit::Points,
                LeadingUnit::PercentPointSize,
                10.0
            ),
            144.0
        );
    }
}
