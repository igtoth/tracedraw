//! File > New From Template: a search field and two lists (where the
//! templates come from, their category) above a grid of thumbnails, a
//! details pane at the right, a thumbnail size slider and Show template
//! details at the bottom. See docs/behavior/new-from-template.md.

use crate::ui::tips::RichTip;
use egui::{Color32, Context, Sense, Ui, Vec2};

use crate::app::App;
use crate::i18n::tr;
use crate::templates::{self, Category, Source, Template};
use crate::theme::Tokens;

/// Thumbnails are drawn once at this size and scaled.
const THUMB_PX: u32 = 192;

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateState {
    pub search: String,
    /// None: all templates; Some(false): built in; Some(true): mine.
    pub mine: Option<bool>,
    pub category: Option<Category>,
    pub selected: Option<usize>,
    /// Thumbnail size in points (40 to 160).
    pub size: f32,
    pub details: bool,
    pub list: Vec<Template>,
}

impl TemplateState {
    pub fn new() -> Self {
        let mut list = templates::built_in();
        list.extend(templates::user_templates(user_dir()));
        TemplateState {
            search: String::new(),
            mine: None,
            category: None,
            selected: None,
            size: 64.0,
            details: true,
            list,
        }
    }

    /// The indices of the templates the search and lists let through.
    pub fn shown(&self) -> Vec<usize> {
        (0..self.list.len())
            .filter(|&i| {
                self.list
                    .get(i)
                    .is_some_and(|t| templates::matches(t, &self.search, self.mine, self.category))
            })
            .collect()
    }
}

impl Default for TemplateState {
    fn default() -> Self {
        Self::new()
    }
}

/// The folder of the user's templates.
pub fn user_dir() -> Option<std::path::PathBuf> {
    crate::settings::config_dir().map(|d| d.join("templates"))
}

/// The thumbnail of template `t`, drawn once per session.
fn thumbnail(ctx: &Context, t: &Template) -> Option<egui::TextureHandle> {
    let key = egui::Id::new(("template_thumb", format!("{:?}", t.source)));
    if let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(tex);
    }
    let doc = templates::document(t, "")?;
    let (w, h, rgba) = templates::thumbnail(&doc, THUMB_PX)?;
    let image = egui::ColorImage::from_rgba_premultiplied([w as usize, h as usize], &rgba);
    let tex = ctx.load_texture(
        format!("tpl:{:?}", t.source),
        image,
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(key, tex.clone()));
    Some(tex)
}

/// Open template `i` of the list as a new drawing.
fn open(app: &mut App, st: &TemplateState, i: usize) -> bool {
    let Some(t) = st.list.get(i) else {
        return false;
    };
    let name = app.next_untitled_name();
    match templates::document(t, &name) {
        Some(doc) => {
            app.open_document(doc, None);
            app.show_welcome = false;
            true
        }
        None => {
            app.status = crate::i18n::trf("status.template_unreadable", &[("n", &t.name)]);
            false
        }
    }
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut TemplateState, close: &mut bool) {
    let title = tr("menu.file.new_from_template");
    let title = title.trim_end_matches(['.', '\u{2026}']).to_string();
    let mut open_now: Option<usize> = None;
    crate::ui::chrome::window(title).show(ctx, |ui| {
        ui.set_width(820.0);
        // Search and the two lists.
        ui.horizontal(|ui| {
            ui.add_sized(
                [380.0, 24.0],
                egui::TextEdit::singleline(&mut st.search).hint_text(tr("tpl.search")),
            );
            let sources = [
                (None, tr("tpl.all_templates")),
                (Some(false), tr("tpl.built_in")),
                (Some(true), tr("tpl.my_templates")),
            ];
            let current = sources
                .iter()
                .find(|(m, _)| *m == st.mine)
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            egui::ComboBox::from_id_salt("tpl_source")
                .width(190.0)
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for (m, n) in &sources {
                        ui.selectable_value(&mut st.mine, *m, n);
                    }
                });
            let current = st
                .category
                .map(|c| tr(c.key()))
                .unwrap_or_else(|| tr("tpl.all_categories"));
            egui::ComboBox::from_id_salt("tpl_category")
                .width(190.0)
                .selected_text(current)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut st.category, None, tr("tpl.all_categories"));
                    for c in Category::ALL {
                        ui.selectable_value(&mut st.category, Some(c), tr(c.key()));
                    }
                });
        });
        ui.add_space(6.0);
        let shown = st.shown();
        if st.selected.is_some_and(|s| !shown.contains(&s)) {
            st.selected = None;
        }
        let height = 420.0;
        ui.horizontal_top(|ui| {
            let grid_w = if st.details { 540.0 } else { 820.0 };
            egui::Frame::new()
                .fill(Tokens::panel())
                .stroke(egui::Stroke::new(1.0, Tokens::border()))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(grid_w - 14.0, height));
                    ui.set_max_width(grid_w - 14.0);
                    egui::ScrollArea::vertical()
                        .id_salt("tpl_grid")
                        .max_height(height)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                                for &i in &shown {
                                    if let Some(picked) = cell(ui, ctx, st, i) {
                                        st.selected = Some(i);
                                        if picked {
                                            open_now = Some(i);
                                        }
                                    }
                                }
                                if shown.is_empty() {
                                    ui.label(
                                        egui::RichText::new(tr("tpl.none_found"))
                                            .color(Tokens::text_dim()),
                                    );
                                }
                            });
                        });
                });
            if st.details {
                details(ui, ctx, st, height);
            }
        });
        ui.add_space(6.0);
        // Browse for a file, the user's folder, the size slider.
        ui.horizontal(|ui| {
            if ui
                .button("\u{1f4c2}")
                .rich_tip(match user_dir().filter(|_| !crate::files::WEB) {
                    Some(dir) => format!(
                        "{}\n{}",
                        tr("welcome.open_template"),
                        crate::i18n::trf(
                            "welcome.templates_folder",
                            &[("d", &dir.display().to_string())],
                        )
                    ),
                    None => tr("welcome.open_template"),
                })
                .clicked()
            {
                crate::files::Dialog::new()
                    .add_filter(tr("file.template"), &["tdt", "tdraw"])
                    .pick_file(app, |app, p| app.open_as_template(p));
                *close = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().slider_width = 120.0;
                ui.add(egui::Slider::new(&mut st.size, 40.0..=160.0).show_value(false))
                    .rich_tip(tr("tpl.thumbnail_size"));
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.checkbox(&mut st.details, tr("tpl.show_details"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized([100.0, 27.0], egui::Button::new(tr("dialog.cancel")))
                    .clicked()
                {
                    *close = true;
                }
                let can = st.selected.is_some();
                if ui
                    .add_enabled(
                        can,
                        egui::Button::new(tr("tpl.open")).min_size(Vec2::new(100.0, 27.0)),
                    )
                    .clicked()
                {
                    open_now = st.selected;
                }
            });
        });
        if st.selected.is_some() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            open_now = st.selected;
        }
    });
    if let Some(i) = open_now {
        if open(app, st, i) {
            *close = true;
        }
    }
}

/// One thumbnail with its name. Some(true) on a double click, Some(false)
/// on a click.
fn cell(ui: &mut Ui, ctx: &Context, st: &TemplateState, i: usize) -> Option<bool> {
    let t = st.list.get(i)?;
    let side = st.size;
    let w = side + 16.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, side + 38.0), Sense::click());
    let selected = st.selected == Some(i);
    if selected || resp.hovered() {
        ui.painter().rect_filled(
            rect,
            3.0,
            if selected {
                Tokens::tool_active()
            } else {
                Tokens::tool_hover()
            },
        );
    }
    let area = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 4.0 + side / 2.0),
        Vec2::splat(side),
    );
    if let Some(tex) = thumbnail(ctx, t) {
        let s = tex.size_vec2();
        let scale = (side / s.x.max(s.y)).min(4.0);
        let r = egui::Rect::from_center_size(area.center(), s * scale);
        ui.painter().rect_filled(
            r.translate(Vec2::splat(2.0)),
            0.0,
            Color32::from_black_alpha(40),
        );
        ui.painter().image(
            tex.id(),
            r,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.painter().rect_stroke(
            r,
            0.0,
            egui::Stroke::new(1.0, Tokens::border()),
            egui::StrokeKind::Outside,
        );
    }
    let name_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 2.0, area.bottom() + 4.0),
        egui::pos2(rect.right() - 2.0, rect.bottom()),
    );
    let galley = ui.painter().layout(
        t.name.clone(),
        egui::FontId::proportional(11.0),
        Tokens::text(),
        name_rect.width(),
    );
    let pos = egui::pos2(
        name_rect.center().x - galley.size().x / 2.0,
        name_rect.top(),
    );
    ui.painter()
        .with_clip_rect(name_rect)
        .galley(pos, galley, Tokens::text());
    let resp = resp.rich_tip(&t.name);
    if resp.double_clicked() {
        Some(true)
    } else if resp.clicked() {
        Some(false)
    } else {
        None
    }
}

/// The details pane: a large thumbnail, the name, category, page size
/// and where it comes from, or "No Selection".
fn details(ui: &mut Ui, ctx: &Context, st: &TemplateState, height: f32) {
    ui.vertical(|ui| {
        ui.set_width(260.0);
        ui.set_min_height(height);
        let Some(t) = st.selected.and_then(|i| st.list.get(i)) else {
            ui.add_space(height / 2.0 - 10.0);
            ui.vertical_centered(|ui| ui.label(tr("tpl.no_selection")));
            return;
        };
        if let Some(tex) = thumbnail(ctx, t) {
            let s = tex.size_vec2();
            let scale = 240.0 / s.x.max(s.y);
            ui.vertical_centered(|ui| {
                ui.add(egui::Image::new((tex.id(), s * scale)));
            });
        }
        ui.add_space(8.0);
        ui.strong(&t.name);
        egui::Grid::new("tpl_details")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(tr("tpl.category"));
                ui.label(tr(t.category.key()));
                ui.end_row();
                let size = match &t.source {
                    Source::BuiltIn(i) => templates::built_in_size(*i),
                    Source::File(_) => {
                        templates::document(t, "").and_then(|d| d.pages.first().map(|p| p.size))
                    }
                };
                if let Some(s) = size {
                    ui.label(tr("tpl.page_size"));
                    ui.label(format!("{:.1} x {:.1} mm", s.width, s.height));
                    ui.end_row();
                }
                ui.label(tr("tpl.source"));
                ui.label(match &t.source {
                    Source::BuiltIn(_) => tr("tpl.built_in"),
                    Source::File(p) => p.display().to_string(),
                });
                ui.end_row();
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_narrow_the_grid_and_drop_a_hidden_selection() {
        let mut st = TemplateState {
            list: templates::built_in(),
            ..TemplateState::new()
        };
        let all = st.shown().len();
        st.category = Some(Category::Flyers);
        assert_eq!(st.shown().len(), 2);
        st.category = None;
        st.mine = Some(true);
        assert!(st.shown().is_empty());
        st.mine = None;
        st.search = "card".into();
        assert!(st.shown().len() < all && !st.shown().is_empty());
    }

    #[test]
    fn opening_a_template_adds_a_drawing() {
        let mut app = App::headless();
        let st = TemplateState {
            list: templates::built_in(),
            ..TemplateState::new()
        };
        let i = st
            .list
            .iter()
            .position(|t| t.category == Category::Forms)
            .expect("invoice");
        assert!(open(&mut app, &st, i));
        assert!(app.has_document());
        let shapes = app.doc().pages[0].layers[0].shapes.len();
        assert!(shapes > 10, "{shapes}");
        assert!((app.doc().pages[0].size.width - 210.0).abs() < 1e-9);
    }
}

/// The templates of `list` as a row of thumbnails with their names (the
/// Welcome Screen's Templates page). Returns the one clicked.
pub fn strip(ui: &mut Ui, list: &[Template]) -> Option<usize> {
    let ctx = ui.ctx().clone();
    let st = TemplateState {
        search: String::new(),
        mine: None,
        category: None,
        selected: None,
        size: 96.0,
        details: false,
        list: list.to_vec(),
    };
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(10.0);
        for i in 0..list.len() {
            if cell(ui, &ctx, &st, i).is_some() {
                picked = Some(i);
            }
        }
    });
    picked
}
