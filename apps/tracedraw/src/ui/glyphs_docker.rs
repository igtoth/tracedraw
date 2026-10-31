//! The Glyphs docker: the glyphs most recently inserted, a font list, a
//! list of character ranges (the entire font, scripts, punctuation,
//! numbers, symbols ...) and a grid of the font's glyphs in that range.
//! A small arrow under a glyph that has alternates (OpenType stylistic
//! alternates, swashes, small capitals ...) lists them. A click inserts
//! the glyph at the caret of the text being edited, else as a new
//! artistic text at the page centre, in the docker's font.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Sense, Ui, Vec2};
use std::sync::Arc;

/// The ranges of the filter list: a name key and the code point spans.
pub const RANGES: [(&str, &[(u32, u32)]); 14] = [
    ("glyphs.entire_font", &[(0, 0x10FFFF)]),
    ("glyphs.basic_latin", &[(0x20, 0x7F)]),
    ("glyphs.latin_supplement", &[(0xA0, 0xFF)]),
    ("glyphs.latin_extended", &[(0x100, 0x24F), (0x1E00, 0x1EFF)]),
    ("glyphs.greek", &[(0x370, 0x3FF), (0x1F00, 0x1FFF)]),
    ("glyphs.cyrillic", &[(0x400, 0x52F)]),
    ("glyphs.hebrew", &[(0x590, 0x5FF)]),
    ("glyphs.arabic", &[(0x600, 0x6FF), (0x750, 0x77F)]),
    (
        "glyphs.punctuation",
        &[(0x2000, 0x206F), (0x2E00, 0x2E7F), (0x3000, 0x303F)],
    ),
    (
        "glyphs.numbers",
        &[
            (0x30, 0x39),
            (0x2070, 0x209F),
            (0x2150, 0x218F),
            (0x2460, 0x24FF),
        ],
    ),
    ("glyphs.currency", &[(0x20A0, 0x20CF)]),
    (
        "glyphs.arrows",
        &[(0x2190, 0x21FF), (0x27F0, 0x27FF), (0x2900, 0x297F)],
    ),
    ("glyphs.math", &[(0x2200, 0x22FF), (0x2A00, 0x2AFF)]),
    (
        "glyphs.symbols",
        &[
            (0x2100, 0x214F),
            (0x2300, 0x23FF),
            (0x2500, 0x27BF),
            (0x1F300, 0x1FAFF),
        ],
    ),
];

/// The OpenType features looked at for alternates.
pub const ALTERNATE_FEATURES: [&str; 9] = [
    "salt", "swsh", "smcp", "c2sc", "ss01", "ss02", "ss03", "ss04", "ss05",
];

/// How many glyphs the Recent glyphs box keeps.
const RECENT: usize = 24;

/// A recent glyph's cell, and how many rows of them the Recent glyphs box
/// is tall: the box keeps that height, empty or full, and scrolls when the
/// glyphs take more rows (#1320).
const RECENT_CELL: f32 = 34.0;
const RECENT_ROWS: f32 = 2.0;

/// The grid's cell, at the zoom slider's start, and the slider's range.
pub const CELL: f32 = 44.0;
pub const CELL_RANGE: std::ops::RangeInclusive<f32> = 28.0..=96.0;

/// The inside height of the Recent glyphs box.
pub fn recent_box_height() -> f32 {
    // A cell is its glyph and the strip of its alternates' arrow.
    RECENT_ROWS * (RECENT_CELL + 6.0) + (RECENT_ROWS - 1.0)
}

/// The glyphs of `chars` in range `range`.
pub fn in_range(chars: &[char], range: usize) -> Vec<char> {
    let spans = RANGES.get(range).map_or(RANGES[0].1, |r| r.1);
    chars
        .iter()
        .copied()
        .filter(|c| {
            let cp = *c as u32;
            !c.is_whitespace() && spans.iter().any(|(a, b)| cp >= *a && cp <= *b)
        })
        .collect()
}

/// The glyphs of `chars` a search finds: a code point in hexadecimal
/// (`20AC`, `U+20AC`, `0x20ac`) finds that character and those whose code
/// starts with it; other text finds the characters it holds (#375).
pub fn search(chars: &[char], query: &str) -> Vec<char> {
    let q = query.trim();
    if q.is_empty() {
        return chars.to_vec();
    }
    let stripped = q
        .strip_prefix("U+")
        .or_else(|| q.strip_prefix("u+"))
        .or_else(|| q.strip_prefix("0x"))
        .or_else(|| q.strip_prefix("0X"));
    let hex = stripped.unwrap_or(q);
    let all_hex = !hex.is_empty() && hex.len() <= 6 && hex.chars().all(|c| c.is_ascii_hexdigit());
    // A prefix always means a code point; bare text means one when it has
    // two or more hex digits, at least one a digit ("20AC", "004"), so a
    // typed letter or word ("A", "abc") finds those characters.
    let by_code = all_hex
        && (stripped.is_some() || (hex.len() >= 2 && hex.chars().any(|c| c.is_ascii_digit())));
    if by_code {
        let hex = hex.to_ascii_uppercase();
        return chars
            .iter()
            .copied()
            .filter(|c| format!("{:04X}", *c as u32).starts_with(&hex))
            .collect();
    }
    let wanted: Vec<char> = q.chars().filter(|c| !c.is_whitespace()).collect();
    chars
        .iter()
        .copied()
        .filter(|c| wanted.contains(c))
        .collect()
}

/// The character code page 1252 has at `b`.
pub fn from_cp1252(b: u8) -> Option<char> {
    match b {
        // The five places the page leaves empty are read as the control
        // characters of the same number, as fonts with a script at the
        // page's places map them (#1358).
        0x80..=0x9F => tracedraw_text::CP1252_HIGH
            .get((b - 0x80) as usize)
            .copied(),
        _ => Some(b as char),
    }
}

/// The keystroke that types `ch` (Alt and its code page 1252 number with
/// a leading zero, as the reference program shows it), when it has one.
pub fn keystroke(ch: char) -> Option<String> {
    let cp = ch as u32;
    let b = if cp < 0x80 || (0xA0..=0xFF).contains(&cp) {
        Some(cp as u8)
    } else {
        tracedraw_text::CP1252_HIGH
            .iter()
            .position(|c| *c == ch)
            .map(|i| 0x80 + i as u8)
    }?;
    (b >= 0x20).then(|| format!("Alt+{:04}", b))
}

/// What the docker says of a glyph: its code point, the character, the
/// range it belongs to and the keystroke that types it (#447).
pub fn describe(ch: char) -> String {
    let cp = ch as u32;
    let range = RANGES
        .iter()
        .skip(1)
        .find(|(_, spans)| spans.iter().any(|(a, b)| cp >= *a && cp <= *b))
        .map(|(key, _)| tr(key));
    let mut out = format!("U+{cp:04X}   {ch}");
    if let Some(r) = range {
        out.push_str(&format!("   {r}"));
    }
    if let Some(k) = keystroke(ch) {
        out.push_str(&format!(
            "   {}",
            crate::i18n::trf("glyphs.keystroke", &[("keys", &k)])
        ));
    }
    out
}

/// The characters `family` draws, cached for the session.
fn coverage(ui: &Ui, family: &str) -> Arc<Vec<char>> {
    let key = egui::Id::new(("glyph_coverage", family.to_string()));
    if let Some(c) = ui.data(|d| d.get_temp::<Arc<Vec<char>>>(key)) {
        return c;
    }
    let c = Arc::new(tracedraw_text::fonts().char_coverage(family));
    ui.data_mut(|d| d.insert_temp(key, c.clone()));
    c
}

/// The alternates of `ch` in `family`, cached for the session.
fn variants(ui: &Ui, family: &str, ch: char) -> Arc<Vec<String>> {
    let key = egui::Id::new(("glyph_variants", family.to_string(), ch));
    if let Some(v) = ui.data(|d| d.get_temp::<Arc<Vec<String>>>(key)) {
        return v;
    }
    let v = Arc::new(tracedraw_text::fonts().glyph_variants(family, ch, &ALTERNATE_FEATURES));
    ui.data_mut(|d| d.insert_temp(key, v.clone()));
    v
}

/// Insert `ch` in `font` (with the OpenType `feature` when given): at the
/// caret of the text being edited, else as a new artistic text at the page
/// centre. It goes to the front of the recent glyphs.
pub fn insert_glyph(app: &mut App, font: &str, ch: char, feature: Option<&str>) {
    let style = |sp: &mut tracedraw_core::TextSpan, font: &str, feature: Option<&str>| {
        sp.font_family = font.to_string();
        if let Some(f) = feature {
            if !sp.features.iter().any(|x| x == f) {
                sp.features.push(f.to_string());
            }
        }
    };
    if app.text_edit.is_some() {
        app.insert_text(&ch.to_string());
        if let Some(te) = app.text_edit.as_mut() {
            te.anchor = te.caret.saturating_sub(1);
        }
        let (f, feat) = (font.to_string(), feature.map(str::to_string));
        app.text_apply_style(move |sp| style(sp, &f, feat.as_deref()));
        if let Some(te) = app.text_edit.as_mut() {
            te.anchor = te.caret;
        }
    } else {
        let before = std::mem::replace(&mut app.text_font, font.to_string());
        let c = app.page_rect().center();
        app.start_text(c, None);
        app.text_insert(&ch.to_string());
        if let Some(f) = feature {
            if let Some(te) = app.text_edit.as_mut() {
                te.anchor = 0;
            }
            let (fo, fe) = (font.to_string(), f.to_string());
            app.text_apply_style(move |sp| style(sp, &fo, Some(&fe)));
        }
        app.finish_text();
        app.text_font = before;
    }
    let recent = &mut app.settings.recent_glyphs;
    recent.retain(|(f, c)| !(f == font && c.starts_with(ch)));
    recent.insert(0, (font.to_string(), ch.to_string()));
    recent.truncate(RECENT);
}

/// Put `ch` on the system clipboard as text.
fn copy_glyph(ch: char) {
    if !crate::clipboard::write_text(&ch.to_string()) {
        log::warn!("the glyph could not go on the clipboard");
    }
}

/// A glyph cell: the glyph large, and a small arrow under it when it has
/// alternates. Returns (clicked, arrow response).
fn cell(
    ui: &mut Ui,
    pics: &mut crate::ui::font_list::FontPreviews,
    ch: char,
    family: &str,
    size: f32,
    has_alternates: bool,
    picked: bool,
) -> (egui::Response, Option<egui::Response>) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(size, size + 6.0), Sense::click());
    if picked {
        ui.painter()
            .rect_filled(rect, 2.0, Tokens::selected_row().gamma_multiply(0.35));
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, Tokens::selected_row_line()),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
    }
    let ppp = ui.ctx().pixels_per_point();
    let em = size * 0.6;
    match pics.picture(ui.ctx(), family, &ch.to_string(), (false, false), em * ppp) {
        Some(p) => {
            let sz = p.texture.size_vec2() / ppp;
            let base_y = rect.top() + size * 0.72;
            let r = egui::Rect::from_min_size(
                egui::pos2(rect.center().x - sz.x / 2.0, base_y - p.baseline_px / ppp),
                sz,
            );
            ui.painter().image(
                p.texture.id(),
                r,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Tokens::text(),
            );
        }
        None => {
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + size / 2.0),
                egui::Align2::CENTER_CENTER,
                ch,
                egui::FontId::proportional(em),
                Tokens::text(),
            );
        }
    }
    let arrow = has_alternates.then(|| {
        let a = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.bottom() - 3.0),
            Vec2::new(12.0, 6.0),
        );
        let c = a.center();
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + Vec2::new(-3.0, -1.5),
                c + Vec2::new(3.0, -1.5),
                c + Vec2::new(0.0, 2.0),
            ],
            Tokens::text_dim(),
            egui::Stroke::NONE,
        ));
        ui.interact(a, resp.id.with("alt"), Sense::click())
    });
    (resp.rich_tip(format!("{ch}  U+{:04X}", ch as u32)), arrow)
}

/// Draw the docker.
pub fn glyphs(app: &mut App, ui: &mut Ui) {
    if app.glyph_font.is_empty() {
        app.glyph_font = app.text_font.clone();
    }
    let mut insert: Option<(String, char, Option<String>)> = None;
    ui.label(tr("glyphs.recent"));
    // The theme's field colour (white in the light themes, dark in the
    // dark ones), so the cells, tinted with the text colour, stay readable
    // (#660). The box is two rows of glyphs tall, empty too, so the font,
    // the ranges and the grid show under it at once (#1320).
    let box_h = recent_box_height();
    egui::Frame::new()
        .fill(Tokens::field())
        .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if app.settings.recent_glyphs.is_empty() {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), box_h), Sense::hover());
                ui.put(
                    rect,
                    egui::Label::new(
                        egui::RichText::new(tr("glyphs.recent_hint"))
                            .italics()
                            .color(Tokens::text_dim()),
                    ),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("glyphs_recent")
                    .min_scrolled_height(box_h)
                    .max_height(box_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(1.0);
                            for (font, c) in app.settings.recent_glyphs.clone() {
                                let Some(ch) = c.chars().next() else {
                                    continue;
                                };
                                let (r, _) = cell(
                                    ui,
                                    &mut app.font_previews,
                                    ch,
                                    &font,
                                    RECENT_CELL,
                                    false,
                                    false,
                                );
                                if r.clicked() {
                                    insert = Some((font.clone(), ch, None));
                                }
                            }
                        });
                    });
            }
        });
    ui.add_space(4.0);
    // The font list of the property bar, with its search as you type and
    // its previews (#447).
    let current = app.glyph_font.clone();
    let width = ui.available_width() - 8.0;
    let (pick, _) = crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("toolbar.font"), |ui| {
        crate::ui::font_list::font_list_live(app, ui, "glyphs_font", &current, width)
    });
    if let Some(p) = pick {
        app.glyph_font = p.family;
    }
    egui::ComboBox::from_id_salt("glyphs_range")
        .width(ui.available_width() - 8.0)
        .selected_text(tr(RANGES[app.glyph_range.min(RANGES.len() - 1)].0))
        .show_ui(ui, |ui| {
            for (i, (key, _)) in RANGES.iter().enumerate() {
                ui.selectable_value(&mut app.glyph_range, i, tr(key));
            }
        })
        .response
        .rich_tip(tr(RANGES[app.glyph_range.min(RANGES.len() - 1)].0));
    ui.add(
        egui::TextEdit::singleline(&mut app.glyph_search)
            .hint_text(tr("glyphs.search_hint"))
            .desired_width(ui.available_width() - 8.0),
    )
    .rich_tip(tr("glyphs.search_hint"));
    ui.add_space(4.0);
    let family = app.glyph_font.clone();
    let chars = search(
        &in_range(&coverage(ui, &family), app.glyph_range),
        &app.glyph_search,
    );
    let info_id = egui::Id::new("glyphs_info");
    let mut hovered: Option<char> = None;
    // The glyph last under the pointer: its code point, the character and
    // its range (above the grid, which takes the rest of the panel).
    let info = ui.data(|d| d.get_temp::<char>(info_id)).map(describe);
    ui.label(
        egui::RichText::new(info.unwrap_or_default())
            .monospace()
            .color(Tokens::text_dim()),
    );
    if chars.is_empty() {
        ui.label(egui::RichText::new(tr("glyphs.none")).color(Tokens::text_dim()));
    }
    // The grid takes the panel's height down to the zoom slider and the
    // Copy button under it.
    let size = app.glyph_size.clamp(*CELL_RANGE.start(), *CELL_RANGE.end());
    let per_row = ((ui.available_width() / (size + 1.0)).floor() as usize).max(1);
    let rows = chars.len().div_ceil(per_row);
    let sp = ui.spacing().clone();
    let bottom = 2.0 * sp.interact_size.y.max(20.0) + 3.0 * sp.item_spacing.y + 8.0;
    let grid_h = (ui.available_height() - bottom).max(size + 7.0);
    let picked = app.glyph_picked;
    egui::ScrollArea::vertical()
        .id_salt("glyphs_grid")
        .max_height(grid_h)
        .min_scrolled_height(grid_h)
        .auto_shrink([false, false])
        .show_rows(ui, size + 7.0, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(1.0);
                    for &ch in chars.iter().skip(row * per_row).take(per_row) {
                        let alts = variants(ui, &family, ch);
                        let (r, arrow) = cell(
                            ui,
                            &mut app.font_previews,
                            ch,
                            &family,
                            size,
                            !alts.is_empty(),
                            picked == Some(ch),
                        );
                        if r.hovered() {
                            hovered = Some(ch);
                        }
                        if r.clicked() {
                            insert = Some((family.clone(), ch, None));
                            app.glyph_picked = Some(ch);
                        }
                        if let Some(a) = arrow {
                            egui::Popup::menu(&a)
                                .id(egui::Id::new(("glyph_alts", ch as u32)))
                                .show(|ui| {
                                    for f in alts.iter() {
                                        if ui
                                            .button(format!(
                                                "{ch}  {}",
                                                tr(&format!("glyphs.feature_{f}"))
                                            ))
                                            .clicked()
                                        {
                                            insert = Some((family.clone(), ch, Some(f.clone())));
                                            ui.close();
                                        }
                                    }
                                });
                        }
                    }
                });
            }
        });
    // Under the grid: its zoom (a small and a large A at the slider's
    // ends) and Copy, which puts the glyph last clicked on the clipboard
    // as text.
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let a = |ui: &mut Ui, size: f32| {
            ui.label(
                egui::RichText::new("A")
                    .size(size)
                    .color(Tokens::text_dim()),
            )
            .rich_tip(tr("glyphs.size"))
        };
        a(ui, 10.0);
        let w = (ui.available_width() - 24.0).max(40.0);
        ui.add(
            crate::ui::Slider::new(&mut app.glyph_size, CELL_RANGE)
                .show_value(false)
                .ticks(false)
                .width(w),
        )
        .rich_tip(tr("glyphs.size"));
        a(ui, 17.0);
    });
    let copy = ui.add_enabled(
        app.glyph_picked.is_some(),
        egui::Button::new(tr("menu.edit.copy"))
            .min_size(Vec2::new(ui.available_width(), sp.interact_size.y)),
    );
    if copy.clicked() {
        if let Some(ch) = app.glyph_picked {
            copy_glyph(ch);
        }
    }
    if let Some(h) = hovered {
        ui.data_mut(|d| d.insert_temp(info_id, h));
    }
    if let Some((font, ch, feature)) = insert {
        insert_glyph(app, &font, ch, feature.as_deref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::document::ShapeKind;

    #[test]
    fn a_search_finds_characters_and_code_points() {
        let chars: Vec<char> = "AaBé€".chars().collect();
        assert_eq!(search(&chars, ""), chars);
        assert_eq!(search(&chars, "é"), vec!['é']);
        assert_eq!(search(&chars, "U+20AC"), vec!['€']);
        assert_eq!(search(&chars, "0x20ac"), vec!['€']);
        // A hexadecimal start finds every code beginning with it.
        assert_eq!(search(&chars, "004"), vec!['A', 'B']);
        // A bare letter or word finds those characters, not codes.
        assert_eq!(search(&chars, "aB"), vec!['a', 'B']);
        assert!(describe('€').starts_with("U+20AC"));
        // The keystroke of the code page the reference program shows
        // (#447), and back.
        assert_eq!(keystroke('©').as_deref(), Some("Alt+0169"));
        assert_eq!(keystroke('€').as_deref(), Some("Alt+0128"));
        assert_eq!(keystroke('A').as_deref(), Some("Alt+0065"));
        assert_eq!(keystroke('\u{2192}'), None);
        assert_eq!(from_cp1252(0x80), Some('€'));
        assert_eq!(from_cp1252(169), Some('©'));
        assert!(describe('©').contains("Alt+0169"));
        // The places the page leaves empty (#1358).
        assert_eq!(from_cp1252(0x81), Some('\u{81}'));
        assert_eq!(from_cp1252(0x9D), Some('\u{9D}'));
        assert_eq!(keystroke('\u{8D}').as_deref(), Some("Alt+0141"));
    }

    #[test]
    fn ranges_pick_their_characters() {
        let chars = ['A', 'a', '1', '\u{3B1}', '\u{416}', '\u{20AC}', '\u{2192}'];
        assert_eq!(in_range(&chars, 0).len(), chars.len());
        assert_eq!(in_range(&chars, 1), vec!['A', 'a', '1']);
        assert_eq!(in_range(&chars, 4), vec!['\u{3B1}']);
        assert_eq!(in_range(&chars, 5), vec!['\u{416}']);
        assert_eq!(in_range(&chars, 10), vec!['\u{20AC}']);
        assert_eq!(in_range(&chars, 11), vec!['\u{2192}']);
    }

    #[test]
    fn a_glyph_goes_in_as_new_text_in_the_dockers_font_and_is_remembered() {
        let mut app = App::headless();
        app.text_font = "Arial".into();
        insert_glyph(&mut app, "Selawik", '\u{20AC}', None);
        let s = app.selected_shapes().into_iter().next().expect("new text");
        let ShapeKind::Text { spans, .. } = &s.kind else {
            panic!("text");
        };
        assert_eq!(spans[0].text, "\u{20AC}");
        assert_eq!(spans[0].font_family, "Selawik");
        assert_eq!(app.text_font, "Arial");
        assert_eq!(
            app.settings.recent_glyphs[0],
            ("Selawik".to_string(), "\u{20AC}".to_string())
        );
        insert_glyph(&mut app, "Selawik", 'x', Some("salt"));
        assert_eq!(app.settings.recent_glyphs.len(), 2);
        let s = app.selected_shapes().into_iter().next().expect("new text");
        let ShapeKind::Text { spans, .. } = &s.kind else {
            panic!("text");
        };
        assert!(spans[0].features.iter().any(|f| f == "salt"));
    }

    /// The recent glyphs are tinted with the theme's text colour, so their
    /// box takes the theme's field colour, not a fixed white (#660). The
    /// theme is shared by the tests running alongside, so this one draws in
    /// whatever theme is set; `theme_tests` checks that the field and text
    /// colours of every theme contrast.
    #[test]
    fn the_recent_glyphs_box_takes_the_theme_field_colour() {
        let mut app = App::headless();
        app.settings.recent_glyphs = vec![("Arial".into(), "A".into())];
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(360.0, 800.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| glyphs(&mut app, ui));
        out.textures_delta.clear();
        // The recent glyph as drawn (a picture of it, tinted, or the
        // character itself), and the framed box behind it.
        let (at, ink) = out
            .shapes
            .iter()
            .find_map(|c| match &c.shape {
                egui::Shape::Mesh(m) if m.texture_id != egui::TextureId::default() => m
                    .vertices
                    .first()
                    .map(|v| (m.calc_bounds().center(), v.color)),
                egui::Shape::Text(t) if t.galley.text() == "A" => {
                    Some((t.visual_bounding_rect().center(), t.fallback_color))
                }
                _ => None,
            })
            .expect("the recent glyph");
        let fill = out
            .shapes
            .iter()
            .find_map(|c| match &c.shape {
                egui::Shape::Rect(r) if r.stroke.width > 0.0 && r.rect.contains(at) => Some(r.fill),
                _ => None,
            })
            .expect("the recent box");
        assert_eq!(fill, Tokens::field());
        assert_eq!(ink, Tokens::text());
    }

    /// One frame of the docker in a panel 300 x 900; the shapes drawn.
    fn draw(app: &mut App, events: Vec<egui::Event>) -> Vec<egui::epaint::ClippedShape> {
        let ctx = crate::theme::ui_context();
        let mut shapes = Vec::new();
        for i in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(300.0, 900.0),
                )),
                events: if i == 1 { events.clone() } else { Vec::new() },
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| glyphs(app, ui));
            out.textures_delta.clear();
            shapes = out.shapes;
        }
        shapes
    }

    /// The Recent glyphs box: the topmost framed box in the field colour.
    fn recent_box(shapes: &[egui::epaint::ClippedShape]) -> egui::Rect {
        shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Rect(r) if r.stroke.width > 0.0 && r.fill == Tokens::field() => {
                    Some(r.rect)
                }
                _ => None,
            })
            .min_by(|a, b| a.top().total_cmp(&b.top()))
            .expect("the recent box")
    }

    /// #1320: with no glyph used yet the Recent glyphs box is two rows of
    /// glyphs tall, not the whole panel, so the font list and the grid
    /// show under it at once; full, it keeps that height.
    #[test]
    fn the_recent_box_is_two_rows_tall_empty_or_full() {
        let mut app = App::headless();
        app.settings.recent_glyphs.clear();
        let empty = recent_box(&draw(&mut app, Vec::new()));
        // The rows, the 4-point margins and the 1-point frame.
        let want = recent_box_height() + 10.0;
        assert!((empty.height() - want).abs() < 1.0, "{empty:?}");
        assert!(empty.bottom() < 150.0, "{empty:?}");
        app.settings.recent_glyphs = (0..RECENT as u32)
            .map(|i| {
                (
                    "Arial".to_string(),
                    char::from_u32(0x41 + i).unwrap_or('A').to_string(),
                )
            })
            .collect();
        let full = recent_box(&draw(&mut app, Vec::new()));
        assert!((full.height() - want).abs() < 1.0, "{full:?}");
    }

    /// #1320: the grid fills the panel down to the zoom slider and the
    /// Copy button, which copies the glyph last clicked and waits for one.
    #[test]
    fn the_grid_ends_above_the_zoom_and_copy() {
        let mut app = App::headless();
        app.settings.recent_glyphs.clear();
        let shapes = draw(&mut app, Vec::new());
        let copy = tr("menu.edit.copy");
        let copy_at = shapes
            .iter()
            .find_map(|c| match &c.shape {
                egui::Shape::Text(t) if t.galley.text() == copy => Some(t.pos.y),
                _ => None,
            })
            .expect("the Copy button");
        assert!(copy_at > 820.0 && copy_at < 900.0, "{copy_at}");
        assert_eq!(app.glyph_picked, None);
    }

    #[test]
    fn the_docker_draws() {
        let mut app = App::headless();
        app.settings.recent_glyphs = vec![("Arial".into(), "A".into())];
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(360.0, 800.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| glyphs(&mut app, ui));
        out.textures_delta.clear();
        assert!(!app.glyph_font.is_empty());
    }
}
