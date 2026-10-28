//! The Font Sampler docker (Text > Font Sampler): a line of sample text
//! set in several fonts one under the other, to compare them. Type the
//! sample, set its size, add fonts from the list (the text font and the
//! fonts of the drawing come first), and click a sample to give the
//! selected text that font.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Sense, Ui, Vec2};

/// The docker's choices.
#[derive(Debug, Clone)]
pub struct FontSampler {
    pub text: String,
    /// Sample size, pixels to the em.
    pub size: f32,
    pub families: Vec<String>,
    /// The family list's filter.
    pub filter: String,
    /// Set up once with the text font and the drawing's fonts.
    seeded: bool,
}

impl Default for FontSampler {
    fn default() -> Self {
        FontSampler {
            text: String::new(),
            size: 28.0,
            families: Vec::new(),
            filter: String::new(),
            seeded: false,
        }
    }
}

const MAX_FAMILIES: usize = 24;

impl FontSampler {
    /// Add a family to the comparison (once, at most 24).
    pub fn add(&mut self, family: &str) {
        if !family.is_empty()
            && !self.families.iter().any(|f| f == family)
            && self.families.len() < MAX_FAMILIES
        {
            self.families.push(family.to_string());
        }
    }

    /// The first time: the text font, then the drawing's fonts.
    pub fn seed(&mut self, text_font: &str, used: &[String]) {
        if self.seeded {
            return;
        }
        self.seeded = true;
        self.add(text_font);
        for f in used {
            self.add(f);
        }
    }

    /// The sample shown: what was typed, else the default pangram.
    pub fn sample(&self) -> String {
        if self.text.trim().is_empty() {
            tr("font_sampler.default_text")
        } else {
            self.text.clone()
        }
    }
}

pub fn font_sampler(app: &mut App, ui: &mut Ui) {
    let used = app.doc().font_families_used();
    let text_font = app.text_font.clone();
    app.font_sampler.seed(&text_font, &used);
    app.font_previews.refresh();
    ui.add(
        egui::TextEdit::multiline(&mut app.font_sampler.text)
            .hint_text(tr("font_sampler.default_text"))
            .desired_rows(2)
            .desired_width(f32::INFINITY),
    )
    .rich_tip(crate::ui::tips::keyed("font_sampler.text"));
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr("font_sampler.size"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        ui.add(crate::ui::Rail(
            egui::Slider::new(&mut app.font_sampler.size, 10.0..=96.0).suffix(" px"),
        ))
        .rich_tip(crate::ui::tips::pair(
            "font_sampler.size",
            "font_sampler.size_desc",
        ));
    });
    // Add a font: type to narrow the list, click to add.
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr("font_sampler.add"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.font_sampler.filter)
                .hint_text(tr("font_sampler.find"))
                .desired_width(150.0),
        )
        .rich_tip(crate::ui::tips::pair(
            "font_sampler.add",
            "font_sampler.add_desc",
        ));
    });
    let q = app.font_sampler.filter.trim().to_lowercase();
    if !q.is_empty() {
        let matches: Vec<String> = app
            .font_families
            .iter()
            .filter(|f| f.to_lowercase().contains(&q))
            .take(8)
            .cloned()
            .collect();
        for f in matches {
            if ui
                .selectable_label(false, format!("+ {f}"))
                .rich_tip_with(|| format!("{f}\n{}", tr("font_sampler.add_one_desc")))
                .clicked()
            {
                app.font_sampler.add(&f);
                app.font_sampler.filter.clear();
            }
        }
    }
    ui.separator();
    let sample = app.font_sampler.sample();
    let size = app.font_sampler.size;
    let mut apply = None;
    let mut remove = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, family) in app.font_sampler.families.clone().iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(family)
                            .color(Tokens::text_dim())
                            .size(11.0),
                    );
                    if app.text_font == *family {
                        ui.label(egui::RichText::new("\u{2713}").size(11.0));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("\u{00D7}")
                            .rich_tip(tr("font_sampler.remove"))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    });
                });
                // A family that is not installed shows in its stand-in.
                let ppp = ui.ctx().pixels_per_point();
                let stand_in = if tracedraw_text::fonts().has_family(family) {
                    None
                } else {
                    tracedraw_text::fonts().substitute(family)
                };
                let shown_family = stand_in.clone().unwrap_or_else(|| family.clone());
                let picture = app.font_previews.picture(
                    ui.ctx(),
                    &shown_family,
                    &sample,
                    (false, false),
                    size * ppp,
                );
                if let Some(s) = &stand_in {
                    ui.label(
                        egui::RichText::new(crate::i18n::trf("font_sampler.shown_in", &[("f", s)]))
                            .color(Tokens::text_dim())
                            .italics()
                            .size(11.0),
                    );
                }
                let resp = match picture {
                    Some(p) => {
                        // On a white card, as the text would print.
                        let show = p.texture.size_vec2() / ppp;
                        let max_w = ui.available_width();
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(max_w, show.y + 10.0), Sense::click());
                        ui.painter().rect_filled(rect, 3.0, egui::Color32::WHITE);
                        if resp.hovered() {
                            ui.painter().rect_stroke(
                                rect,
                                3.0,
                                egui::Stroke::new(1.5, Tokens::ACCENT),
                                egui::StrokeKind::Inside,
                            );
                        }
                        // At the chosen size; what does not fit is cut off.
                        let img = egui::Rect::from_min_size(rect.min + Vec2::new(6.0, 5.0), show);
                        let room = rect.shrink2(Vec2::new(6.0, 0.0));
                        let clip = img.intersect(room);
                        let uv = egui::Rect::from_min_max(
                            egui::Pos2::ZERO,
                            egui::pos2(
                                ((clip.right() - img.left()) / show.x.max(1.0)).clamp(0.0, 1.0),
                                1.0,
                            ),
                        );
                        ui.painter().image(
                            p.texture.id(),
                            clip,
                            uv,
                            crate::ui::font_list::CARD_INK,
                        );
                        resp
                    }
                    None => ui.label(
                        egui::RichText::new(tr("font_sampler.not_installed"))
                            .color(Tokens::text_dim())
                            .italics(),
                    ),
                };
                if resp.rich_tip(tr("font_sampler.apply_tip")).clicked() {
                    apply = Some(family.clone());
                }
                ui.separator();
            }
        });
    if let Some(i) = remove {
        if i < app.font_sampler.families.len() {
            app.font_sampler.families.remove(i);
        }
    }
    if let Some(f) = apply {
        app.apply_text_font(&f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_are_added_once_seeded_once_and_capped() {
        let mut s = FontSampler::default();
        s.seed("Inter", &["Lora".to_string(), "Inter".to_string()]);
        assert_eq!(s.families, ["Inter", "Lora"]);
        s.families.clear();
        s.seed("Other", &[]);
        assert!(s.families.is_empty(), "seeded only the first time");
        for i in 0..40 {
            s.add(&format!("F{i}"));
        }
        s.add("F0");
        s.add("");
        assert_eq!(s.families.len(), MAX_FAMILIES);
        assert_eq!(s.sample(), tr("font_sampler.default_text"));
        s.text = "Hamburgefonstiv".into();
        assert_eq!(s.sample(), "Hamburgefonstiv");
    }
}
