//! The font list: a field holding the family (type to find one) and a
//! list that drops from it with the font count, a filter, the fonts used
//! last, every family drawn in its own font with an icon of how it is
//! built, its styles one click away, the size of the samples and a
//! preview of the family under the pointer. See docs/behavior/text.md,
//! "Font list".

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::{Color32, Key, Rect, Sense, Ui, Vec2};
use std::collections::HashMap;
use tracedraw_core::document::TextSpan;
use tracedraw_text::traits::{FontFilter, FontTrait};
use tracedraw_text::{FamilyInfo, FontKind};

/// Width of the list.
const POPUP_W: f32 = 380.0;
/// Highest the list of families grows.
const LIST_H: f32 = 300.0;
/// Height of the preview under the list.
const PREVIEW_H: f32 = 104.0;
/// The preview's sample text and its size in points.
pub const SAMPLE: &str = "AaBbYyZz";
const SAMPLE_PX: f32 = 46.0;
/// The most fonts used last the list may show at its top (Options >
/// Text > Fonts picks how many, five by default).
pub const RECENT_MAX: usize = 20;
/// The sample size's range, in points.
pub const SAMPLE_SIZES: std::ops::RangeInclusive<f32> = 12.0..=40.0;
/// The size samples start at.
pub const DEFAULT_SAMPLE: f32 = 18.0;
/// How long the pointer rests on a family to download (browser) before it
/// is fetched for its picture.
const REST_SECONDS: f64 = 0.4;
/// Pictures kept before the oldest go.
const MAX_PICTURES: usize = 1200;
/// The longest a frame spends reading families for the filter (#867):
/// with many fonts the rest come in the next frames.
const FILTER_BUDGET: std::time::Duration = std::time::Duration::from_millis(8);
/// Families read together for the filter.
const FILTER_CHUNK: usize = 24;

/// A family picked in a font list, with the style when one was picked
/// (bold, italic).
#[derive(Debug, Clone, PartialEq)]
pub struct FontPick {
    pub family: String,
    pub style: Option<(bool, bool)>,
}

/// Which families the list shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Show {
    #[default]
    All,
    Installed,
    Drawing,
    Download,
}

impl Show {
    const ALL: [Show; 4] = [Show::All, Show::Installed, Show::Drawing, Show::Download];

    fn key(self) -> &'static str {
        match self {
            Show::All => "font_list.show_all",
            Show::Installed => "font_list.show_installed",
            Show::Drawing => "font_list.show_drawing",
            Show::Download => "font_list.show_download",
        }
    }
}

/// A picture of a text in a font, and where its baseline is, in pixels.
#[derive(Clone)]
pub struct Picture {
    pub texture: egui::TextureHandle,
    pub baseline_px: f32,
}

/// Pictures of family names (and samples) in their own fonts and what the
/// lists know of each family, for the fonts as they are now: both start
/// again when fonts are added.
#[derive(Default)]
pub struct FontPreviews {
    generation: u64,
    pictures: HashMap<PictureKey, Option<Picture>>,
    infos: HashMap<String, Option<FamilyInfo>>,
    /// Families to download that were asked for, each once.
    requested: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PictureKey {
    family: String,
    text: String,
    bold: bool,
    italic: bool,
    em_px: u16,
}

impl FontPreviews {
    /// Start again when fonts came in since the last frame.
    pub fn refresh(&mut self) {
        let g = tracedraw_text::fonts().generation();
        if g != self.generation || self.pictures.len() > MAX_PICTURES {
            self.generation = g;
            self.pictures.clear();
            self.infos.clear();
        }
    }

    /// What the list shows about a family (cached).
    pub fn info(&mut self, family: &str) -> Option<FamilyInfo> {
        if let Some(i) = self.infos.get(family) {
            return i.clone();
        }
        let info = tracedraw_text::fonts().family_info(family);
        self.infos.insert(family.to_string(), info.clone());
        info
    }

    /// The `families` with what `filter` asks, and whether every one of
    /// them was read: a family is read once (for the fonts as they are
    /// now), for at most [`FILTER_BUDGET`] a frame, so that the list opens
    /// as fast with many fonts and fills in over the next frames (#867).
    /// Nothing checked costs nothing.
    pub fn filtered(&mut self, families: &[String], filter: FontFilter) -> (Vec<String>, bool) {
        if filter.is_empty() {
            return (families.to_vec(), true);
        }
        let missing: Vec<String> = families
            .iter()
            .filter(|f| !self.infos.contains_key(f.as_str()))
            .cloned()
            .collect();
        let start = web_time::Instant::now();
        let mut read = 0;
        for chunk in missing.chunks(FILTER_CHUNK) {
            let infos = tracedraw_text::fonts().family_infos(chunk);
            for (f, info) in chunk.iter().zip(infos) {
                self.infos.insert(f.clone(), info);
            }
            read += chunk.len();
            if start.elapsed() >= FILTER_BUDGET {
                break;
            }
        }
        let complete = read == missing.len();
        let shown = families
            .iter()
            .filter(|f| {
                self.infos
                    .get(f.as_str())
                    .is_some_and(|i| filter.matches(i.as_ref()))
            })
            .cloned()
            .collect();
        (shown, complete)
    }

    /// `text` in `family` at `em_px` pixels to the em, as a texture (cached);
    /// `None` when the family is not installed or draws nothing.
    pub fn picture(
        &mut self,
        ctx: &egui::Context,
        family: &str,
        text: &str,
        style: (bool, bool),
        em_px: f32,
    ) -> Option<Picture> {
        let key = PictureKey {
            family: family.to_string(),
            text: text.to_string(),
            bold: style.0,
            italic: style.1,
            em_px: em_px.round().clamp(4.0, 400.0) as u16,
        };
        if let Some(p) = self.pictures.get(&key) {
            return p.clone();
        }
        let picture =
            render_text(family, text, style, f32::from(key.em_px)).map(|(image, baseline_px)| {
                let name = format!("font_list:{family}:{text}:{}", key.em_px);
                Picture {
                    texture: ctx.load_texture(name, image, egui::TextureOptions::LINEAR),
                    baseline_px,
                }
            });
        self.pictures.insert(key, picture.clone());
        picture
    }

    /// Ask for a family to download once.
    fn request(&mut self, family: &str) {
        if !self.requested.iter().any(|f| f == family) {
            self.requested.push(family.to_string());
            tracedraw_text::fonts().request_family(family);
        }
    }
}

/// The colour of text pictures shown on a white card (the preview, the
/// font sampler), as the text would print.
pub const CARD_INK: Color32 = Color32::from_gray(0x20);

/// `text` set in `family` (bold and italic as `style`) with `em_px`
/// pixels to the em, a white mask on transparent (tinted where it is
/// painted: the theme's text colour in lists, #660), and its baseline's distance
/// from the top in pixels. `None` when the family is not installed (its
/// stand-in would show instead) or the text draws nothing.
pub fn render_text(
    family: &str,
    text: &str,
    style: (bool, bool),
    em_px: f32,
) -> Option<(egui::ColorImage, f32)> {
    let fonts = tracedraw_text::fonts();
    if !fonts.has_family(family) || text.is_empty() || !em_px.is_finite() || em_px <= 0.0 {
        return None;
    }
    // At 72 pt the em is an inch: 25.4 mm.
    let mut span = TextSpan::new(text, family, 72.0);
    span.bold = style.0;
    span.italic = style.1;
    let layout = fonts.outline(std::slice::from_ref(&span));
    let line = layout.lines.first()?;
    let scale = f64::from(em_px) / 25.4;
    let ascent = line.ascent.abs().max(layout.bounds.y1.max(0.0));
    let descent = line.descent.abs().max((-layout.bounds.y0).max(0.0));
    let advance = line
        .edges
        .last()
        .copied()
        .unwrap_or(0.0)
        .max(layout.bounds.x1);
    let pad = 1.0;
    let w = ((advance.max(0.0) * scale).ceil() + 2.0 * pad).clamp(1.0, 2048.0) as u32;
    let h = (((ascent + descent) * scale).ceil() + 2.0 * pad).clamp(1.0, 1024.0) as u32;
    let path = tracedraw_render::to_sk_path(&layout.path)?;
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(0xff, 0xff, 0xff, 0xff);
    paint.anti_alias = true;
    let baseline = (ascent * scale) as f32 + pad as f32;
    let t = tiny_skia::Transform::from_row(
        scale as f32,
        0.0,
        0.0,
        -(scale as f32),
        pad as f32,
        baseline,
    );
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, t, None);
    let image = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], pixmap.data());
    Some((image, baseline))
}

/// Whether the list's families are needed this frame: while it is open,
/// its field has the focus or just lost it (Enter picks), or the wheel
/// turns over it (#665). Closed on a bar it needs none (#616).
fn list_needed(open: bool, focused: bool, lost_focus: bool, wheeling: bool) -> bool {
    open || focused || lost_focus || wheeling
}

/// The families the list holds for `show`; none when they are not
/// `needed`, so that a closed list on a bar clones no name a frame
/// (#616: 1332 families on the reporter's PC).
fn listed_families(app: &App, show: Show, needed: bool) -> Vec<String> {
    if !needed {
        return Vec::new();
    }
    match show {
        Show::All => app.font_families.clone(),
        Show::Installed => {
            let fonts = tracedraw_text::fonts();
            app.font_families
                .iter()
                .filter(|f| fonts.has_family(f))
                .cloned()
                .collect()
        }
        Show::Drawing => {
            let used = if app.has_document() {
                app.doc().font_families_used()
            } else {
                Vec::new()
            };
            app.font_families
                .iter()
                .filter(|f| used.contains(f))
                .cloned()
                .collect()
        }
        Show::Download => {
            let fonts = tracedraw_text::fonts();
            app.font_families
                .iter()
                .filter(|f| !fonts.has_family(f))
                .cloned()
                .collect()
        }
    }
}

/// The families a list shows for what was typed: all of them when
/// nothing was (or the field still holds the current family), else those
/// holding the typed letters, ignoring case: the name itself first, then
/// names starting with them, then the rest, each alphabetically.
pub fn matching(families: &[String], typed: Option<&str>, current: &str) -> Vec<String> {
    let typed = typed.map(str::trim).unwrap_or("");
    if typed.is_empty() || typed == current {
        return families.to_vec();
    }
    let t = typed.to_lowercase();
    let mut exact = Vec::new();
    let mut starts = Vec::new();
    let mut inside = Vec::new();
    for f in families {
        let l = f.to_lowercase();
        if l == t {
            exact.push(f.clone());
        } else if l.starts_with(&t) {
            starts.push(f.clone());
        } else if l.contains(&t) {
            inside.push(f.clone());
        }
    }
    exact.extend(starts);
    exact.extend(inside);
    exact
}

/// Put `family` first among the fonts used last, keeping [`RECENT_MAX`].
pub fn remember(recent: &mut Vec<String>, family: &str) {
    recent.retain(|f| f != family);
    recent.insert(0, family.to_string());
    recent.truncate(RECENT_MAX);
}

/// One row of the list: a family, or one of its styles under it.
#[derive(Debug, Clone, PartialEq)]
enum Row {
    Family(String),
    Style {
        family: String,
        name: String,
        bold: bool,
        italic: bool,
    },
}

/// A list's state between frames.
#[derive(Debug, Clone, Default)]
struct ListState {
    /// What was typed in the field since it took the focus.
    typed: Option<String>,
    /// The row the arrow keys moved to.
    highlight: Option<usize>,
    /// The family whose styles show under it.
    expanded: Option<String>,
    /// Bring the highlighted row into view.
    scroll: bool,
    show: Show,
    /// The family (and style) last under the pointer.
    hovered: Option<(String, (bool, bool))>,
    /// The keys or typing moved the highlight since the pointer last
    /// rested on a row: the preview follows the highlight.
    keyed: bool,
    rest: Option<(String, f64)>,
    /// What the families must have (#867).
    filter: FontFilter,
    /// Families are still being read for the filter.
    reading: bool,
}

/// A font list showing `current`, `width` points wide. Returns what was
/// picked; the caller sets it (and keeps it among the fonts used last
/// through [`remember`], which a pick here already did).
///
/// Also returns the family (and style) the list points at while it is
/// open: the row under the pointer, or the one the keys moved to (for a
/// live preview in the drawing, #370).
pub fn font_list_live(
    app: &mut App,
    ui: &mut Ui,
    id_salt: &str,
    current: &str,
    width: f32,
) -> (Option<FontPick>, Option<FontPick>) {
    font_list_with(app, ui, id_salt, current, width, true)
}

/// [`font_list_live`]; `wheel` lets the mouse wheel over the closed list
/// pick the family before or after the current one (#665).
fn font_list_with(
    app: &mut App,
    ui: &mut Ui,
    id_salt: &str,
    current: &str,
    width: f32,
    wheel: bool,
) -> (Option<FontPick>, Option<FontPick>) {
    let id = ui.make_persistent_id(("font_list", id_salt));
    let popup_id = id.with("popup");
    let ctx = ui.ctx().clone();
    let mut st: ListState = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    app.font_previews.refresh();
    let mut pick: Option<FontPick> = None;

    // The field and its arrow.
    let mut text = st.typed.clone().unwrap_or_else(|| current.to_string());
    let field_id = id.with("field");
    let row = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let field = ui.add(
            egui::TextEdit::singleline(&mut text)
                .id(field_id)
                .desired_width((width - 18.0).max(40.0)),
        );
        let (arrow_rect, arrow) =
            ui.allocate_exact_size(Vec2::new(18.0, field.rect.height()), Sense::click());
        let fill = if arrow.hovered() {
            Tokens::tool_hover()
        } else {
            crate::theme::Tokens::panel()
        };
        ui.painter().rect_filled(arrow_rect.shrink(1.0), 1.0, fill);
        let c = arrow_rect.center();
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + Vec2::new(-4.0, -2.0),
                c + Vec2::new(4.0, -2.0),
                c + Vec2::new(0.0, 3.0),
            ],
            Tokens::icon(),
            egui::Stroke::NONE,
        ));
        (field, arrow)
    });
    let (field, arrow) = row.inner;
    let anchor = field.rect.union(arrow.rect);
    // A family that came with the drawing (embedded in the file, not
    // installed) says so in the field while it shows that name (#518).
    if !field.has_focus() && st.typed.is_none() && is_embedded(current) {
        embedded_tag(ui.painter(), field.rect.shrink2(Vec2::new(3.0, 1.0)), true);
    }
    let was_open = egui::Popup::is_id_open(&ctx, popup_id);
    if field.gained_focus() {
        // Typing replaces the name.
        if let Some(mut state) = egui::TextEdit::load_state(&ctx, field_id) {
            let n = text.chars().count();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(n),
                )));
            state.store(&ctx, field_id);
        }
        egui::Popup::open_id(&ctx, popup_id);
        st.scroll = true;
    }
    if field.changed() {
        st.typed = Some(text.clone());
        st.highlight = Some(0);
        st.scroll = true;
        st.keyed = true;
        egui::Popup::open_id(&ctx, popup_id);
    }
    if arrow.clicked() {
        if was_open {
            egui::Popup::close_id(&ctx, popup_id);
        } else {
            egui::Popup::open_id(&ctx, popup_id);
            st.scroll = true;
        }
    }

    // What the list holds: built only while it is open, typed in or
    // turned with the wheel. Closed on a bar it costs nothing a frame,
    // where it cloned every family name (#616: 1332 of them).
    let wheeling = wheel
        && ui.rect_contains_pointer(anchor)
        && ui.input(|i| {
            i.raw
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::MouseWheel { .. }))
        });
    let needed = list_needed(
        egui::Popup::is_id_open(&ctx, popup_id),
        field.has_focus(),
        field.lost_focus(),
        wheeling,
    );
    let mut all = listed_families(app, st.show, needed);
    st.reading = false;
    if needed && !st.filter.is_empty() {
        let (shown, complete) = app.font_previews.filtered(&all, st.filter);
        all = shown;
        st.reading = !complete;
        if st.reading {
            ctx.request_repaint();
        }
    }
    let families = matching(&all, st.typed.as_deref(), current);
    let mut rows: Vec<Row> = Vec::with_capacity(families.len());
    for f in &families {
        rows.push(Row::Family(f.clone()));
        if st.expanded.as_deref() == Some(f.as_str()) {
            if let Some(info) = app.font_previews.info(f) {
                for face in info.faces {
                    // A semibold face is the family with its weight, not
                    // bold (#892).
                    let (family, bold, italic) = tracedraw_text::face_family(f, &face);
                    rows.push(Row::Style {
                        family,
                        name: face.name,
                        bold,
                        italic,
                    });
                }
            }
        }
    }
    let typing = st.typed.as_deref().is_some_and(|t| t != current);
    if st.highlight.is_none() || !typing && st.scroll {
        // A face named with its weight ("Titillium Web SemiBold") that no
        // family row lists: its family's row.
        let base = tracedraw_text::split_weight(current).map(|(b, _)| b);
        st.highlight = rows
            .iter()
            .position(|r| matches!(r, Row::Family(f) if f == current))
            .or_else(|| {
                rows.iter()
                    .position(|r| matches!(r, Row::Family(f) if Some(f.as_str()) == base))
            });
    }

    // The wheel over the closed list: the family after (down) or before
    // (up) the current one, as the list orders them (#665).
    let open = egui::Popup::is_id_open(&ctx, popup_id);
    let mut wheeled = false;
    if wheel && !open && ui.rect_contains_pointer(anchor) {
        let step = ui.input(|i| {
            i.raw.events.iter().rev().find_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } if delta.y != 0.0 => Some(delta.y),
                _ => None,
            })
        });
        if let Some(dy) = step {
            // The bar under the list does not scroll as well.
            crate::ui::field::take_wheel(ui);
            let at = families.iter().position(|f| f == current);
            let next = match (at, dy < 0.0) {
                (Some(i), true) => (i + 1).min(families.len().saturating_sub(1)),
                (Some(i), false) => i.saturating_sub(1),
                (None, _) => 0,
            };
            if let Some(f) = families.get(next).filter(|f| *f != current) {
                pick = Some(FontPick {
                    family: f.clone(),
                    style: None,
                });
                wheeled = true;
            }
        }
    }

    // Keys while the field has the focus.
    if open && field.has_focus() {
        let (down, up, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                i.key_pressed(Key::Enter),
                i.consume_key(egui::Modifiers::NONE, Key::Escape),
            )
        });
        let last = rows.len().saturating_sub(1);
        if down {
            st.highlight = Some(st.highlight.map_or(0, |h| (h + 1).min(last)));
            st.scroll = true;
            st.keyed = true;
        }
        if up {
            st.highlight = Some(st.highlight.map_or(0, |h| h.saturating_sub(1)));
            st.scroll = true;
            st.keyed = true;
        }
        if enter {
            pick = st.highlight.and_then(|h| rows.get(h)).map(row_pick);
        }
        if escape {
            egui::Popup::close_id(&ctx, popup_id);
            st.typed = None;
            field.surrender_focus();
        }
    } else if field.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter)) {
        pick = st.highlight.and_then(|h| rows.get(h)).map(row_pick);
    }

    // The list.
    let mut hovered_now: Option<(String, (bool, bool))> = None;
    let shown = egui::Popup::new(popup_id, ctx.clone(), anchor, ui.layer_id())
        .open_memory(None)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .width(POPUP_W)
        .show(|ui| {
            ui.set_width(POPUP_W);
            list_body(
                app,
                ui,
                &mut st,
                &rows,
                &families,
                current,
                typing,
                &mut pick,
                &mut hovered_now,
            );
        });
    if shown.is_none() && !field.has_focus() {
        st.typed = None;
        st.rest = None;
        st.hovered = None;
    }
    if hovered_now.is_some() {
        st.hovered = hovered_now;
        st.keyed = false;
    }
    let live = if shown.is_none() {
        None
    } else if st.keyed {
        st.highlight.and_then(|h| rows.get(h)).map(row_pick)
    } else {
        st.hovered.as_ref().map(|(f, style)| FontPick {
            family: f.clone(),
            style: (*style != (false, false)).then_some(*style),
        })
    };

    if let Some(p) = pick.as_ref().filter(|_| !wheeled) {
        crate::ui::font_list::remember(&mut app.settings.recent_fonts, &p.family);
        egui::Popup::close_id(&ctx, popup_id);
        st.typed = None;
        st.expanded = None;
        field.surrender_focus();
    }
    ctx.data_mut(|d| d.insert_temp(id, st));
    (pick, live)
}

/// A font list for the text settings (`app.text_font`, bold and
/// italic); true when they changed. While it is open, the selected text
/// shows the family under the pointer (#370).
pub fn text_font_list(app: &mut App, ui: &mut Ui, id_salt: &str, width: f32) -> bool {
    let current = app.text_settings_shown().0;
    // The wheel changes the text's font, never asks for the font of new
    // text (a dialog at every turn of the wheel).
    let wheel = app.has_text_target() || id_salt == "options_default_font";
    let (pick, live) = font_list_with(app, ui, id_salt, &current, width, wheel);
    // Only the lists that set the text's font preview it; the one that
    // owns the preview ends it when it closes.
    let owner = id_salt.to_string();
    // The default font in Options is a setting, not the selection's.
    let live = live.filter(|_| id_salt != "options_default_font");
    if pick.is_some() {
        app.end_font_preview();
    } else if live.is_some() {
        app.preview_text(&owner, live.map(LiveText::Font));
    } else if app.live_font.as_ref().is_some_and(|l| l.owner == owner) {
        app.end_font_preview();
    }
    match pick {
        Some(p) => {
            // With no text to change, the font picked would be the one of
            // new text: asked first, as the reference program does.
            if !app.has_text_target() {
                app.dialog = crate::ui::dialogs::Dialog::TextDefaults {
                    font: Some(p.family),
                    style: p.style,
                    size: None,
                };
                return false;
            }
            app.text_font = p.family;
            if let Some((bold, italic)) = p.style {
                app.text_bold = bold;
                app.text_italic = italic;
            }
            true
        }
        None => false,
    }
}

/// The font of new text (Options > Text > Display): it changes the
/// default only, never the text selected (#449).
pub fn default_font_list(app: &mut App, ui: &mut Ui, id_salt: &str, width: f32) {
    let current = app.text_default_font.clone();
    let (pick, _) = font_list_live(app, ui, id_salt, &current, width);
    if let Some(p) = pick {
        if !app.has_text_target() {
            app.text_font = p.family.clone();
        }
        app.text_default_font = p.family;
    }
}

fn row_pick(row: &Row) -> FontPick {
    match row {
        Row::Family(f) => FontPick {
            family: f.clone(),
            style: None,
        },
        Row::Style {
            family,
            bold,
            italic,
            ..
        } => FontPick {
            family: family.clone(),
            style: Some((*bold, *italic)),
        },
    }
}

/// The inside of the list: count and filter, fonts used last, the
/// families, the sample size and the preview.
#[allow(clippy::too_many_arguments)]
fn list_body(
    app: &mut App,
    ui: &mut Ui,
    st: &mut ListState,
    rows: &[Row],
    families: &[String],
    current: &str,
    typing: bool,
    pick: &mut Option<FontPick>,
    hovered_now: &mut Option<(String, (bool, bool))>,
) {
    let ctx = ui.ctx().clone();
    ui.spacing_mut().item_spacing.y = 2.0;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(trf(
                "font_list.count",
                &[("n", &families.len().to_string())],
            ))
            .color(Tokens::text_dim())
            .size(11.5),
        );
        if st.reading {
            ui.label(
                egui::RichText::new(tr("font_list.reading"))
                    .color(Tokens::text_dim())
                    .size(11.5),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            filter_menu(ui, st);
            egui::ComboBox::from_id_salt(ui.id().with("show"))
                .selected_text(egui::RichText::new(tr(st.show.key())).size(11.5))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for s in Show::ALL {
                        if ui.selectable_label(st.show == s, tr(s.key())).clicked() {
                            st.show = s;
                            st.highlight = None;
                            st.scroll = true;
                        }
                    }
                });
        });
    });
    let sample = app
        .settings
        .font_sample_px
        .clamp(*SAMPLE_SIZES.start(), *SAMPLE_SIZES.end());
    let row_h = (sample * 1.35 + 6.0).max(22.0);
    let ppp = ctx.pixels_per_point();

    // The fonts used last, while nothing is typed.
    let recent: Vec<String> = app
        .settings
        .recent_fonts
        .iter()
        .filter(|f| app.font_families.contains(f))
        .take(if app.settings.show_recent_fonts {
            app.settings.recent_font_count.clamp(1, RECENT_MAX)
        } else {
            0
        })
        .cloned()
        .collect();
    if !typing && st.show == Show::All && st.filter.is_empty() && !recent.is_empty() {
        ui.label(
            egui::RichText::new(tr("font_list.recent"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        for f in &recent {
            let row = Row::Family(f.clone());
            let r = draw_row(app, ui, st, &row, row_h, sample, ppp, f == current, false);
            if r.clicked {
                *pick = Some(row_pick(&row));
            }
            if r.hovered {
                *hovered_now = Some((f.clone(), (false, false)));
            }
        }
        ui.separator();
    }

    // Every family.
    if rows.is_empty() && !st.reading {
        let why = if st.filter.is_empty() || typing {
            "font_list.no_match"
        } else {
            "font_list.no_filter_match"
        };
        ui.label(egui::RichText::new(tr(why)).color(Tokens::text_dim()));
    }
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt(ui.id().with("families"))
        .max_height(LIST_H)
        .auto_shrink([false, true]);
    if st.scroll {
        if let Some(h) = st.highlight {
            let y =
                (h as f32 * (row_h + ui.spacing().item_spacing.y) - LIST_H / 2.0 + row_h).max(0.0);
            scroll = scroll.vertical_scroll_offset(y);
        }
        st.scroll = false;
    }
    let spacing = ui.spacing().item_spacing.y;
    scroll.show_rows(ui, row_h, rows.len(), |ui, range| {
        ui.spacing_mut().item_spacing.y = spacing;
        for i in range {
            let Some(row) = rows.get(i) else {
                continue;
            };
            let selected = matches!(row, Row::Family(f) if f == current);
            let highlighted = st.highlight == Some(i);
            let r = draw_row(app, ui, st, row, row_h, sample, ppp, selected, highlighted);
            if r.toggled {
                if let Row::Family(f) = row {
                    st.expanded = if st.expanded.as_deref() == Some(f.as_str()) {
                        None
                    } else {
                        Some(f.clone())
                    };
                }
            } else if r.clicked {
                *pick = Some(row_pick(row));
            }
            if r.hovered {
                *hovered_now = Some(match row {
                    Row::Family(f) => (f.clone(), (false, false)),
                    Row::Style {
                        family,
                        bold,
                        italic,
                        ..
                    } => (family.clone(), (*bold, *italic)),
                });
            }
        }
    });

    // The size of the samples.
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("A")
                .size(10.0)
                .color(Tokens::text_dim()),
        );
        let mut s = app.settings.font_sample_px;
        // The slider fills the row between the small and the large A.
        ui.spacing_mut().slider_width = (ui.available_width() - 24.0).max(60.0);
        ui.add(
            egui::Slider::new(&mut s, SAMPLE_SIZES)
                .show_value(false)
                .clamping(egui::SliderClamping::Always),
        )
        .on_hover_text(tr("font_list.sample_size"));
        app.settings.font_sample_px = s;
        ui.label(
            egui::RichText::new("A")
                .size(17.0)
                .color(Tokens::text_dim()),
        );
    });

    // The preview of the family under the pointer (else the one in use).
    let keyed = st.highlight.and_then(|h| rows.get(h)).map(|r| match r {
        Row::Family(f) => (f.clone(), (false, false)),
        Row::Style {
            family,
            bold,
            italic,
            ..
        } => (family.clone(), (*bold, *italic)),
    });
    let (family, style) = hovered_now
        .clone()
        .or_else(|| if st.keyed { keyed.clone() } else { None })
        .or_else(|| st.hovered.clone())
        .or(keyed)
        .unwrap_or_else(|| (current.to_string(), (false, false)));
    preview(app, ui, &family, style, ppp);

    // A family to download under the pointer for a moment is fetched.
    let now = ctx.input(|i| i.time);
    match hovered_now.as_ref().map(|(f, _)| f.clone()) {
        Some(f) if !tracedraw_text::fonts().has_family(&f) => {
            let since = match &st.rest {
                Some((g, t)) if *g == f => *t,
                _ => {
                    st.rest = Some((f.clone(), now));
                    now
                }
            };
            if now - since >= REST_SECONDS {
                app.font_previews.request(&f);
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(120));
            }
        }
        _ => st.rest = None,
    }
}

/// The i18n key of a trait the filter offers.
fn trait_key(t: FontTrait) -> &'static str {
    match t {
        FontTrait::Serif => "font_list.trait_serif",
        FontTrait::SansSerif => "font_list.trait_sans",
        FontTrait::Script => "font_list.trait_script",
        FontTrait::Decorative => "font_list.trait_decorative",
        FontTrait::Monospace => "font_list.trait_monospace",
        FontTrait::Symbol => "font_list.trait_symbol",
        FontTrait::Variable => "font_list.trait_variable",
        FontTrait::Features => "font_list.trait_features",
    }
}

/// The filter: what the listed families must have (#867), checked in a
/// menu that stays open while boxes are ticked. Its button says how many
/// are checked.
fn filter_menu(ui: &mut Ui, st: &mut ListState) {
    let n = st.filter.count();
    let label = if n == 0 {
        tr("font_list.filter")
    } else {
        trf("font_list.filter_n", &[("n", &n.to_string())])
    };
    let mut text = egui::RichText::new(label).size(11.5);
    if n > 0 {
        text = text.strong();
    }
    let before = st.filter;
    egui::ComboBox::from_id_salt(ui.id().with("filter"))
        .selected_text(text)
        .width(96.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            for (i, t) in FontTrait::ALL.into_iter().enumerate() {
                // The kinds of letters, then what a family has.
                if i > 0 && t.is_class() != FontTrait::ALL[i - 1].is_class() {
                    ui.separator();
                }
                let mut on = st.filter.has(t);
                if ui.checkbox(&mut on, tr(trait_key(t))).changed() {
                    st.filter.set(t, on);
                }
            }
            ui.separator();
            if ui
                .add_enabled(n > 0, egui::Button::new(tr("font_list.filter_clear")))
                .clicked()
            {
                st.filter = FontFilter::default();
            }
        })
        .response
        .on_hover_text(tr("font_list.filter_tip"));
    if st.filter != before {
        st.highlight = None;
        st.scroll = true;
    }
}

/// What happened to a row this frame.
#[derive(Default)]
struct RowResponse {
    clicked: bool,
    hovered: bool,
    /// Its styles were shown or hidden.
    toggled: bool,
}

/// One row: the icon of how the family is built, the name in its own font
/// (else in the interface font), and the arrow to its styles.
#[allow(clippy::too_many_arguments)]
fn draw_row(
    app: &mut App,
    ui: &mut Ui,
    st: &ListState,
    row: &Row,
    row_h: f32,
    sample: f32,
    ppp: f32,
    selected: bool,
    highlighted: bool,
) -> RowResponse {
    let ctx = ui.ctx().clone();
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::click());
    let mut out = RowResponse {
        hovered: resp.hovered(),
        ..Default::default()
    };
    if !ui.is_rect_visible(rect) {
        return out;
    }
    let painter = ui.painter_at(rect);
    let fill = if selected {
        Some(Tokens::tool_active())
    } else if highlighted || resp.hovered() {
        Some(Tokens::tool_hover())
    } else {
        None
    };
    if let Some(c) = fill {
        painter.rect_filled(rect, 2.0, c);
    }
    let (family, label, style, indent) = match row {
        Row::Family(f) => (f.as_str(), f.as_str(), (false, false), 0.0),
        Row::Style {
            family,
            name,
            bold,
            italic,
        } => (family.as_str(), name.as_str(), (*bold, *italic), 22.0),
    };
    let info = app.font_previews.info(family);
    let mut left = rect.left() + 4.0 + indent;
    if matches!(row, Row::Family(_)) {
        let icon = Rect::from_min_size(egui::pos2(left, rect.center().y - 8.0), Vec2::splat(16.0));
        let tip = kind_icon(&painter, icon, info.as_ref());
        if ui.rect_contains_pointer(icon) {
            resp.clone().on_hover_text(tip);
        }
        left = icon.right() + 8.0;
    }
    // The styles, for a family with more than one.
    let mut right = rect.right() - 4.0;
    let faces = info.as_ref().map_or(0, |i| i.faces.len());
    if matches!(row, Row::Family(_)) && faces > 1 {
        let arrow = Rect::from_min_max(
            egui::pos2(rect.right() - 20.0, rect.top()),
            rect.right_bottom(),
        );
        let open = st.expanded.as_deref() == Some(family);
        let c = arrow.center();
        let points = if open {
            vec![
                c + Vec2::new(-4.0, -2.0),
                c + Vec2::new(4.0, -2.0),
                c + Vec2::new(0.0, 3.0),
            ]
        } else {
            vec![
                c + Vec2::new(-2.0, -4.0),
                c + Vec2::new(3.0, 0.0),
                c + Vec2::new(-2.0, 4.0),
            ]
        };
        let hot = ui.rect_contains_pointer(arrow);
        painter.add(egui::epaint::PathShape::convex_polygon(
            points,
            if hot { Tokens::text() } else { Tokens::icon() },
            egui::Stroke::NONE,
        ));
        if hot {
            resp.clone()
                .on_hover_text(trf("font_list.styles", &[("n", &faces.to_string())]));
        }
        if resp.clicked() && hot {
            out.toggled = true;
        }
        right = arrow.left() - 2.0;
    }
    if resp.clicked() && !out.toggled {
        out.clicked = true;
    }
    // "Embedded" at the end of a family that came with the drawing.
    if matches!(row, Row::Family(_)) && info.as_ref().is_some_and(|i| i.embedded) {
        let tag = embedded_tag(
            &painter,
            Rect::from_min_max(
                egui::pos2(left, rect.top()),
                egui::pos2(right, rect.bottom()),
            ),
            false,
        );
        right = tag.left() - 6.0;
    }
    // The name in its own font; symbol fonts, families to download and
    // names it cannot draw in the interface font.
    let own = !info.as_ref().is_some_and(|i| i.symbol);
    let picture = if own {
        app.font_previews
            .picture(&ctx, family, label, style, sample * ppp)
    } else {
        None
    };
    let room = Rect::from_min_max(
        egui::pos2(left, rect.top()),
        egui::pos2(right, rect.bottom()),
    );
    match picture {
        Some(p) => {
            let size = p.texture.size_vec2() / ppp;
            let baseline = rect.center().y + sample * 0.32;
            let top = baseline - p.baseline_px / ppp;
            let shown = Rect::from_min_size(egui::pos2(left, top), size);
            let clip = shown.intersect(room);
            let uv = Rect::from_min_max(
                egui::pos2(
                    (clip.left() - shown.left()) / size.x,
                    (clip.top() - shown.top()) / size.y,
                ),
                egui::pos2(
                    (clip.right() - shown.left()) / size.x.max(1.0),
                    (clip.bottom() - shown.top()) / size.y.max(1.0),
                ),
            );
            painter.image(p.texture.id(), clip, uv, Tokens::text());
        }
        None => {
            let font = egui::FontId::proportional((sample * 0.8).clamp(12.0, 22.0));
            painter.with_clip_rect(room).text(
                egui::pos2(left, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                font,
                Tokens::text(),
            );
        }
    }
    out
}

/// Whether `family` came with a drawing and is not installed.
fn is_embedded(family: &str) -> bool {
    !family.is_empty() && tracedraw_text::fonts().is_document_family(family)
}

/// The word "Embedded" at the right end of `r`, on the field's colour
/// when `field` (over the end of a long name); the room it took.
fn embedded_tag(painter: &egui::Painter, r: Rect, field: bool) -> Rect {
    let text = tr("font_list.embedded_tag");
    let font = egui::FontId::proportional(11.0);
    let galley = painter.layout_no_wrap(text, font, Color32::from_rgb(0xC0, 0x6A, 0x12));
    let size = galley.size();
    let at = egui::pos2(r.right() - size.x, r.center().y - size.y / 2.0);
    let room = Rect::from_min_size(at, size).expand2(Vec2::new(3.0, 0.0));
    if field {
        painter.rect_filled(room, 0.0, Tokens::field());
    }
    painter.galley(at, galley, Color32::from_rgb(0xC0, 0x6A, 0x12));
    room
}

/// The small picture of how a family is built; returns its description.
fn kind_icon(painter: &egui::Painter, r: Rect, info: Option<&FamilyInfo>) -> String {
    let (letter, color, key) = match info {
        Some(i) if i.embedded => (
            "D",
            Color32::from_rgb(0xC0, 0x6A, 0x12),
            "font_list.embedded",
        ),
        Some(i) => match i.kind {
            FontKind::TrueType => (
                "T",
                Color32::from_rgb(0x4A, 0x6F, 0x9C),
                "font_list.truetype",
            ),
            FontKind::OpenType => (
                "O",
                Color32::from_rgb(0x2E, 0x8B, 0x57),
                "font_list.opentype",
            ),
            FontKind::Variable => (
                "V",
                Color32::from_rgb(0x7B, 0x4E, 0xA3),
                "font_list.variable",
            ),
            FontKind::Offered => ("", Color32::from_gray(0x8C), "font_list.download"),
        },
        None => ("", Color32::from_gray(0xB0), "font_list.download"),
    };
    if letter.is_empty() {
        // A downward arrow into a tray: fetched when first used.
        let s = egui::Stroke::new(1.4, color);
        let c = r.center();
        painter.line_segment([c + Vec2::new(0.0, -6.0), c + Vec2::new(0.0, 2.0)], s);
        painter.line_segment([c + Vec2::new(-3.5, -1.5), c + Vec2::new(0.0, 2.0)], s);
        painter.line_segment([c + Vec2::new(3.5, -1.5), c + Vec2::new(0.0, 2.0)], s);
        painter.line_segment([c + Vec2::new(-6.0, 3.0), c + Vec2::new(-6.0, 6.0)], s);
        painter.line_segment([c + Vec2::new(-6.0, 6.0), c + Vec2::new(6.0, 6.0)], s);
        painter.line_segment([c + Vec2::new(6.0, 6.0), c + Vec2::new(6.0, 3.0)], s);
    } else {
        painter.rect_stroke(
            r.shrink(1.0),
            3.0,
            egui::Stroke::new(1.2, color),
            egui::StrokeKind::Inside,
        );
        painter.text(
            r.center() + Vec2::new(0.0, 0.5),
            egui::Align2::CENTER_CENTER,
            letter,
            crate::theme::bold(10.5),
            color,
        );
    }
    let mut tip = tr(key);
    if info.is_some_and(|i| i.symbol) {
        tip = format!("{tip}, {}", tr("font_list.symbol"));
    }
    tip
}

/// The preview: the sample in the family (and style) between its top,
/// baseline and bottom lines, and its name under it.
fn preview(app: &mut App, ui: &mut Ui, family: &str, style: (bool, bool), ppp: f32) {
    let ctx = ui.ctx().clone();
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), PREVIEW_H), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, Color32::WHITE);
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let area = Rect::from_min_max(
        rect.min + Vec2::new(10.0, 8.0),
        egui::pos2(rect.right() - 10.0, rect.bottom() - 24.0),
    );
    let guide = egui::Stroke::new(1.0, Color32::from_rgb(0x9C, 0xC8, 0xF0));
    match app
        .font_previews
        .picture(&ctx, family, SAMPLE, style, SAMPLE_PX * ppp)
    {
        Some(p) => {
            let size = p.texture.size_vec2() / ppp;
            let scale = (area.width() / size.x).min(area.height() / size.y).min(1.0);
            let shown = Rect::from_min_size(
                egui::pos2(area.left(), area.center().y - size.y * scale / 2.0),
                size * scale,
            );
            let baseline = shown.top() + p.baseline_px / ppp * scale;
            for y in [shown.top() + 1.0, baseline, shown.bottom() - 1.0] {
                painter.hline(rect.x_range().shrink(4.0), y, guide);
            }
            painter.image(
                p.texture.id(),
                shown,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                CARD_INK,
            );
        }
        None => {
            painter.text(
                area.center(),
                egui::Align2::CENTER_CENTER,
                tr("font_list.no_preview"),
                egui::FontId::proportional(12.0),
                Color32::from_gray(0x80),
            );
        }
    }
    let style_name = match style {
        (false, false) => tr("font_list.regular"),
        (true, false) => tr("font_list.bold"),
        (false, true) => tr("font_list.italic"),
        (true, true) => tr("font_list.bold_italic"),
    };
    painter.text(
        egui::pos2(rect.left() + 10.0, rect.bottom() - 7.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{family} ({style_name})"),
        egui::FontId::proportional(11.5),
        CARD_INK,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn typing_finds_the_name_then_names_starting_with_it_then_the_rest() {
        let all = names(&[
            "Arial",
            "Arial Black",
            "Bahnschrift",
            "Liberation Sans",
            "Open Sans",
            "Sans",
        ]);
        // Nothing typed, or the field still holding the family: all.
        assert_eq!(matching(&all, None, "Arial"), all);
        assert_eq!(matching(&all, Some("Arial"), "Arial"), all);
        assert_eq!(
            matching(&all, Some("sans"), "Arial"),
            names(&["Sans", "Liberation Sans", "Open Sans"])
        );
        assert_eq!(
            matching(&all, Some("ari"), "Bahnschrift"),
            names(&["Arial", "Arial Black"])
        );
        assert!(matching(&all, Some("zzz"), "Arial").is_empty());
        assert_eq!(matching(&all, Some("  "), "Arial"), all);
    }

    /// A closed list on a bar clones no family name a frame; open, typed
    /// in, just left with Enter or turned with the wheel it holds them
    /// all (#616, #665).
    #[test]
    fn a_closed_list_builds_no_family_list() {
        let mut app = App::headless();
        app.font_families = (0..1332).map(|i| format!("Family {i}")).collect();
        let closed = list_needed(false, false, false, false);
        assert!(listed_families(&app, Show::All, closed).is_empty());
        for (open, focused, lost, wheel) in [
            (true, false, false, false),
            (false, true, false, false),
            (false, false, true, false),
            (false, false, false, true),
        ] {
            let needed = list_needed(open, focused, lost, wheel);
            assert_eq!(listed_families(&app, Show::All, needed).len(), 1332);
        }
    }

    #[test]
    fn the_fonts_used_last_keep_the_newest_first() {
        let mut recent = Vec::new();
        for f in ["A", "B", "C", "A", "D", "E", "F"] {
            remember(&mut recent, f);
        }
        assert_eq!(recent, names(&["F", "E", "D", "A", "C", "B"]));
        // The list keeps up to the most Options > Text > Fonts can show.
        for i in 0..30 {
            remember(&mut recent, &format!("Font {i}"));
        }
        assert_eq!(recent.len(), RECENT_MAX);
        assert_eq!(recent[0], "Font 29");
    }

    #[test]
    fn names_are_drawn_in_their_own_font() {
        let fonts = tracedraw_text::fonts();
        let Some(family) = fonts
            .families()
            .into_iter()
            .find(|f| fonts.family_info(f).is_some_and(|i| !i.symbol))
        else {
            return;
        };
        let (image, baseline) =
            render_text(&family, "Hamburgefonts", (false, false), 24.0).expect("drawn");
        assert!(
            image.size[0] > 40 && image.size[1] >= 20,
            "{:?}",
            image.size
        );
        assert!(baseline > 10.0 && baseline < image.size[1] as f32);
        // Ink, and transparent paper. The ink is a white mask, tinted
        // where it is painted (#660).
        assert!(image.pixels.iter().any(|p| p.a() > 200));
        assert!(image.pixels.iter().any(|p| p.r() == 255 && p.a() == 255));
        assert!(image.pixels.iter().any(|p| p.a() == 0));
        // A family that is not installed draws nothing (its stand-in would
        // be a lie), and so does no text.
        assert!(render_text("No Such Family 12345", "Abc", (false, false), 24.0).is_none());
        assert!(render_text(&family, "", (false, false), 24.0).is_none());
    }

    /// The names in the list are drawn in the theme's text colour, light
    /// on the dark themes, not dark on the dark popup (#660).
    #[test]
    fn names_take_the_theme_text_colour() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let fonts = tracedraw_text::fonts();
        let Some(family) = app
            .font_families
            .iter()
            .find(|f| fonts.family_info(f).is_some_and(|i| !i.symbol))
            .cloned()
        else {
            return;
        };
        let mut st = ListState::default();
        let rows = vec![Row::Family(family.clone())];
        let families = vec![family.clone()];
        let mut tints = Vec::new();
        // Twice: the picture is made on the first frame.
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1200.0, 900.0),
                )),
                ..Default::default()
            };
            let (mut pick, mut hovered) = (None, None);
            let mut out = ctx.run_ui(input, |ui| {
                list_body(
                    &mut app,
                    ui,
                    &mut st,
                    &rows,
                    &families,
                    &family,
                    false,
                    &mut pick,
                    &mut hovered,
                );
            });
            out.textures_delta.clear();
            tints.clear();
            for s in &out.shapes {
                if let egui::Shape::Mesh(m) = &s.shape {
                    if m.texture_id != egui::TextureId::default() {
                        tints.extend(m.vertices.iter().map(|v| v.color));
                    }
                }
            }
        }
        assert!(!tints.is_empty());
        assert!(tints.iter().all(|c| *c != Color32::WHITE), "{tints:?}");
        assert!(tints.iter().all(|c| c.r() == c.g() && c.g() == c.b()));
    }

    /// The list draws open, with every filter, styles shown, a long list
    /// and typed text; picking returns the family and keeps it among the
    /// fonts used last.
    /// A family that came with a drawing and is not installed says
    /// "Embedded" in the font box and at the end of its row in the list;
    /// installed families do not (#518).
    #[test]
    fn fonts_embedded_in_the_drawing_say_so() {
        let mut app = App::headless();
        app.add_carried_fonts(&[tracedraw_core::DocumentFont::new(
            "Selawik",
            include_bytes!("../../assets/fonts/Selawik-Regular.ttf").to_vec(),
        )]);
        if !tracedraw_text::fonts().is_document_family("Selawik") {
            return; // installed on this machine
        }
        let installed = app
            .font_families
            .iter()
            .find(|f| !is_embedded(f))
            .cloned()
            .expect("an installed family");
        let tag = tr("font_list.embedded_tag");
        let ctx = crate::theme::ui_context();
        let texts = |out: &egui::FullOutput| -> Vec<String> {
            out.shapes
                .iter()
                .filter_map(|c| match &c.shape {
                    egui::Shape::Text(t) => Some(t.galley.text().to_string()),
                    _ => None,
                })
                .collect()
        };
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1200.0, 900.0),
            )),
            ..Default::default()
        };
        // The closed font box.
        for (family, tagged) in [("Selawik", true), (installed.as_str(), false)] {
            let mut out = ctx.run_ui(input(), |ui| {
                font_list_live(&mut app, ui, "embedded_test", family, 220.0);
            });
            out.textures_delta.clear();
            assert_eq!(texts(&out).contains(&tag), tagged, "{family}");
        }
        // The rows of the list.
        for (family, tagged) in [("Selawik", true), (installed.as_str(), false)] {
            let mut st = ListState::default();
            let rows = vec![Row::Family(family.to_string())];
            let families = vec![family.to_string()];
            let (mut pick, mut hovered) = (None, None);
            let mut out = ctx.run_ui(input(), |ui| {
                list_body(
                    &mut app,
                    ui,
                    &mut st,
                    &rows,
                    &families,
                    "",
                    false,
                    &mut pick,
                    &mut hovered,
                );
            });
            out.textures_delta.clear();
            assert_eq!(texts(&out).contains(&tag), tagged, "row {family}");
        }
    }

    /// The filter (#867) reads each family once, a little a frame, and
    /// keeps the families with what it asks; nothing checked keeps all.
    #[test]
    fn the_filter_keeps_the_families_with_what_it_asks() {
        let fonts = tracedraw_text::fonts();
        let mut families = fonts.families();
        families.push("Not Installed Anywhere".to_string());
        let mut previews = FontPreviews::default();
        let (all, done) = previews.filtered(&families, FontFilter::default());
        assert!(done);
        assert_eq!(all, families);
        for t in FontTrait::ALL {
            let mut filter = FontFilter::default();
            filter.set(t, true);
            let shown = loop {
                let (shown, done) = previews.filtered(&families, filter);
                if done {
                    break shown;
                }
            };
            assert!(!shown.iter().any(|f| f == "Not Installed Anywhere"));
            for f in &families {
                let info = fonts.family_info(f);
                assert_eq!(
                    shown.contains(f),
                    filter.matches(info.as_ref()),
                    "{f} {t:?}"
                );
            }
        }
    }

    #[test]
    fn the_list_draws_in_every_state() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.text_font = app.font_families.first().cloned().unwrap_or_default();
        let first = app.text_font.clone();
        app.settings.recent_fonts = vec![first.clone(), "Not Installed Anywhere".into()];
        let id = egui::Id::new("test_list");
        // Closed, as bars show it.
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1200.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            let current = app.text_font.clone();
            ui.push_id(id, |ui| font_list_live(&mut app, ui, "t", &current, 180.0));
        });
        out.textures_delta.clear();
        // Every filter and an expanded family draw.
        for (i, show) in Show::ALL.into_iter().enumerate() {
            // With and without a filter, still reading or read (#867).
            let mut filter = FontFilter::default();
            if i % 2 == 1 {
                filter.set(FontTrait::Serif, true);
                filter.set(FontTrait::Variable, true);
            }
            let mut st = ListState {
                show,
                expanded: Some(first.clone()),
                typed: Some(first.chars().take(2).collect()),
                scroll: true,
                filter,
                reading: i == 1,
                ..Default::default()
            };
            st.hovered = Some((first.clone(), (true, false)));
            let mut pick = None;
            let mut hovered = None;
            let rows = vec![
                Row::Family(first.clone()),
                Row::Style {
                    family: first.clone(),
                    name: "Bold".into(),
                    bold: true,
                    italic: false,
                },
            ];
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1200.0, 900.0),
                )),
                ..Default::default()
            };
            let families = vec![first.clone()];
            let mut out = ctx.run_ui(input, |ui| {
                list_body(
                    &mut app,
                    ui,
                    &mut st,
                    &rows,
                    &families,
                    &first,
                    true,
                    &mut pick,
                    &mut hovered,
                );
            });
            out.textures_delta.clear();
            assert!(pick.is_none());
        }
        // A pick keeps the family first among the fonts used last.
        remember(&mut app.settings.recent_fonts, "Picked");
        assert_eq!(
            app.settings.recent_fonts.first().map(String::as_str),
            Some("Picked")
        );
    }
}

/// What the drawing shows while a list points at it (#370).
#[derive(Debug, Clone, PartialEq)]
pub enum LiveText {
    /// A family (and style) from a font list.
    Font(FontPick),
    /// A size, in points, from the size list.
    Size(f64),
}

/// A font or size shown in the drawing while a list points at it.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveFont {
    /// The list showing it.
    pub owner: String,
    pub shown: LiveText,
    /// The text settings before (family, bold, italic, size).
    pub saved: (String, bool, bool, f64),
}

impl App {
    /// What a font change would act on: text being edited, table cells or
    /// a selected text object.
    pub fn has_text_target(&self) -> bool {
        self.text_edit.is_some()
            || self.table_edit.is_some()
            || self
                .selected_shapes()
                .iter()
                .any(|s| matches!(s.kind, tracedraw_core::ShapeKind::Text { .. }))
    }

    /// The unit font sizes are shown and typed in (#529).
    pub fn text_units(&self) -> crate::app::Units {
        crate::app::Units::from_id(&self.settings.text_units)
    }

    /// The answer OK to "make it the default of new text?": the font
    /// (with its style) and the size new text gets.
    pub fn set_text_defaults(
        &mut self,
        font: Option<String>,
        style: Option<(bool, bool)>,
        size: Option<f64>,
    ) {
        if let Some(f) = font {
            self.text_default_font = f.clone();
            self.text_font = f;
        }
        if let Some((bold, italic)) = style {
            self.text_bold = bold;
            self.text_italic = italic;
        }
        if let Some(s) = size.filter(|s| s.is_finite() && *s > 0.0) {
            self.text_default_size = s;
            self.text_size_pt = s;
        }
    }

    /// The text settings as they are, not as a preview shows them.
    pub fn text_settings_shown(&self) -> (String, bool, bool, f64) {
        self.live_font.as_ref().map_or_else(
            || {
                (
                    self.text_font.clone(),
                    self.text_bold,
                    self.text_italic,
                    self.text_size_pt,
                )
            },
            |l| l.saved.clone(),
        )
    }

    /// Show the selected text in `want` while the list `owner` points at
    /// it, as a preview the history does not keep; `None` (or what is in
    /// use already) shows the text as it is.
    pub fn preview_text(&mut self, owner: &str, want: Option<LiveText>) {
        if self.live_font.as_ref().map(|l| &l.shown) == want.as_ref() {
            return;
        }
        self.end_font_preview();
        let Some(w) = want else {
            return;
        };
        let same = match &w {
            LiveText::Font(p) => p.family == self.text_font && p.style.is_none(),
            LiveText::Size(s) => (s - self.text_size_pt).abs() < 1e-6,
        };
        if same || !self.has_text_target() {
            return;
        }
        let saved = self.text_settings_shown();
        self.engine.begin_preview();
        match &w {
            LiveText::Font(p) => {
                self.text_font = p.family.clone();
                if let Some((bold, italic)) = p.style {
                    self.text_bold = bold;
                    self.text_italic = italic;
                }
            }
            LiveText::Size(s) => self.text_size_pt = *s,
        }
        self.apply_text_style();
        (
            self.text_font,
            self.text_bold,
            self.text_italic,
            self.text_size_pt,
        ) = saved.clone();
        self.live_font = Some(LiveFont {
            owner: owner.to_string(),
            shown: w,
            saved,
        });
    }

    /// End a preview: the text as it was, the history untouched.
    pub fn end_font_preview(&mut self) {
        if let Some(l) = self.live_font.take() {
            self.engine.withdraw_preview();
            (
                self.text_font,
                self.text_bold,
                self.text_italic,
                self.text_size_pt,
            ) = l.saved;
        }
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use tracedraw_core::geometry::Point;

    /// Selecting a text shows its font and size, but the font and size of
    /// new text (Options > Text > Display) stay as set, can be changed
    /// with the text selected, and are what is kept for the next session
    /// (#449).
    #[test]
    fn the_default_text_font_does_not_follow_the_selection() {
        let mut app = App::headless();
        app.text_default_font = "Arial".into();
        app.text_default_size = 24.0;
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("Text", "Impact", 130.0)],
                origin: Point::new(10.0, 10.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        app.sync_text_defaults_from(id);
        assert_eq!(app.text_font, "Impact");
        assert_eq!(app.text_default_font, "Arial");
        // Options sets a default while the text is selected: the text is
        // left alone.
        app.text_default_size = 48.0;
        assert_eq!(text_of(&app, id), ("Impact".to_string(), 130.0));
        let prefs = app.tool_prefs();
        assert_eq!(
            (prefs.text_font.as_str(), prefs.text_size_pt),
            ("Arial", 48.0)
        );
        // Nothing selected: the Text tool's bar shows the defaults again.
        app.select(Vec::new());
        app.tool = crate::tools::Tool::Text;
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(&mut app, ui));
        out.textures_delta.clear();
        assert_eq!((app.text_font.as_str(), app.text_size_pt), ("Arial", 48.0));
    }

    /// The wheel over the closed font list of the bar picks the family
    /// after (down) or before (up) the current one, for the selected text;
    /// it does not open the list, keep the families as used last, or ask
    /// for the font of new text when nothing is selected (#665).
    #[test]
    fn the_wheel_over_the_font_list_steps_the_font() {
        let mut app = App::headless();
        let families = app.font_families.clone();
        if families.len() < 3 {
            return;
        }
        let id = app
            .new_shape(tracedraw_core::ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("Text", &families[1], 24.0)],
                origin: Point::new(10.0, 10.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        app.sync_text_defaults_from(id);
        let recent = app.settings.recent_fonts.clone();
        let ctx = crate::theme::ui_context();
        let origin = egui::pos2(100.0, 100.0);
        let run = |app: &mut App, events: Vec<egui::Event>| -> bool {
            let mut changed = false;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::Area::new(egui::Id::new("font_bar"))
                        .fixed_pos(origin)
                        .show(ui.ctx(), |ui| {
                            changed = text_font_list(app, ui, "propbar_font", 180.0);
                        });
                },
            );
            out.textures_delta.clear();
            changed
        };
        let wheel = |app: &mut App, dy: f32| {
            run(
                app,
                vec![egui::Event::PointerMoved(origin + egui::vec2(40.0, 8.0))],
            );
            run(
                app,
                vec![egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, dy),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                }],
            )
        };
        run(&mut app, Vec::new());
        // Down: the next family; up twice: back past it to the first.
        assert!(wheel(&mut app, -1.0));
        assert_eq!(app.text_font, families[2]);
        assert!(wheel(&mut app, 1.0));
        assert_eq!(app.text_font, families[1]);
        assert!(wheel(&mut app, 1.0));
        assert_eq!(app.text_font, families[0]);
        // At the top it stays.
        assert!(!wheel(&mut app, 1.0));
        assert_eq!(app.text_font, families[0]);
        assert_eq!(app.settings.recent_fonts, recent);
        let popup = egui::Id::new("font_bar")
            .with(("font_list", "propbar_font"))
            .with("popup");
        assert!(!egui::Popup::is_id_open(&ctx, popup));
        // Nothing selected: the wheel leaves the font of new text alone
        // and asks nothing.
        app.select(Vec::new());
        app.dialog = crate::ui::dialogs::Dialog::None;
        assert!(!wheel(&mut app, -1.0));
        assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::None));
    }

    fn text_of(app: &App, id: tracedraw_core::ShapeId) -> (String, f64) {
        match app.doc().find_shape(id).map(|s| s.kind.clone()) {
            Some(tracedraw_core::ShapeKind::Text { spans, .. }) => {
                (spans[0].font_family.clone(), spans[0].size_pt)
            }
            _ => panic!("text"),
        }
    }

    /// While a list points at a family or size, the selected text shows
    /// it; when it stops, the text, the settings and the history are as
    /// they were (#370).
    #[test]
    fn the_text_shows_the_font_a_list_points_at() {
        let mut app = App::headless();
        app.start_text(Point::new(20.0, 50.0), None);
        app.text_insert("Preview");
        app.finish_text();
        let id = *app.selection.first().expect("text");
        let (font, size) = text_of(&app, id);
        app.undo();
        app.redo();
        let history = app.engine.history_labels();
        let pick = FontPick {
            family: "Preview Family".into(),
            style: None,
        };
        app.preview_text("propbar_font", Some(LiveText::Font(pick.clone())));
        assert_eq!(text_of(&app, id).0, "Preview Family");
        // The settings stay as they were.
        assert_eq!(app.text_font, font);
        assert_eq!(app.text_settings_shown().0, font);
        // Another family: the preview follows.
        let other = FontPick {
            family: "Other Family".into(),
            style: None,
        };
        app.preview_text("propbar_font", Some(LiveText::Font(other)));
        assert_eq!(text_of(&app, id).0, "Other Family");
        app.end_font_preview();
        assert_eq!(text_of(&app, id), (font.clone(), size));
        assert_eq!(app.engine.history_labels(), history);
        assert!(app.live_font.is_none());
        // Sizes too.
        app.preview_text("size", Some(LiveText::Size(size * 2.0)));
        assert!((text_of(&app, id).1 - size * 2.0).abs() < 1e-6);
        app.preview_text("size", None);
        assert_eq!(text_of(&app, id), (font.clone(), size));
        assert_eq!(app.engine.history_labels(), history);
        // Nothing selected: no preview.
        app.select(Vec::new());
        app.preview_text("propbar_font", Some(LiveText::Font(pick)));
        assert!(app.live_font.is_none());
        assert_eq!(text_of(&app, id).0, font);
    }
}
