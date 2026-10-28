//! File > Document Properties: the drawing's language, title, subject,
//! author, copyright, tags, notes and rating, edited until OK, over a
//! scrolling report of the file, the document, its colour settings and
//! what it holds (objects, text, bitmaps, styles, effects, fills,
//! outlines, symbols). See docs/behavior/document-properties.md.

use crate::app::App;
use crate::doc_stats::DocStats;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Context, RichText, Ui};
use tracedraw_core::Command;

/// Languages offered, as their tag and their own name.
pub const LANGUAGES: [(&str, &str); 30] = [
    ("ar", "العربية"),
    ("bn", "বাংলা"),
    ("cs-CZ", "Čeština"),
    ("da-DK", "Dansk"),
    ("de-DE", "Deutsch (Deutschland)"),
    ("el-GR", "Ελληνικά"),
    ("en-GB", "English (United Kingdom)"),
    ("en-US", "English (United States)"),
    ("es-ES", "Español (España)"),
    ("es-MX", "Español (México)"),
    ("fi-FI", "Suomi"),
    ("fr-FR", "Français (France)"),
    ("he-IL", "עברית"),
    ("hi-IN", "हिन्दी"),
    ("hr-HR", "Hrvatski"),
    ("hu-HU", "Magyar"),
    ("id-ID", "Bahasa Indonesia"),
    ("it-IT", "Italiano"),
    ("ja-JP", "日本語"),
    ("ko-KR", "한국어"),
    ("nl-NL", "Nederlands"),
    ("no-NO", "Norsk"),
    ("pl-PL", "Polski"),
    ("pt-BR", "Português (Brasil)"),
    ("pt-PT", "Português (Portugal)"),
    ("ru-RU", "Русский"),
    ("sv-SE", "Svenska"),
    ("tr-TR", "Türkçe"),
    ("zh-CN", "中文 (简体)"),
    ("zh-TW", "中文 (繁體)"),
];

/// The fields being edited, applied on OK.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocPropsState {
    pub loaded: bool,
    pub language: String,
    pub title: String,
    pub subject: String,
    pub author: String,
    pub copyright: String,
    pub tags: String,
    pub notes: String,
    pub rating: u8,
}

impl DocPropsState {
    fn load(&mut self, app: &App) {
        let md = &app.doc().metadata;
        *self = DocPropsState {
            loaded: true,
            language: md.language.clone(),
            title: app.doc().title.clone(),
            subject: md.subject.clone(),
            author: md.author.clone(),
            copyright: md.copyright.clone(),
            tags: md.keywords.clone(),
            notes: md.notes.clone(),
            rating: md.rating,
        };
    }
}

fn language_name(tag: &str) -> String {
    LANGUAGES
        .iter()
        .find(|(t, _)| *t == tag)
        .map(|(_, n)| n.to_string())
        .unwrap_or_else(|| tag.to_string())
}

fn rating_label(r: u8) -> String {
    if r == 0 {
        tr("docprops.rating_none")
    } else {
        "\u{2605}".repeat(r as usize)
    }
}

/// A section title in the accent colour with a line to the right edge.
fn heading(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).color(Tokens::ACCENT));
        let r = ui.available_rect_before_wrap();
        let y = r.center().y;
        ui.painter().line_segment(
            [
                egui::pos2(r.left() + 6.0, y),
                egui::pos2(r.right() - 4.0, y),
            ],
            egui::Stroke::new(1.0, Tokens::ACCENT),
        );
    });
}

/// One report line: the label, then its value at a fixed column.
fn line(ui: &mut Ui, label: &str, value: impl Into<String>) {
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(170.0, 18.0),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.set_min_width(170.0);
                ui.label(label);
            },
        );
        ui.add(egui::Label::new(value.into()).wrap());
    });
}

fn note(ui: &mut Ui, key: &str) {
    ui.label(RichText::new(tr(key)).color(Tokens::text_dim()));
}

fn n(v: usize) -> String {
    v.to_string()
}

/// The scrolling report under the fields.
fn report(app: &App, ui: &mut Ui, st: &DocStats) {
    let doc = app.doc();
    let md = &doc.metadata;
    let c = &app.settings.color;
    let units = app.units;
    heading(ui, &tr("docprops.sec_file"));
    line(
        ui,
        &tr("docprops.name_location"),
        app.file
            .as_ref()
            .map(|f| f.display().to_string())
            .unwrap_or_else(|| trf("docprops.unsaved", &[("name", &doc.title)])),
    );
    heading(ui, &tr("docprops.sec_document"));
    line(ui, &tr("docprops.pages"), n(doc.pages.len()));
    line(ui, &tr("docprops.layers"), n(st.layers));
    let size = app.page_size();
    let dpi = if md.resolution_dpi > 0.0 {
        md.resolution_dpi
    } else {
        app.settings.default_dpi
    };
    let px = |mm: f64| mm / 25.4 * dpi;
    line(
        ui,
        &tr("newdoc.page_size"),
        format!(
            "{:.2} x {:.2} {} ({:.0} x {:.0} px)",
            units.from_mm(size.width),
            units.from_mm(size.height),
            units.short(),
            px(size.width),
            px(size.height)
        ),
    );
    line(
        ui,
        &tr("newdoc.orientation"),
        if size.width > size.height {
            tr("dialog.landscape")
        } else {
            tr("dialog.portrait")
        },
    );
    line(ui, &tr("newdoc.resolution"), format!("{dpi:.0}"));
    heading(ui, &tr("docprops.sec_color"));
    let or = |s: &str, d: &str| {
        if s.trim().is_empty() {
            d.to_string()
        } else {
            s.to_string()
        }
    };
    line(
        ui,
        &tr("color_mgmt.rgb_profile"),
        or(&md.rgb_profile, &c.rgb_profile),
    );
    line(
        ui,
        &tr("color_mgmt.cmyk_profile"),
        doc.cmyk_profile
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| or(&md.cmyk_profile, &c.cmyk_profile)),
    );
    line(
        ui,
        &tr("color_mgmt.gray_profile"),
        or(&md.gray_profile, &c.gray_profile),
    );
    line(
        ui,
        &tr("color_mgmt.primary_mode"),
        if md.primary_color_mode == tracedraw_core::document::PrimaryColorMode::Rgb {
            "RGB"
        } else {
            "CMYK"
        },
    );
    let intent = or(&md.rendering_intent, &c.intent);
    line(
        ui,
        &tr("color_mgmt.intent"),
        match intent.as_str() {
            "Absolute colorimetric" => tr("color_mgmt.intent_absolute"),
            "Perceptual" => tr("color_mgmt.intent_perceptual"),
            "Saturation" => tr("color_mgmt.intent_saturation"),
            _ => tr("color_mgmt.intent_relative"),
        },
    );
    heading(ui, &tr("docprops.sec_objects"));
    line(ui, &tr("docprops.objects"), n(st.objects));
    line(ui, &tr("docprops.points"), n(st.points));
    line(ui, &tr("docprops.max_points"), n(st.max_points));
    line(ui, &tr("docprops.max_subpaths"), n(st.max_subpaths));
    for (key, v) in [
        ("docprops.curves", st.curves),
        ("docprops.rectangles", st.rectangles),
        ("docprops.ellipses", st.ellipses),
        ("docprops.polygons", st.polygons),
        ("docprops.groups", st.groups),
        ("docprops.tables", st.tables),
    ] {
        if v > 0 {
            line(ui, &tr(key), n(v));
        }
    }
    heading(ui, &tr("docprops.sec_text"));
    if st.artistic_texts + st.paragraph_texts == 0 {
        note(ui, "docprops.no_text");
    } else {
        line(ui, &tr("docprops.artistic_texts"), n(st.artistic_texts));
        line(ui, &tr("docprops.paragraph_texts"), n(st.paragraph_texts));
        line(ui, &tr("docprops.characters"), n(st.characters));
        line(ui, &tr("docprops.words"), n(st.words));
        line(
            ui,
            &tr("docprops.fonts_used"),
            st.fonts.iter().cloned().collect::<Vec<_>>().join(", "),
        );
    }
    heading(ui, &tr("docprops.sec_bitmaps"));
    if st.bitmaps == 0 {
        note(ui, "docprops.no_bitmaps");
    } else {
        line(ui, &tr("docprops.bitmaps"), n(st.bitmaps));
        line(
            ui,
            &tr("docprops.total_pixels"),
            st.bitmap_pixels.to_string(),
        );
        if let Some((w, h)) = st.largest_bitmap {
            line(ui, &tr("docprops.largest_bitmap"), format!("{w} x {h} px"));
        }
    }
    heading(ui, &tr("docprops.sec_styles"));
    if st.object_styles + st.color_styles == 0 {
        note(ui, "docprops.no_styles");
    } else {
        line(ui, &tr("docprops.object_styles"), n(st.object_styles));
        line(ui, &tr("docprops.color_styles"), n(st.color_styles));
    }
    heading(ui, &tr("docprops.sec_effects"));
    if st.effects.is_empty() {
        note(ui, "docprops.no_effects");
    } else {
        for (key, v) in &st.effects {
            line(ui, &tr(key), n(*v));
        }
    }
    heading(ui, &tr("docprops.sec_fills"));
    for (i, key) in [
        "docprops.fill_none",
        "docprops.fill_uniform",
        "docprops.fill_fountain",
        "docprops.fill_pattern",
        "docprops.fill_texture",
        "docprops.fill_mesh",
    ]
    .iter()
    .enumerate()
    {
        if st.fills[i] > 0 {
            line(ui, &tr(key), n(st.fills[i]));
        }
    }
    heading(ui, &tr("docprops.sec_outlines"));
    for (i, key) in [
        "docprops.outline_none",
        "docprops.outline_solid",
        "docprops.outline_dashed",
    ]
    .iter()
    .enumerate()
    {
        if st.outlines[i] > 0 {
            line(ui, &tr(key), n(st.outlines[i]));
        }
    }
    if !st.color_models.is_empty() {
        ui.label(tr("docprops.color_models"));
        for (m, v) in &st.color_models {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                line(ui, m, n(*v));
            });
        }
    }
    heading(ui, &tr("docprops.sec_symbols"));
    if st.symbols + st.symbol_instances == 0 {
        note(ui, "docprops.no_symbols");
    } else {
        line(ui, &tr("docprops.symbols_defined"), n(st.symbols));
        line(ui, &tr("docprops.symbol_instances"), n(st.symbol_instances));
    }
    ui.add_space(6.0);
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut DocPropsState, close: &mut bool) {
    if !st.loaded {
        st.load(app);
    }
    let stats = crate::doc_stats::stats(app.doc());
    let mut ok = false;
    let mut help = false;
    let size = egui::vec2(430.0, 780.0);
    let closed = crate::ui::chrome::dialog(
        ctx,
        "document_properties",
        &tr("docprops.title"),
        size,
        |ui| {
            let area = ui.max_rect();
            let inner = egui::Rect::from_min_max(
                area.min + egui::vec2(12.0, 10.0),
                egui::pos2(area.max.x - 14.0, area.max.y - 52.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                egui::Grid::new("docprops_fields")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        let w = inner.width() - 90.0;
                        ui.label(tr("options.language"));
                        egui::ComboBox::from_id_salt("docprops_lang")
                            .width(w - 8.0)
                            .selected_text(if st.language.is_empty() {
                                tr("docprops.rating_none")
                            } else {
                                language_name(&st.language)
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut st.language,
                                    String::new(),
                                    tr("docprops.rating_none"),
                                );
                                for (tag, name) in LANGUAGES {
                                    ui.selectable_value(&mut st.language, tag.to_string(), name);
                                }
                            });
                        ui.end_row();
                        for (key, v) in [
                            ("dialog.title", &mut st.title),
                            ("dialog.subject", &mut st.subject),
                            ("dialog.author", &mut st.author),
                            ("dialog.copyright", &mut st.copyright),
                            ("docprops.tags", &mut st.tags),
                        ] {
                            ui.label(tr(key));
                            ui.add(egui::TextEdit::singleline(v).desired_width(w));
                            ui.end_row();
                        }
                        ui.label(tr("dialog.notes"));
                        ui.add(
                            egui::TextEdit::multiline(&mut st.notes)
                                .id(crate::ui::dialogs::multiline_id(1))
                                .desired_rows(3)
                                .desired_width(w),
                        );
                        ui.end_row();
                        ui.label(tr("dialog.rating"));
                        egui::ComboBox::from_id_salt("docprops_rating")
                            .width(w - 8.0)
                            .selected_text(rating_label(st.rating))
                            .show_ui(ui, |ui| {
                                for r in 0..=5u8 {
                                    ui.selectable_value(&mut st.rating, r, rating_label(r));
                                }
                            });
                        ui.end_row();
                    });
                ui.add_space(6.0);
                crate::ui::color_management::tab_page()
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.set_width(inner.width() - 18.0);
                        ui.set_max_width(inner.width() - 18.0);
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(ui.available_height() - 8.0)
                            .show(ui, |ui| report(app, ui, &stats));
                    });
            });
            let buttons = egui::Rect::from_min_size(
                egui::pos2(area.min.x + 12.0, area.max.y - 42.0),
                egui::vec2(area.width() - 24.0, 27.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(buttons), |ui| {
                ui.horizontal(|ui| {
                    help = ui
                        .add_sized([27.0, 27.0], egui::Button::new("?"))
                        .rich_tip(tr("dialog.help"))
                        .clicked();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.cancel")))
                            .clicked()
                        {
                            *close = true;
                        }
                        if ui
                            .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.ok")))
                            .clicked()
                            || crate::ui::dialogs::enter_pressed(ui)
                        {
                            ok = true;
                            *close = true;
                        }
                    });
                });
            });
        },
    );
    if closed {
        *close = true;
    }
    if help {
        app.open_dialog_help();
    }
    if ok {
        apply(app, st);
    }
}

/// OK: the title and the properties change in one undo step, only when
/// something changed.
fn apply(app: &mut App, st: &DocPropsState) {
    let mut md = app.doc().metadata.clone();
    md.language = st.language.clone();
    md.subject = st.subject.clone();
    md.author = st.author.clone();
    md.copyright = st.copyright.clone();
    md.keywords = st.tags.clone();
    md.notes = st.notes.clone();
    md.rating = st.rating;
    let mut cmds = Vec::new();
    if st.title != app.doc().title {
        cmds.push(Command::SetTitle {
            title: st.title.clone(),
        });
    }
    if md != app.doc().metadata {
        cmds.push(Command::SetMetadata { metadata: md });
    }
    if cmds.is_empty() {
        return;
    }
    if let Err(e) = app.engine.run_batch("Document Properties", &cmds) {
        app.status = e.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;

    fn frame(ctx: &Context, app: &mut App, enter: bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        if enter {
            input.events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, ui.ctx()));
        out.textures_delta.clear();
    }

    #[test]
    fn ok_stores_the_fields_in_one_step_and_cancel_keeps_them() {
        let mut app = App::headless();
        let mut st = DocPropsState::default();
        st.load(&app);
        st.language = "pt-BR".into();
        st.title = "Poster".into();
        st.author = "Ana".into();
        st.tags = "sale, poster".into();
        st.rating = 4;
        apply(&mut app, &st);
        let md = &app.doc().metadata;
        assert_eq!(md.language, "pt-BR");
        assert_eq!(md.author, "Ana");
        assert_eq!(md.keywords, "sale, poster");
        assert_eq!(md.rating, 4);
        assert_eq!(app.doc().title, "Poster");
        assert_eq!(app.engine.undo_label(), Some("Document Properties"));
        app.undo();
        assert_eq!(app.doc().metadata.author, "");
        assert_ne!(app.doc().title, "Poster");
        // Nothing changed: no undo step.
        let before = app.engine.revision();
        let mut same = DocPropsState::default();
        same.load(&app);
        apply(&mut app, &same);
        assert_eq!(app.engine.revision(), before);
        // The dialog draws, and Enter is OK.
        app.dialog = Dialog::DocumentProperties(Box::default());
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        assert!(matches!(app.dialog, Dialog::None));
    }

    #[test]
    fn languages_have_tags_and_names() {
        assert_eq!(language_name("en-US"), "English (United States)");
        assert_eq!(language_name("xx"), "xx");
        assert_eq!(rating_label(3), "\u{2605}\u{2605}\u{2605}");
    }
}
