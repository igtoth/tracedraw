//! The AI Generate docker: Create image, Remix image and Create vectors
//! through the AI services the user set up (Tools > Options > AI), with
//! an information page. The work is in `ai_image.rs`; see
//! docs/behavior/ai-generate.md.

use crate::ai_image::{
    Aspect, ImageStatus, Mode, VectorEngine, IDEAS, MAX_COUNT, PALETTES, REMIX_IDEAS, STYLES,
    VECTOR_STYLES,
};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::ai_ui::{banner, Need};
use crate::ui::tips::RichTip;
use egui::{Color32, RichText, Sense, Ui, Vec2};

const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// A section title with a Reset button at the right. True when Reset was
/// pressed.
fn heading(ui: &mut Ui, key: &str, reset: bool) -> bool {
    ui.add_space(6.0);
    let mut pressed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr(key)).strong());
        if reset {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                pressed = ui
                    .small_button("\u{21BA}")
                    .rich_tip(tr("aigen.reset"))
                    .clicked();
            });
        }
    });
    pressed
}

/// The notes of a known model, for the model's help button.
fn model_note(model: &str) -> String {
    let key = if model.contains("pro") && model.contains("gemini") {
        "aigen.model_gemini_pro"
    } else if model.contains("gemini") {
        "aigen.model_gemini"
    } else if model.starts_with("gpt-image") {
        "aigen.model_gpt_image"
    } else if model.starts_with("dall-e") {
        "aigen.model_dalle"
    } else {
        "aigen.model_other"
    };
    tr(key)
}

pub fn ai_generate(app: &mut App, ui: &mut Ui) {
    // The modes as picture buttons, and the information page at the right.
    ui.horizontal(|ui| {
        for (mode, glyph, key) in [
            (Mode::Create, "\u{1F5BC}", "aigen.mode_create"),
            (Mode::Remix, "\u{1F504}", "aigen.mode_remix"),
            (Mode::Vector, "\u{2712}", "aigen.mode_vector"),
            (Mode::Draw, "\u{270D}", "aigen.mode_draw"),
        ] {
            let on = app.ai_image.mode == mode && !app.ai_image.info;
            if ui
                .selectable_label(on, RichText::new(glyph).size(16.0))
                .rich_tip(tr(key))
                .clicked()
            {
                app.ai_image.mode = mode;
                app.ai_image.info = false;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .selectable_label(app.ai_image.info, RichText::new("\u{2139}").size(16.0))
                .rich_tip(tr("aigen.info"))
                .clicked()
            {
                app.ai_image.info = !app.ai_image.info;
            }
        });
    });
    if app.ai_image.info {
        egui::ScrollArea::vertical()
            .id_salt("aigen_info")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 10.0);
                info_page(app, ui)
            });
        return;
    }
    let mode = app.ai_image.mode;
    if mode == Mode::Draw {
        // Draw with words: the drawing service makes and changes objects.
        ui.label(RichText::new(tr("aigen.draw_title")).strong());
        ui.label(
            RichText::new(tr("aigen.draw_about"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        egui::ScrollArea::vertical()
            .id_salt("aigen_draw")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width() - 10.0);
                crate::ui::ai_docker::ai_drawing(app, ui);
            });
        return;
    }
    let svg = mode == Mode::Vector && app.ai_image.vector_engine == VectorEngine::Svg;
    let need = if svg { Need::Drawing } else { Need::Images };
    banner(app, ui, need);
    let ready = if svg {
        app.ai_drawing_ready()
    } else {
        app.ai_image_ready()
    };
    let (title, about) = match mode {
        Mode::Create => ("aigen.create_title", "aigen.create_about"),
        Mode::Remix => ("aigen.remix_title", "aigen.remix_about"),
        Mode::Vector => ("aigen.vector_title", "aigen.vector_about"),
        Mode::Draw => ("aigen.draw_title", "aigen.draw_about"),
    };
    // The settings and results scroll; the footer stays at the bottom.
    let footer_h = 118.0;
    let scroll_h = (ui.available_height() - footer_h).max(120.0);
    egui::ScrollArea::vertical()
        .id_salt("aigen_scroll")
        .max_height(scroll_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - 10.0);
            ui.label(RichText::new(tr(title)).strong());
            ui.label(
                RichText::new(tr(about))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
            ui.add_enabled_ui(ready && !app.ai_image.busy(), |ui| body(app, ui, mode, svg));
            results(app, ui);
            ui.add_space(6.0);
            ui.label(
                RichText::new(tr("aigen.privacy"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
        });
    footer(app, ui, mode, ready);
}

fn body(app: &mut App, ui: &mut Ui, mode: Mode, svg: bool) {
    if mode == Mode::Vector {
        ui.horizontal(|ui| {
            ui.label(tr("aigen.engine"));
            ui.selectable_value(
                &mut app.ai_image.vector_engine,
                VectorEngine::Trace,
                tr("aigen.engine_trace"),
            )
            .rich_tip(tr("aigen.engine_trace_tip"));
            ui.selectable_value(
                &mut app.ai_image.vector_engine,
                VectorEngine::Svg,
                tr("aigen.engine_svg"),
            )
            .rich_tip(tr("aigen.engine_svg_tip"));
        });
    }
    // The model in use, from the service's list, and its notes.
    ui.horizontal(|ui| {
        ui.label(tr("ai.model"));
        if svg {
            ui.label(RichText::new(app.settings.ai.model()).strong());
            return;
        }
        let Some(prefs) = app.ai_image_prefs() else {
            // The service makes drawings only (or has no image model).
            ui.label(RichText::new(tr("aiui.not_ready")).color(Tokens::text_dim()));
            return;
        };
        let current = prefs.model();
        let mut list: Vec<String> = if app.ai_image.models.is_empty() {
            prefs
                .provider
                .preset_models()
                .iter()
                .map(|m| m.to_string())
                .collect()
        } else {
            app.ai_image.models.clone()
        };
        if !list.contains(&current) {
            list.insert(0, current.clone());
        }
        egui::ComboBox::from_id_salt("aigen_model")
            .width((ui.available_width() - 34.0).max(80.0))
            .selected_text(current.clone())
            .show_ui(ui, |ui| {
                // Each model with a line on what it is good at.
                ui.set_min_width(260.0);
                ui.set_max_width(280.0);
                for m in &list {
                    let on = *m == current;
                    let resp = ui
                        .vertical(|ui| {
                            let r = ui.radio(on, RichText::new(m.as_str()).strong());
                            ui.add(
                                egui::Label::new(
                                    RichText::new(model_note(m))
                                        .color(Tokens::text_dim())
                                        .size(11.0),
                                )
                                .wrap(),
                            );
                            r
                        })
                        .inner;
                    if resp.clicked() {
                        app.settings.ai.image_model = m.clone();
                        ui.close();
                    }
                    ui.add_space(3.0);
                }
            });
        ui.small_button("?").rich_tip(model_note(&current));
    });
    // The reference picture: needed to remix, optional otherwise.
    if !svg {
        let key = if mode == Mode::Remix {
            "aigen.source"
        } else {
            "aigen.example"
        };
        if heading(ui, key, app.ai_image.source.is_some()) {
            app.ai_image.source = None;
        }
        source(app, ui, mode);
    }
    // The prompt, with Reset.
    let reset = heading(ui, "aigen.prompt", true);
    let text = match mode {
        Mode::Remix => &mut app.ai_image.remix_prompt,
        _ => &mut app.ai_image.prompt,
    };
    if reset {
        text.clear();
    }
    ui.add(
        egui::TextEdit::multiline(text)
            .id(egui::Id::new("aigen_prompt"))
            .hint_text(tr(match mode {
                Mode::Remix => "aigen.remix_hint",
                _ => "aigen.prompt_hint",
            }))
            .desired_rows(6)
            .desired_width(f32::INFINITY),
    );
    if mode == Mode::Remix {
        // Ready-made requests for the usual remixes.
        ui.horizontal_wrapped(|ui| {
            for (key, prompt) in REMIX_IDEAS {
                if ui.small_button(tr(key)).rich_tip(prompt).clicked() {
                    app.ai_image.remix_prompt = prompt.to_string();
                }
            }
        });
    } else {
        if ui
            .button(format!("\u{1F4A1} {}", tr("aigen.inspire")))
            .clicked()
        {
            let n = app.ai_image.prompt.len() + app.ai_image.results.len() * 7;
            let mut pick = IDEAS[n % IDEAS.len()];
            if pick == app.ai_image.prompt {
                pick = IDEAS[(n + 1) % IDEAS.len()];
            }
            app.ai_image.prompt = pick.to_string();
        }
        ui.horizontal(|ui| {
            ui.label(tr("aigen.aspect"));
            let current = app.ai_image.aspect;
            egui::ComboBox::from_id_salt("aigen_aspect")
                .width(ui.available_width() - 8.0)
                .selected_text(tr(current.key()))
                .show_ui(ui, |ui| {
                    if app.ai_image.source.is_some() {
                        ui.selectable_value(
                            &mut app.ai_image.aspect,
                            Aspect::MatchSource,
                            tr(Aspect::MatchSource.key()),
                        );
                    }
                    for a in Aspect::ALL {
                        ui.selectable_value(&mut app.ai_image.aspect, a, tr(a.key()));
                    }
                });
        });
    }
    // Style and palette fold away to make room for the results.
    let folded = app.ai_image.folded;
    ui.vertical_centered(|ui| {
        if ui
            .small_button(if folded { "\u{2304}" } else { "\u{2303}" })
            .rich_tip(tr(if folded { "aigen.unfold" } else { "aigen.fold" }))
            .clicked()
        {
            app.ai_image.folded = !folded;
        }
    });
    if folded {
        return;
    }
    let vector = mode == Mode::Vector;
    if heading(ui, "aigen.style", true) {
        app.ai_image.style = None;
        app.ai_image.vector_style = None;
    }
    let cols = 3;
    egui::Grid::new("aigen_styles")
        .num_columns(cols)
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            let list: &[(&str, &str)] = if vector { &VECTOR_STYLES } else { &STYLES };
            for (i, (key, _)) in list.iter().enumerate() {
                let current = if vector {
                    app.ai_image.vector_style
                } else {
                    app.ai_image.style
                };
                let on = current == Some(i);
                let swatch = if vector { 6 + i } else { i };
                if tile(ui, &tr(key), on, |p, r| style_swatch(p, r, swatch)).clicked() {
                    let v = if on { None } else { Some(i) };
                    if vector {
                        app.ai_image.vector_style = v;
                    } else {
                        app.ai_image.style = v;
                    }
                }
                if i % cols == cols - 1 {
                    ui.end_row();
                }
            }
        });
    if heading(ui, "aigen.palette", true) {
        app.ai_image.palette = None;
    }
    let cols = 4;
    egui::Grid::new("aigen_palettes")
        .num_columns(cols)
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for (i, (key, _, colours)) in PALETTES.iter().enumerate() {
                let on = app.ai_image.palette == Some(i);
                if tile(ui, &tr(key), on, |p, r| gradient(p, r, colours)).clicked() {
                    app.ai_image.palette = if on { None } else { Some(i) };
                }
                if i % cols == cols - 1 {
                    ui.end_row();
                }
            }
        });
}

/// The reference picture with Use Selection and Import File under it.
fn source(app: &mut App, ui: &mut Ui, mode: Mode) {
    let shown = app.ai_image.source.clone().or_else(|| {
        // Remix starts from the selected bitmap until another is chosen.
        (mode == Mode::Remix)
            .then(|| app.ai_source_bitmap())
            .flatten()
            .and_then(|(id, png)| {
                let img = crate::bitmap_fx::decode(&png)?;
                Some(crate::ai_image::SourceImage {
                    png: png.to_vec(),
                    width: img.width(),
                    height: img.height(),
                    shape: Some(id),
                })
            })
    });
    let w = ui.available_width().min(240.0);
    match shown.as_ref().and_then(|s| texture(ui.ctx(), &s.png)) {
        Some(tex) => {
            let s = tex.size_vec2();
            let scale = (w / s.x).min(150.0 / s.y);
            ui.vertical_centered(|ui| {
                ui.add(egui::Image::new((tex.id(), s * scale)));
            });
        }
        None => {
            ui.vertical_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, 70.0), Sense::hover());
                ui.painter().rect_stroke(
                    r,
                    3.0,
                    egui::Stroke::new(1.0, Tokens::border()),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    tr(if mode == Mode::Remix {
                        "aigen.select_bitmap"
                    } else {
                        "aigen.example_hint"
                    }),
                    egui::FontId::proportional(11.0),
                    Tokens::text_dim(),
                );
            });
        }
    }
    ui.vertical_centered(|ui| {
        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 64.0).max(0.0) / 2.0);
            if ui
                .small_button("\u{2B1A}")
                .rich_tip(tr("aigen.use_selection"))
                .clicked()
            {
                app.ai_source_from_selection();
            }
            if ui
                .small_button("\u{1F4C2}")
                .rich_tip(tr("aigen.import_source"))
                .clicked()
            {
                app.ai_source_from_file();
            }
        });
    });
}

/// The pictures of the last Generate; a click inserts one.
fn results(app: &mut App, ui: &mut Ui) {
    if app.ai_image.results.is_empty() {
        return;
    }
    let mut place = None;
    egui::CollapsingHeader::new(RichText::new(tr("aigen.results")).strong())
        .id_salt("aigen_results")
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let n = app.ai_image.results.len();
                let side = if n == 1 {
                    ui.available_width().min(240.0)
                } else {
                    ((ui.available_width() - 8.0) / 2.0).min(120.0)
                };
                for (i, r) in app.ai_image.results.iter().enumerate() {
                    if let Some(tex) = texture(ui.ctx(), &r.png) {
                        let s = tex.size_vec2();
                        let scale = side / s.x.max(s.y);
                        let resp = ui
                            .add(egui::Image::new((tex.id(), s * scale)).sense(Sense::click()))
                            .rich_tip(tr("aigen.place_again"));
                        if resp.clicked() {
                            place = Some(i);
                        }
                    }
                }
            });
            ui.label(
                RichText::new(tr("aigen.results_note"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
        });
    if let Some(img) = place.and_then(|i| app.ai_image.results.get(i).cloned()) {
        app.ai_place_image(&img, None, false);
    }
}

/// Images, what will be sent, Generate (or the spinner and Cancel) and
/// the outcome.
fn footer(app: &mut App, ui: &mut Ui, mode: Mode, ready: bool) {
    ui.separator();
    let svg = mode == Mode::Vector && app.ai_image.vector_engine == VectorEngine::Svg;
    ui.add_enabled_ui(ready && !app.ai_image.busy(), |ui| {
        ui.horizontal(|ui| {
            if !svg {
                ui.label(tr("aigen.images"));
                ui.add(
                    crate::ui::field::NumField::new(&mut app.ai_image.count).range(1..=MAX_COUNT),
                );
            }
            ui.checkbox(&mut app.ai_image.auto_place, tr("aigen.auto_place"))
                .rich_tip(tr("aigen.auto_place_tip"));
        });
    });
    let service = match app.ai_image_prefs() {
        Some(p) if !svg => tr(p.provider.key()),
        _ => tr(app.settings.ai.provider.key()),
    };
    let requests = if svg {
        1
    } else {
        app.ai_image.count.clamp(1, MAX_COUNT) as usize
    };
    ui.label(
        RichText::new(trf(
            "aigen.requests",
            &[("n", &requests.to_string()), ("service", &service)],
        ))
        .color(Tokens::text_dim())
        .size(11.0),
    );
    if app.ai_image.busy() {
        // The button says it is working, with the progress and Cancel.
        let label = match app.ai_image.status {
            ImageStatus::Waiting { done, total } if total > 1 => trf(
                "aigen.waiting",
                &[("done", &done.to_string()), ("total", &total.to_string())],
            ),
            _ => tr("aigen.in_progress"),
        };
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 90.0).max(80.0);
            ui.add_enabled(
                false,
                egui::Button::new(format!("\u{2726} {label}")).min_size(Vec2::new(w, 30.0)),
            );
            ui.spinner();
            if ui.button(tr("dialog.cancel")).clicked() {
                app.ai_image_cancel();
            }
        });
    } else {
        let has_source = app.ai_image.source.is_some() || app.ai_source_bitmap().is_some();
        let can = ready
            && match mode {
                Mode::Remix => has_source && !app.ai_image.remix_prompt.trim().is_empty(),
                _ => !app.ai_image.prompt.trim().is_empty(),
            };
        let button =
            egui::Button::new(RichText::new(format!("\u{2726} {}", tr("aigen.generate"))).strong())
                .min_size(Vec2::new(ui.available_width(), 30.0));
        if ui.add_enabled(can, button).clicked() {
            app.ai_generate();
        }
    }
    match &app.ai_image.status {
        ImageStatus::Done(n) => {
            ui.label(trf("aigen.done", &[("n", &n.to_string())]));
        }
        ImageStatus::Failed(e) => {
            ui.label(RichText::new(e.as_str()).color(ERROR));
        }
        _ => {}
    }
}

/// The information page: what AI Generate does, where each service
/// stands, the models, what is sent and who bills it.
fn info_page(app: &mut App, ui: &mut Ui) {
    ui.label(RichText::new(tr("aigen.info_title")).strong().size(14.0));
    ui.add_space(4.0);
    for key in [
        "aigen.info_about",
        "aigen.info_create",
        "aigen.info_remix",
        "aigen.info_vector",
        "aigen.info_cutout",
    ] {
        ui.label(tr(key));
        ui.add_space(3.0);
    }
    ui.add_space(4.0);
    ui.label(RichText::new(tr("aiui.services")).strong());
    let pictures = app.ai_image_prefs();
    let offered = app.settings.ai.can_make_pictures();
    for (key, provider, model, ready, offered) in [
        (
            "aiui.pictures",
            pictures
                .as_ref()
                .map(|p| tr(p.provider.key()))
                .unwrap_or_else(|| tr(app.settings.ai.provider.key())),
            pictures.as_ref().map(|p| p.model()).unwrap_or_default(),
            app.ai_image_ready(),
            offered,
        ),
        (
            "aiui.drawings",
            tr(app.settings.ai.provider.key()),
            app.settings.ai.model(),
            app.ai_drawing_ready(),
            true,
        ),
    ] {
        let state = if !offered {
            tr("aiui.drawings_only_state")
        } else if ready {
            tr("aiui.ready")
        } else {
            tr("aiui.not_ready")
        };
        let line = if model.is_empty() {
            format!("{} {state}", tr(key))
        } else {
            format!("{} {provider} \u{2022} {model} \u{2022} {state}", tr(key))
        };
        ui.add(egui::Label::new(line).wrap());
    }
    ui.add_space(4.0);
    ui.label(RichText::new(tr("aigen.info_costs_title")).strong());
    ui.label(tr("aigen.info_costs"));
    ui.add_space(4.0);
    ui.label(RichText::new(tr("aiui.privacy_title")).strong());
    ui.label(RichText::new(tr("aiui.privacy")).color(Tokens::text_dim()));
    ui.add_space(6.0);
    if ui
        .button(tr("aiui.set_up"))
        .rich_tip(crate::ui::tips::keyed("aiui.set_up"))
        .clicked()
    {
        crate::ui::ai_ui::open_options(app, crate::ui::dialogs::OptionsPage::AiService);
    }
}

/// A picture's texture, made once.
fn texture(ctx: &egui::Context, png: &[u8]) -> Option<egui::TextureHandle> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    png.hash(&mut h);
    let key = egui::Id::new(("aigen_tex", h.finish()));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(t);
    }
    let img = crate::bitmap_fx::decode(png)?;
    // Thumbnails stay small.
    let img = if img.width().max(img.height()) > 256 {
        let s = 256.0 / img.width().max(img.height()) as f32;
        crate::bitmap_fx::resample(
            &img,
            ((img.width() as f32 * s) as u32).max(1),
            ((img.height() as f32 * s) as u32).max(1),
        )
    } else {
        img
    };
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        img.as_raw(),
    );
    let t = ctx.load_texture("aigen", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, t.clone()));
    Some(t)
}

/// A square tile with a picture and a name under it.
fn tile(
    ui: &mut Ui,
    name: &str,
    on: bool,
    paint: impl FnOnce(&egui::Painter, egui::Rect),
) -> egui::Response {
    let w = ((ui.available_width() - 16.0) / 3.0).clamp(56.0, 84.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, w * 0.62 + 16.0), Sense::click());
    let pic = egui::Rect::from_min_size(rect.min, Vec2::new(w, w * 0.62));
    let p = ui.painter();
    paint(p, pic);
    let stroke = if on {
        egui::Stroke::new(2.0, Tokens::ACCENT)
    } else if resp.hovered() {
        egui::Stroke::new(1.0, Tokens::ACCENT)
    } else {
        egui::Stroke::new(1.0, Tokens::border())
    };
    p.rect_stroke(pic, 2.0, stroke, egui::StrokeKind::Inside);
    p.text(
        egui::pos2(rect.center().x, pic.bottom() + 2.0),
        egui::Align2::CENTER_TOP,
        name,
        egui::FontId::proportional(10.0),
        if on { Tokens::ACCENT } else { Tokens::text() },
    );
    resp.rich_tip(name)
}

fn rgb(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// A swatch of colours left to right.
fn gradient(p: &egui::Painter, r: egui::Rect, colours: &[u32; 3]) {
    // One mesh: three columns of vertices, colours blended between.
    let mut mesh = egui::Mesh::default();
    for (i, c) in colours.iter().enumerate() {
        let x = r.left() + r.width() * i as f32 / 2.0;
        mesh.colored_vertex(egui::pos2(x, r.top()), rgb(*c));
        mesh.colored_vertex(egui::pos2(x, r.bottom()), rgb(*c));
    }
    for i in 0..2u32 {
        let a = i * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    p.add(egui::Shape::mesh(mesh));
}

/// A small picture of each style: the same leaf drawn its way.
fn style_swatch(p: &egui::Painter, r: egui::Rect, i: usize) {
    let c = r.center();
    let s = r.height() * 0.32;
    let leaf = |fill: Color32, stroke: egui::Stroke| {
        let pts: Vec<egui::Pos2> = (0..24)
            .map(|k| {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                egui::pos2(c.x + a.cos() * s * 1.6, c.y + a.sin() * s * 0.9)
            })
            .collect();
        p.add(egui::Shape::convex_polygon(pts, fill, stroke));
    };
    match i {
        // Photorealistic: soft light background, green leaf.
        0 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0x9C, 0xB8, 0x8A));
            leaf(Color32::from_rgb(0x3E, 0x7A, 0x2E), egui::Stroke::NONE);
        }
        // Line art.
        1 => {
            p.rect_filled(r, 0.0, Color32::WHITE);
            leaf(Color32::TRANSPARENT, egui::Stroke::new(1.2, Color32::BLACK));
        }
        // Watercolour.
        2 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xF6, 0xF0, 0xE4));
            leaf(
                Color32::from_rgba_unmultiplied(0x6A, 0xA8, 0x6A, 150),
                egui::Stroke::NONE,
            );
        }
        // Illustration.
        3 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xF9, 0xD5, 0x6E));
            leaf(
                Color32::from_rgb(0x2E, 0x8B, 0x57),
                egui::Stroke::new(1.5, Color32::from_rgb(0x1B, 0x4D, 0x31)),
            );
        }
        // 3D render.
        4 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0x4A, 0x5A, 0x6A));
            p.circle_filled(
                c + Vec2::new(2.0, 3.0),
                s * 1.2,
                Color32::from_black_alpha(80),
            );
            p.circle_filled(c, s * 1.2, Color32::from_rgb(0x58, 0xB0, 0x6A));
            p.circle_filled(
                c - Vec2::splat(s * 0.4),
                s * 0.35,
                Color32::from_white_alpha(120),
            );
        }
        // Vector styles: flat, line, comic, icon, sticker, silhouette.
        6 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xF4, 0xE9, 0xD8));
            p.circle_filled(
                c + Vec2::new(-s * 0.6, 0.0),
                s * 0.9,
                Color32::from_rgb(0xE8, 0x50, 0x5B),
            );
            p.rect_filled(
                egui::Rect::from_center_size(c + Vec2::new(s * 0.7, s * 0.2), Vec2::splat(s * 1.4)),
                0.0,
                Color32::from_rgb(0x2F, 0xB5, 0xA8),
            );
        }
        7 => {
            p.rect_filled(r, 0.0, Color32::WHITE);
            p.circle_stroke(c, s * 1.1, egui::Stroke::new(1.4, Color32::BLACK));
            p.line_segment(
                [c - Vec2::splat(s), c + Vec2::splat(s)],
                egui::Stroke::new(1.4, Color32::BLACK),
            );
        }
        8 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0x4F, 0xC3, 0xF7));
            leaf(
                Color32::from_rgb(0xFF, 0xC1, 0x07),
                egui::Stroke::new(2.5, Color32::BLACK),
            );
        }
        9 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xEC, 0xEF, 0xF1));
            p.circle_stroke(
                c,
                s * 1.1,
                egui::Stroke::new(3.0, Color32::from_rgb(0x26, 0x32, 0x38)),
            );
            p.circle_filled(c, s * 0.35, Color32::from_rgb(0x26, 0x32, 0x38));
        }
        // Hand drawn: a wobbly pencil line.
        10 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xFA, 0xF6, 0xEE));
            let pts: Vec<egui::Pos2> = (0..=20)
                .map(|k| {
                    let t = k as f32 / 20.0;
                    egui::pos2(
                        r.left() + 6.0 + t * (r.width() - 12.0),
                        c.y + (t * 9.0).sin() * s * 0.6 + (t * 23.0).sin() * 1.2,
                    )
                })
                .collect();
            p.add(egui::Shape::line(
                pts,
                egui::Stroke::new(1.6, Color32::from_rgb(0x4E, 0x34, 0x2E)),
            ));
        }
        // Kawaii poster: a round pastel face.
        11 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xFF, 0xE4, 0xEC));
            p.circle_filled(c, s * 1.15, Color32::from_rgb(0xFF, 0xF3, 0xC4));
            p.circle_filled(c + Vec2::new(-s * 0.4, -s * 0.1), s * 0.12, Color32::BLACK);
            p.circle_filled(c + Vec2::new(s * 0.4, -s * 0.1), s * 0.12, Color32::BLACK);
            p.circle_filled(
                c + Vec2::new(-s * 0.65, s * 0.25),
                s * 0.18,
                Color32::from_rgb(0xFF, 0x9E, 0xB5),
            );
            p.circle_filled(
                c + Vec2::new(s * 0.65, s * 0.25),
                s * 0.18,
                Color32::from_rgb(0xFF, 0x9E, 0xB5),
            );
        }
        // Pop art.
        _ => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xFF, 0x3D, 0x7F));
            for k in 0..6 {
                p.circle_filled(
                    egui::pos2(r.left() + 6.0 + k as f32 * 10.0, r.top() + 6.0),
                    2.0,
                    Color32::from_rgb(0xFF, 0xE0, 0x3D),
                );
            }
            leaf(
                Color32::from_rgb(0x00, 0xC8, 0xFF),
                egui::Stroke::new(2.0, Color32::BLACK),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(360.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| ai_generate(app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_docker_draws_in_every_mode_and_state() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        // Not set up: the banner, controls greyed; also with a service
        // that makes drawings only, and its information page.
        frame(&ctx, &mut app);
        app.ai.key = "k".into();
        frame(&ctx, &mut app);
        app.ai_image.info = true;
        frame(&ctx, &mut app);
        app.ai_image.info = false;
        // Google Gemini: pictures too.
        app.settings.ai.provider = crate::ai::Provider::OpenAiCompatible;
        app.settings.ai.endpoint = crate::ai::PRESETS[2].2.to_string();
        assert!(app.ai_image_ready());
        app.ai_image.info = true;
        frame(&ctx, &mut app);
        app.ai_image.info = false;
        for mode in [Mode::Create, Mode::Remix, Mode::Vector] {
            app.ai_image.mode = mode;
            for status in [
                ImageStatus::Idle,
                ImageStatus::Done(2),
                ImageStatus::Failed("no".into()),
                ImageStatus::Waiting { done: 1, total: 3 },
            ] {
                app.ai_image.status = status;
                frame(&ctx, &mut app);
            }
        }
        // With results to show.
        let png = crate::bitmap_fx::encode(&image::RgbaImage::from_pixel(
            4,
            3,
            image::Rgba([9, 9, 9, 255]),
        ))
        .expect("png");
        app.ai_image.results.push(crate::ai_image::GenImage {
            png,
            width: 4,
            height: 3,
        });
        frame(&ctx, &mut app);
    }
}
