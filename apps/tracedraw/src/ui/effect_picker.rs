//! The Effect Picker (Effects > Effect Picker): a floating window with
//! every bitmap effect as a tile showing it applied to a small landscape.
//! A search field and a group list narrow the tiles; a click applies the
//! effect to the selection (opening its settings when it has any). The
//! last effects used come first under Recent.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Context, Sense, Vec2};

/// Picture side, pixels.
const THUMB: u32 = 64;
const TILE: Vec2 = Vec2::new(92.0, 100.0);
/// New pictures made per frame (the first opening fills in over a few).
const PER_FRAME: usize = 6;
const RECENT: usize = 8;

/// Which tiles show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Recent,
    /// Index into `crate::fx::groups()`.
    Group(usize),
}

#[derive(Debug, Clone, Default)]
pub struct EffectPicker {
    pub open: bool,
    pub search: String,
    pub filter: Filter,
    /// Effect ids, most recent first.
    pub recent: Vec<String>,
    /// A click adds the effect to the selection's Bitmap Effects lens
    /// (what lies beneath changes) instead of to the object itself.
    pub as_lens: bool,
}

impl EffectPicker {
    /// Remember `id` as the most recently used effect.
    pub fn used(&mut self, id: &str) {
        self.recent.retain(|r| r != id);
        self.recent.insert(0, id.to_string());
        self.recent.truncate(RECENT);
    }
}

/// The effects the picker shows for `filter` and `search` (matched,
/// ignoring case, against the effect's and its group's names): (group
/// key, effect id).
pub fn visible(
    filter: Filter,
    search: &str,
    recent: &[String],
    name: impl Fn(&str) -> String,
) -> Vec<(&'static str, &'static str)> {
    let q = search.trim().to_lowercase();
    let groups = crate::fx::groups();
    let all: Vec<(&'static str, &'static str)> = groups
        .iter()
        .flat_map(|(g, fx)| fx.iter().map(move |f| (*g, f.id)))
        .collect();
    let picked: Vec<(&'static str, &'static str)> = match filter {
        Filter::All => all,
        Filter::Recent => recent
            .iter()
            .filter_map(|r| all.iter().find(|(_, id)| id == r).copied())
            .collect(),
        Filter::Group(i) => groups
            .get(i)
            .map(|(g, fx)| fx.iter().map(|f| (*g, f.id)).collect())
            .unwrap_or_default(),
    };
    if q.is_empty() {
        return picked;
    }
    picked
        .into_iter()
        .filter(|(g, id)| {
            name(&format!("fx.{id}")).to_lowercase().contains(&q)
                || name(g).to_lowercase().contains(&q)
                || id.replace('_', " ").contains(&q)
        })
        .collect()
}

pub fn show(app: &mut App, ctx: &Context) {
    if !app.effect_picker.open {
        return;
    }
    let mut open = true;
    let has = !app.selection.is_empty();
    let mut chosen: Option<&'static str> = None;
    egui::Window::new(tr("effect_picker.title"))
        .id(egui::Id::new("effect_picker"))
        .open(&mut open)
        .default_size([520.0, 480.0])
        .default_pos(egui::pos2(260.0, 150.0))
        .min_width(260.0)
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            let picker = &mut app.effect_picker;
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut picker.search)
                        .hint_text(tr("effect_picker.search"))
                        .desired_width(180.0),
                );
                let groups = crate::fx::groups();
                let label = match picker.filter {
                    Filter::All => tr("effect_picker.all"),
                    Filter::Recent => tr("effect_picker.recent"),
                    Filter::Group(i) => groups.get(i).map(|(k, _)| tr(k)).unwrap_or_default(),
                };
                egui::ComboBox::from_id_salt("effect_picker_group")
                    .selected_text(label)
                    .width(150.0)
                    .height(400.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut picker.filter,
                            Filter::All,
                            tr("effect_picker.all"),
                        );
                        ui.selectable_value(
                            &mut picker.filter,
                            Filter::Recent,
                            tr("effect_picker.recent"),
                        );
                        ui.separator();
                        for (i, (key, _)) in groups.iter().enumerate() {
                            ui.selectable_value(&mut picker.filter, Filter::Group(i), tr(key));
                        }
                    });
            });
            ui.checkbox(&mut picker.as_lens, tr("effect_picker.as_lens"))
                .rich_tip(tr("effect_picker.as_lens_tip"));
            if !has {
                ui.label(
                    egui::RichText::new(tr("effect_picker.select_first"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
            }
            ui.separator();
            let tiles = visible(picker.filter, &picker.search, &picker.recent, tr);
            if tiles.is_empty() {
                ui.label(
                    egui::RichText::new(tr("effect_picker.none"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
            }
            let mut made = 0;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                        for (group, id) in tiles {
                            let (rect, resp) = ui.allocate_exact_size(
                                TILE,
                                if has { Sense::click() } else { Sense::hover() },
                            );
                            if !ui.is_rect_visible(rect) {
                                continue;
                            }
                            let painter = ui.painter();
                            if resp.hovered() && has {
                                painter.rect_filled(rect, 4.0, Tokens::tool_hover());
                            }
                            let pic = egui::Rect::from_center_size(
                                egui::pos2(rect.center().x, rect.min.y + 6.0 + THUMB as f32 / 2.0),
                                Vec2::splat(THUMB as f32),
                            );
                            let mut tex =
                                crate::ui::fx_icons::texture_sized(ui.ctx(), id, THUMB, false);
                            if tex.is_none() && made < PER_FRAME {
                                tex = crate::ui::fx_icons::texture_sized(ui.ctx(), id, THUMB, true);
                                made += 1;
                            }
                            match tex {
                                Some(tex) => {
                                    painter.image(
                                        tex,
                                        pic,
                                        egui::Rect::from_min_max(
                                            egui::Pos2::ZERO,
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        if has {
                                            egui::Color32::WHITE
                                        } else {
                                            egui::Color32::from_white_alpha(140)
                                        },
                                    );
                                }
                                None => {
                                    painter.rect_filled(pic, 2.0, Tokens::border());
                                    ui.ctx().request_repaint();
                                }
                            }
                            painter.rect_stroke(
                                pic,
                                2.0,
                                egui::Stroke::new(1.0, Tokens::border()),
                                egui::StrokeKind::Outside,
                            );
                            let galley = painter.layout(
                                tr(&format!("fx.{id}")),
                                egui::FontId::proportional(11.0),
                                if has {
                                    Tokens::text()
                                } else {
                                    Tokens::text_dim()
                                },
                                TILE.x - 4.0,
                            );
                            let at = egui::pos2(
                                rect.center().x - galley.size().x / 2.0,
                                pic.max.y + 3.0,
                            );
                            painter.galley(at, galley, Tokens::text());
                            let resp = resp.rich_tip(tr(group));
                            if resp.clicked() {
                                chosen = Some(id);
                            }
                        }
                    });
                });
        });
    if !open {
        app.effect_picker.open = false;
    }
    if let Some(id) = chosen {
        app.effect_picker.used(id);
        if app.effect_picker.as_lens {
            app.add_lens_effect(id);
        } else {
            crate::ui::effect_dialog::choose_effect(app, id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_groups_and_recent_narrow_the_tiles() {
        let name = |k: &str| crate::i18n::tr(k);
        let all = visible(Filter::All, "", &[], name);
        let total: usize = crate::fx::groups().iter().map(|(_, g)| g.len()).sum();
        assert_eq!(all.len(), total);
        // A group shows its own effects only.
        let blur = crate::fx::groups()
            .iter()
            .position(|(k, _)| *k == "menu.effects.blur")
            .expect("a blur group");
        let blurs = visible(Filter::Group(blur), "", &[], name);
        assert!(!blurs.is_empty() && blurs.iter().all(|(g, _)| *g == "menu.effects.blur"));
        // Search matches effect names and group names, any case.
        let found = visible(Filter::All, "GAUSS", &[], name);
        assert!(
            found.iter().any(|(_, id)| id.contains("gaussian")),
            "{found:?}"
        );
        let by_group = visible(Filter::All, &name("menu.effects.blur"), &[], name);
        assert!(by_group.len() >= blurs.len());
        assert!(visible(Filter::All, "zzzz-nothing", &[], name).is_empty());
        // Recent keeps the order of use, at most eight, no repeats.
        let mut p = EffectPicker::default();
        for (_, id) in all.iter().take(10) {
            p.used(id);
        }
        p.used(all[9].1);
        assert_eq!(p.recent.len(), RECENT);
        assert_eq!(p.recent[0], all[9].1);
        let recent = visible(Filter::Recent, "", &p.recent, name);
        assert_eq!(recent.first().map(|(_, id)| *id), Some(all[9].1));
    }
}
