//! Dockers added with the full workspace: Align and Distribute, Shaping,
//! Step and Repeat, Text, Glyphs, Color, Color Styles, Object Styles,
//! Find and Replace, Scripts, Palettes, Lens, Blend, Contour, Envelope,
//! Extrude, Bevel, Brush Strokes, Bitmap Mask, Object Data, Links,
//! Symbols, Pages, Guidelines, Fonts.

use crate::app::{App, DockerTab};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::Ui;
use tracedraw_core::{document::ShapeKind, live::Effect, Color, Command, Fill, ShapeId};

/// The colour picker every colour of the drawing uses (#442).
fn color_button(ui: &mut Ui, c: &mut Color) -> bool {
    crate::ui::color_picker::color_button(ui, c)
}

fn dim(ui: &mut Ui, text: String) {
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

pub fn show(app: &mut App, ui: &mut Ui, tab: DockerTab) {
    match tab {
        DockerTab::AlignDistribute => crate::ui::align_docker::align_distribute(app, ui),
        DockerTab::Shaping => crate::ui::shaping_docker::shaping(app, ui),
        DockerTab::StepAndRepeat => step_and_repeat(app, ui),
        DockerTab::Text => crate::ui::text_docker::text_docker(app, ui),
        DockerTab::Glyphs => crate::ui::glyphs_docker::glyphs(app, ui),
        DockerTab::Color => crate::ui::color_docker::color(app, ui),
        DockerTab::ColorStyles => color_styles(app, ui),
        DockerTab::ObjectStyles => object_styles(app, ui),
        DockerTab::FindReplace => find_replace(app, ui),
        DockerTab::Scripts => scripts(app, ui),
        DockerTab::Palettes => palettes(app, ui),
        DockerTab::Lens => lens(app, ui),
        DockerTab::Blend => blend(app, ui),
        DockerTab::Contour => contour(app, ui),
        DockerTab::Envelope => crate::ui::envelope_ui::envelope_docker(app, ui),
        DockerTab::Extrude => extrude(app, ui),
        DockerTab::Bevel => bevel(app, ui),
        DockerTab::BrushStrokes => brush_strokes(app, ui),
        DockerTab::BitmapMask => bitmap_mask(app, ui),
        DockerTab::ObjectData => object_data(app, ui),
        DockerTab::Links => links(app, ui),
        DockerTab::Symbols => symbols(app, ui),
        DockerTab::Pages => pages(app, ui),
        DockerTab::Guidelines => guidelines(app, ui),
        DockerTab::Fonts => fonts(app, ui),
        DockerTab::FontSampler => crate::ui::font_sampler::font_sampler(app, ui),
        DockerTab::Corners => corners(app, ui),
        DockerTab::JoinCurves => join_curves(app, ui),
        DockerTab::FitToPath => fit_to_path(app, ui),
        DockerTab::Coordinates => crate::ui::coords_docker::coordinates(app, ui),
        DockerTab::VectorMosaic => crate::ui::vector_mosaic_docker::vector_mosaic(app, ui),
        DockerTab::PictureMosaic => crate::ui::picture_mosaic_docker::picture_mosaic(app, ui),
        DockerTab::Shelf => crate::ui::shelf_docker::shelf(app, ui),
        DockerTab::Export => crate::ui::export_docker::export_docker(app, ui),
        DockerTab::Comments => crate::ui::comments_docker::comments_docker(app, ui),
        DockerTab::AiGenerate => crate::ui::ai_generate_docker::ai_generate(app, ui),
        DockerTab::Effects => crate::ui::effects_docker::effects_docker(app, ui),
        DockerTab::Macros => crate::ui::macros_ui::manager(app, ui),
        DockerTab::PrintMerge => crate::ui::print_merge_docker::print_merge(app, ui),
        _ => {}
    }
}

fn step_and_repeat(app: &mut App, ui: &mut Ui) {
    app.step_repeat_follow_selection();
    let u = app.units;
    let has = !app.selection.is_empty();
    // With nothing selected the values stay at their defaults (#1032).
    ui.add_enabled_ui(has, |ui| {
        step_and_repeat_fields(&mut app.step_repeat, u, ui)
    });
    if ui
        .add_enabled(has, egui::Button::new(tr("docker.apply")))
        .rich_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.sr_apply_desc",
        ))
        .clicked()
    {
        app.step_and_repeat_apply();
    }
}

fn step_and_repeat_fields(sr: &mut crate::ops2::StepRepeat, u: crate::app::Units, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(tr("docker.copies"));
        ui.add(crate::ui::field::NumField::new(&mut sr.copies).range(1..=999))
            .rich_tip(crate::ui::tips::keyed("docker.sr_copies"));
    });
    ui.strong(tr("docker.horizontal_settings"));
    ui.horizontal(|ui| {
        ui.radio_value(&mut sr.mode_x, 0, tr("docker.offset"))
            .rich_tip(crate::ui::tips::pair(
                "docker.offset",
                "docker.sr_offset_desc",
            ));
        ui.radio_value(&mut sr.mode_x, 1, tr("docker.spacing"))
            .rich_tip(crate::ui::tips::pair(
                "docker.spacing",
                "docker.sr_spacing_desc",
            ));
    });
    let mut dx = u.world_from_mm(sr.dx);
    if ui
        .add(
            crate::ui::field::NumField::new(&mut dx)
                .speed(0.5)
                .suffix(format!(" {}", u.short())),
        )
        .rich_tip(crate::ui::tips::keyed("docker.sr_dx"))
        .changed()
    {
        sr.dx = u.world_to_mm(dx);
    }
    ui.strong(tr("docker.vertical_settings"));
    ui.horizontal(|ui| {
        ui.radio_value(&mut sr.mode_y, 0, tr("docker.offset"))
            .rich_tip(crate::ui::tips::pair(
                "docker.offset",
                "docker.sr_offset_desc",
            ));
        ui.radio_value(&mut sr.mode_y, 1, tr("docker.spacing"))
            .rich_tip(crate::ui::tips::pair(
                "docker.spacing",
                "docker.sr_spacing_desc",
            ));
    });
    let mut dy = u.world_from_mm(sr.dy);
    if ui
        .add(
            crate::ui::field::NumField::new(&mut dy)
                .speed(0.5)
                .suffix(format!(" {}", u.short())),
        )
        .rich_tip(crate::ui::tips::keyed("docker.sr_dy"))
        .changed()
    {
        sr.dy = u.world_to_mm(dy);
    }
}

fn color_styles(app: &mut App, ui: &mut Ui) {
    let styles = app.doc().color_styles.clone();
    ui.horizontal(|ui| {
        if ui
            .button(tr("docker.new_from_selection"))
            .rich_tip(crate::ui::tips::pair(
                "docker.new_from_selection",
                "docker.cs_new_from_selection_desc",
            ))
            .clicked()
        {
            let mut st = styles.clone();
            for s in app.selected_shapes() {
                if let Some(c) = s.fill.preview_color() {
                    if !st.iter().any(|x| x.color == c) {
                        st.push(tracedraw_core::ColorStyle {
                            name: format!("Color {}", st.len() + 1),
                            color: c,
                            harmony: None,
                        });
                    }
                }
            }
            app.run(Command::SetColorStyles { styles: st });
        }
        if ui
            .button(tr("docker.new_color_style"))
            .rich_tip(crate::ui::tips::keyed("docker.new_color_style"))
            .clicked()
        {
            let mut st = styles.clone();
            st.push(tracedraw_core::ColorStyle {
                name: format!("Color {}", st.len() + 1),
                color: app.mixer_color,
                harmony: None,
            });
            app.run(Command::SetColorStyles { styles: st });
        }
    });
    let mut st = styles.clone();
    let mut changed = false;
    let mut remove = None;
    for (i, cs) in st.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= color_button(ui, &mut cs.color);
            changed |= ui
                .add(egui::TextEdit::singleline(&mut cs.name).desired_width(90.0))
                .rich_tip(crate::ui::tips::keyed("docker.cs_name"))
                .lost_focus();
            if ui
                .small_button(tr("docker.fill"))
                .rich_tip(crate::ui::tips::pair("docker.fill", "docker.cs_fill_desc"))
                .clicked()
            {
                app.apply_fill(Fill::Solid(cs.color));
            }
            if ui
                .small_button(tr("docker.outline"))
                .rich_tip(crate::ui::tips::pair(
                    "docker.outline",
                    "docker.cs_outline_desc",
                ))
                .clicked()
            {
                app.apply_outline_color(Some(cs.color));
            }
            if ui
                .small_button("x")
                .rich_tip(crate::ui::tips::keyed("docker.cs_delete"))
                .clicked()
            {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        st.remove(i);
        changed = true;
    }
    if changed {
        // Objects using the old colour follow the style.
        let mut cmds = vec![Command::SetColorStyles { styles: st.clone() }];
        for (old, new) in styles.iter().zip(st.iter()) {
            if old.color != new.color {
                let ids: Vec<ShapeId> = app
                    .doc()
                    .all_layers()
                    .flat_map(|l| &l.shapes)
                    .filter(|s| s.fill == Fill::Solid(old.color))
                    .map(|s| s.id)
                    .collect();
                if !ids.is_empty() {
                    cmds.push(Command::SetFill {
                        shapes: ids,
                        fill: Fill::Solid(new.color),
                    });
                }
            }
        }
        let _ = app.engine.run_batch("Color Styles", &cmds);
    }
    ui.separator();
    ui.strong(tr("docker.harmony_editor"));
    ui.horizontal_wrapped(|ui| {
        for h in crate::palette::Harmony::ALL {
            if ui
                .button(tr(h.key()))
                .rich_tip_with(|| format!("{}\n{}", tr(h.key()), tr(&format!("{}_desc", h.key()))))
                .clicked()
            {
                let base = app.mixer_color;
                let mut st = app.doc().color_styles.clone();
                let name = tr(h.key());
                for (k, c) in h.colors(base).into_iter().enumerate() {
                    st.push(tracedraw_core::ColorStyle {
                        name: format!("{name} {}", k + 1),
                        color: c,
                        harmony: Some(name.clone()),
                    });
                }
                app.run(Command::SetColorStyles { styles: st });
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("docker.base_color"));
        let mut c = app.mixer_color;
        if color_button(ui, &mut c) {
            app.mixer_color = c;
        }
    });
}

/// Object Styles: the default properties of new objects, then the styles
/// with their parts and the objects linked to each.
fn object_styles(app: &mut App, ui: &mut Ui) {
    default_properties(app, ui);
    ui.separator();
    styles_list(app, ui);
}

/// A fill swatch and an outline swatch (a red stroke across for none).
fn style_swatches(ui: &mut Ui, fill: &Fill, stroke: &Option<tracedraw_core::Stroke>) {
    let none = |ui: &mut Ui, r: egui::Rect| {
        ui.painter().line_segment(
            [r.left_bottom(), r.right_top()],
            egui::Stroke::new(1.5, egui::Color32::from_rgb(0xD0, 0x30, 0x30)),
        );
    };
    let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 2.0, egui::Color32::WHITE);
    match crate::app::fill_preview_color(fill) {
        Some(c) => {
            ui.painter().rect_filled(r, 2.0, c);
        }
        None => none(ui, r),
    }
    ui.painter().rect_stroke(
        r,
        2.0,
        egui::Stroke::new(1.0, Tokens::control_border()),
        egui::StrokeKind::Inside,
    );
    let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 2.0, egui::Color32::WHITE);
    match stroke {
        Some(s) => {
            let c = crate::canvas::to_color32(s.color);
            let w = (s.width as f32 * 4.0).clamp(1.0, 4.0);
            ui.painter().rect_stroke(
                r.shrink(2.0),
                1.0,
                egui::Stroke::new(w, c),
                egui::StrokeKind::Inside,
            );
        }
        None => none(ui, r),
    }
}

/// Default Properties: what each kind of new object starts with.
fn default_properties(app: &mut App, ui: &mut Ui) {
    use tracedraw_core::{DefaultKind, ObjectDefaults};
    ui.strong(tr("defaults.title"));
    let first = app.selected_shapes().first().cloned();
    for kind in DefaultKind::ALL {
        let d = app.doc().defaults.get(kind).clone();
        ui.horizontal(|ui| {
            style_swatches(ui, &d.fill, &d.stroke);
            ui.label(tr(crate::defaults::kind_key(kind)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(tr("docker.reset"))
                    .rich_tip(tr("defaults.reset_hint"))
                    .clicked()
                {
                    let fresh = ObjectDefaults::default().get(kind).clone();
                    app.set_default_properties(kind, |p| *p = fresh);
                }
                let from = ui
                    .add_enabled(
                        first.is_some(),
                        egui::Button::new(tr("docker.from_selection")).small(),
                    )
                    .rich_tip(tr("defaults.from_selection_hint"));
                if from.clicked() {
                    if let Some(s) = &first {
                        let (fill, stroke) = (s.fill.clone(), s.stroke.clone());
                        app.set_default_properties(kind, |p| {
                            p.fill = fill;
                            p.stroke = stroke;
                        });
                    }
                }
            });
        });
    }
    ui.checkbox(&mut app.settings.defaults.ask, tr("defaults.ask"))
        .rich_tip(crate::ui::tips::pair(
            "defaults.ask_tip",
            "defaults.ask_desc",
        ));
}

/// The styles: a row each (swatches, name, objects linked, Apply, Update,
/// delete) and the parts it sets.
fn styles_list(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.strong(tr("docker.object_styles"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    !app.selection.is_empty(),
                    egui::Button::new(tr("docker.new_style_from_selection")).small(),
                )
                .rich_tip(crate::ui::tips::keyed("docker.new_style_from_selection"))
                .clicked()
            {
                app.new_style_from_selection();
            }
        });
    });
    let styles = app.doc().object_styles.clone();
    if styles.is_empty() {
        dim(ui, tr("docker.object_styles_hint"));
    }
    let counts = crate::styles::linked_counts(app.doc());
    let first = app.selected_shapes().first().cloned();
    let mut st = styles.clone();
    let mut changed = false;
    let mut remove = None;
    let mut apply = None;
    for (i, os) in st.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            style_swatches(ui, &os.fill, &os.stroke);
            changed |= ui
                .add(egui::TextEdit::singleline(&mut os.name).desired_width(96.0))
                .rich_tip(crate::ui::tips::keyed("docker.style_name"))
                .lost_focus();
            let n = counts.get(&os.id).copied().unwrap_or(0);
            dim(ui, trf("docker.style_objects", &[("n", &n.to_string())]));
        });
        ui.horizontal(|ui| {
            ui.add_space(38.0);
            changed |= ui
                .checkbox(&mut os.parts.fill, tr("docker.style_fill"))
                .rich_tip(crate::ui::tips::keyed("docker.style_fill"))
                .changed();
            changed |= ui
                .checkbox(&mut os.parts.outline, tr("docker.style_outline"))
                .rich_tip(crate::ui::tips::keyed("docker.style_outline"))
                .changed();
            changed |= ui
                .checkbox(&mut os.parts.transparency, tr("docker.style_transparency"))
                .rich_tip(crate::ui::tips::keyed("docker.style_transparency"))
                .changed();
        });
        ui.horizontal(|ui| {
            ui.add_space(38.0);
            if ui
                .add_enabled(
                    !app.selection.is_empty(),
                    egui::Button::new(tr("docker.apply")).small(),
                )
                .rich_tip(tr("docker.style_apply_hint"))
                .clicked()
            {
                apply = Some(i);
            }
            if ui
                .add_enabled(
                    first.is_some(),
                    egui::Button::new(tr("docker.update_from_selection")).small(),
                )
                .rich_tip(tr("docker.style_update_hint"))
                .clicked()
            {
                if let Some(s) = &first {
                    os.fill = s.fill.clone();
                    os.stroke = s.stroke.clone();
                    os.opacity = s.opacity;
                    changed = true;
                }
            }
            if ui
                .small_button(tr("docker.delete"))
                .rich_tip(tr("docker.style_delete_hint"))
                .clicked()
            {
                remove = Some(i);
            }
        });
        ui.add_space(2.0);
    }
    if let Some(i) = remove {
        st.remove(i);
        changed = true;
    }
    if changed {
        app.run(Command::SetObjectStyles { styles: st });
    } else if let Some(i) = apply {
        app.apply_style(i);
    }
    let linked = !app.styled_selection().is_empty();
    if ui
        .add_enabled(linked, egui::Button::new(tr("docker.break_style_link")))
        .rich_tip(crate::ui::tips::keyed("docker.break_style_link"))
        .clicked()
    {
        app.break_style_links();
    }
}

fn find_replace(app: &mut App, ui: &mut Ui) {
    let mut st = std::mem::take(&mut app.find_state);
    ui.horizontal(|ui| {
        ui.radio_value(&mut st.kind, 0, tr("dialog.find_text"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.find_text",
                "docker.find_text_desc",
            ));
        ui.radio_value(&mut st.kind, 1, tr("dialog.find_objects"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.find_objects",
                "docker.find_objects_desc",
            ));
    });
    ui.horizontal(|ui| {
        ui.label(tr("dialog.find"));
        ui.text_edit_singleline(&mut st.find)
            .rich_tip(crate::ui::tips::pair(
                "docker.find_field",
                "docker.find_field_desc",
            ));
    });
    if st.kind == 0 {
        ui.horizontal(|ui| {
            ui.label(tr("dialog.replace_with"));
            ui.text_edit_singleline(&mut st.replace)
                .rich_tip(crate::ui::tips::pair(
                    "docker.replace_field",
                    "docker.replace_field_desc",
                ));
        });
        ui.checkbox(&mut st.match_case, tr("dialog.match_case"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.match_case",
                "docker.match_case_desc",
            ));
        ui.checkbox(&mut st.whole_word, tr("dialog.whole_word"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.whole_word",
                "docker.whole_word_desc",
            ));
    }
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr("dialog.find_next"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.find_next",
                "docker.find_next_desc",
            ))
            .clicked()
        {
            st.results =
                crate::export::find_shapes(app, &st.find, st.kind, st.match_case, st.whole_word);
            if !st.results.is_empty() {
                st.cursor = (st.cursor + 1) % st.results.len();
                let id = st.results[st.cursor];
                app.select(vec![id]);
            }
        }
        if ui
            .button(tr("dialog.find_all"))
            .rich_tip(crate::ui::tips::pair(
                "dialog.find_all",
                "docker.find_all_desc",
            ))
            .clicked()
        {
            st.results =
                crate::export::find_shapes(app, &st.find, st.kind, st.match_case, st.whole_word);
            app.select(st.results.clone());
        }
        if st.kind == 0 {
            if ui
                .button(tr("dialog.replace"))
                .rich_tip(crate::ui::tips::pair(
                    "dialog.replace",
                    "docker.replace_desc",
                ))
                .clicked()
            {
                crate::export::replace_text(
                    app,
                    &st.find,
                    &st.replace,
                    st.match_case,
                    st.whole_word,
                    false,
                );
            }
            if ui
                .button(tr("dialog.replace_all"))
                .rich_tip(crate::ui::tips::pair(
                    "dialog.replace_all",
                    "docker.replace_all_desc",
                ))
                .clicked()
            {
                crate::export::replace_text(
                    app,
                    &st.find,
                    &st.replace,
                    st.match_case,
                    st.whole_word,
                    true,
                );
            }
        }
    });
    ui.label(trf(
        "dialog.found_n",
        &[("n", &st.results.len().to_string())],
    ));
    app.find_state = st;
}

fn scripts(app: &mut App, ui: &mut Ui) {
    let debugging = app.script_debug.is_some() || app.macros.run.is_some();
    // The language of the snippet: JavaScript, or VBA (run as a macro).
    ui.horizontal(|ui| {
        let vba = app.macros.scripts_vba;
        let js_r = ui
            .selectable_label(!vba, "JavaScript")
            .rich_tip_with(|| format!("JavaScript\n{}", tr("docker.js_desc")));
        let vba_r = ui
            .add_enabled(!crate::files::WEB, egui::Button::selectable(vba, "VBA"))
            .rich_tip(tr("docker.vba_hint"));
        if (js_r.clicked() && vba) || (vba_r.clicked() && !vba) {
            std::mem::swap(&mut app.script_source, &mut app.macros.other_source);
            app.macros.scripts_vba = !vba;
            if app.macros.scripts_vba && app.script_source.is_empty() {
                app.script_source = "Sub Main()\n    Dim s As Shape\n    Set s = ActiveLayer.CreateRectangle2(1, 1, 2, 1)\n    s.Fill.UniformColor.CMYKAssign 0, 100, 100, 0\n    Debug.Print \"Objects: \" & ActivePage.Shapes.Count\nEnd Sub\n".into();
            }
        }
    });
    let vba = app.macros.scripts_vba;
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(!debugging, egui::Button::new(tr("docker.run")))
            .rich_tip(crate::ui::tips::pair(
                "docker.run",
                "docker.run_script_desc",
            ))
            .clicked()
        {
            let src = app.script_source.clone();
            if vba {
                app.run_vba_snippet(&src);
            } else {
                app.run_script(&src);
            }
        }
        if app.macros.run.is_some()
            && ui
                .button(tr("docker.stop"))
                .rich_tip(crate::ui::tips::pair(
                    "docker.stop",
                    "docker.stop_script_desc",
                ))
                .clicked()
        {
            app.stop_macro();
        }
        if !crate::files::WEB
            && !vba
            && ui
                .add_enabled(!debugging, egui::Button::new(tr("docker.debug")))
                .rich_tip(tr("docker.debug_hint"))
                .clicked()
        {
            app.start_script_debug();
        }
        if ui
            .selectable_label(app.recording.is_some(), tr("docker.record"))
            .rich_tip(crate::ui::tips::pair("docker.record", "docker.record_desc"))
            .clicked()
        {
            app.toggle_recording();
        }
        let (lang, ext) = if vba {
            ("VBA", "bas")
        } else {
            ("JavaScript", "js")
        };
        if ui
            .button(tr("docker.load"))
            .rich_tip(crate::ui::tips::pair(
                "docker.load",
                "docker.load_script_desc",
            ))
            .clicked()
        {
            crate::files::Dialog::new()
                .add_filter(lang, &[ext])
                .pick_file(app, |app, p| {
                    if let Ok(s) = crate::files::read_to_string(p) {
                        app.script_source = s;
                    }
                });
        }
        if ui
            .button(tr("docker.save"))
            .rich_tip(crate::ui::tips::pair(
                "docker.save",
                "docker.save_script_desc",
            ))
            .clicked()
        {
            if let Some(p) = crate::files::Dialog::new()
                .add_filter(lang, &[ext])
                .set_file_name(format!("macro.{ext}"))
                .save_path()
            {
                let _ = crate::files::write(p, &app.script_source);
            }
        }
        if ui
            .button(tr("docker.clear"))
            .rich_tip(crate::ui::tips::pair(
                "docker.clear",
                "docker.clear_output_desc",
            ))
            .clicked()
        {
            app.scripts_output.clear();
        }
        if !vba
            && ui
                .button(tr("docker.install_filter"))
                .rich_tip(tr("docker.install_filter_hint"))
                .clicked()
        {
            let src = app.script_source.clone();
            app.install_script_filter(&src);
        }
    });
    if !app.settings.script_filters.is_empty() {
        ui.strong(tr("docker.script_filters"));
        let mut remove = None;
        let mut edit = None;
        for f in &app.settings.script_filters {
            ui.horizontal(|ui| {
                ui.label(f.label());
                if ui
                    .small_button(tr("docker.edit"))
                    .rich_tip(crate::ui::tips::pair(
                        "docker.edit",
                        "docker.edit_filter_desc",
                    ))
                    .clicked()
                {
                    edit = Some(f.source.clone());
                }
                if ui
                    .small_button(tr("docker.remove"))
                    .rich_tip(crate::ui::tips::pair(
                        "docker.remove",
                        "docker.remove_filter_desc",
                    ))
                    .clicked()
                {
                    remove = Some(f.name.clone());
                }
            });
        }
        if let Some(src) = edit {
            app.script_source = src;
        }
        if let Some(name) = remove {
            app.remove_script_filter(&name);
        }
    }
    if app.script_source.is_empty() && !vba {
        app.script_source = "// Example\nconst r = ActiveLayer.CreateRectangle(20, 20, 60, 40);\nr.Fill.ApplyUniformFill(Color.CMYK(0, 100, 100, 0));\nprint('Objects: ' + ActivePage.Shapes.Count);\n".into();
    }
    if debugging {
        script_debug_bar(app, ui);
    }
    egui::ScrollArea::vertical()
        .id_salt("script_src")
        .max_height(220.0)
        .show(ui, |ui| script_editor(app, ui));
    ui.separator();
    ui.strong(tr("docker.output"));
    egui::ScrollArea::vertical()
        .id_salt("script_out")
        .max_height(140.0)
        .show(ui, |ui| {
            for line in &app.scripts_output {
                ui.label(egui::RichText::new(line).monospace().size(11.0));
            }
        });
    dim(
        ui,
        tr(if vba {
            "docker.scripts_hint_vba"
        } else {
            "docker.scripts_hint"
        }),
    );
}

/// Continue, Step and Stop for a script in the debugger, where it is,
/// and an expression to evaluate where it paused.
fn script_debug_bar(app: &mut App, ui: &mut Ui) {
    use crate::scripting::DebugCtl;
    let paused = app.debug_paused;
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(paused.is_some(), egui::Button::new(tr("docker.continue")))
            .clicked()
        {
            app.debug_control(DebugCtl::Continue);
        }
        if ui
            .add_enabled(paused.is_some(), egui::Button::new(tr("docker.step")))
            .clicked()
        {
            app.debug_control(DebugCtl::Step);
        }
        if ui.button(tr("docker.stop")).clicked() {
            app.debug_control(DebugCtl::Stop);
        }
        match paused {
            Some(line) => ui.label(trf("docker.paused_at", &[("line", &line.to_string())])),
            None => ui.label(tr("docker.running")),
        };
    });
    if paused.is_some() {
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut app.debug_expr)
                    .hint_text(tr("docker.expression"))
                    .desired_width(ui.available_width() - 80.0),
            );
            let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.button(tr("docker.evaluate")).clicked() || enter)
                && !app.debug_expr.trim().is_empty()
            {
                let e = app.debug_expr.trim().to_string();
                app.debug_control(DebugCtl::Eval(e));
            }
        });
    }
}

/// Width of the editor's margin with the line numbers and breakpoints.
const GUTTER: f32 = 34.0;

/// The script editor: line numbers and breakpoints (red dots, set and
/// cleared by a click in the margin) to its left, the paused line marked.
fn script_editor(app: &mut App, ui: &mut Ui) {
    let out = egui::TextEdit::multiline(&mut app.script_source)
        .code_editor()
        .desired_width(f32::INFINITY)
        .desired_rows(10)
        .margin(egui::Margin {
            left: GUTTER as i8,
            right: 4,
            top: 2,
            bottom: 2,
        })
        .show(ui);
    let rect = out.response.response.rect;
    let painter = ui.painter_at(rect);
    let font = egui::FontId::monospace(10.0);
    // The span of each line of the script (wrapped rows belong to the
    // line they continue) and the middle of its first row.
    let mut tops: Vec<(f32, f32)> = Vec::new();
    let mut mids: Vec<f32> = Vec::new();
    let mut new_line = true;
    for row in &out.galley.rows {
        let r = row.rect().translate(out.galley_pos.to_vec2());
        if new_line {
            tops.push((r.top(), r.bottom()));
            mids.push(r.center().y);
        } else if let Some(last) = tops.last_mut() {
            last.1 = r.bottom();
        }
        new_line = row.ends_with_newline;
    }
    let gutter = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.left() + GUTTER - 4.0, rect.bottom()),
    );
    painter.rect_filled(gutter.shrink(1.0), 0.0, Tokens::panel());
    for (i, (top, bottom)) in tops.iter().enumerate() {
        let line = i as u32 + 1;
        let y = mids.get(i).copied().unwrap_or((top + bottom) / 2.0);
        if app.debug_paused == Some(line) {
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(rect.left(), *top),
                    egui::pos2(rect.right(), *bottom),
                ),
                0.0,
                egui::Color32::from_rgba_unmultiplied(255, 220, 0, 60),
            );
        }
        if app.breakpoints.contains(&line) {
            painter.circle_filled(
                egui::pos2(rect.left() + 7.0, y),
                4.0,
                egui::Color32::from_rgb(0xD0, 0x30, 0x30),
            );
        }
        painter.text(
            egui::pos2(rect.left() + GUTTER - 7.0, y),
            egui::Align2::RIGHT_CENTER,
            line.to_string(),
            font.clone(),
            Tokens::text_dim(),
        );
    }
    // The margin takes its own clicks (laid over the editor's).
    let margin = ui.interact(gutter, ui.id().with("script_gutter"), egui::Sense::click());
    if margin.clicked() {
        if let Some(p) = margin.interact_pointer_pos().or(margin.hover_pos()) {
            if let Some(i) = tops.iter().position(|(t, b)| p.y >= *t && p.y < *b) {
                app.toggle_breakpoint(i as u32 + 1);
            }
        }
    }
}

fn palettes(app: &mut App, ui: &mut Ui) {
    let n = app.palettes.len();
    for i in 0..n {
        let name = app.palettes[i].display_name();
        let count = app.palettes[i].colors.len();
        let on = app.visible_palettes.contains(&i);
        ui.horizontal(|ui| {
            let mut v = on;
            if ui
                .checkbox(&mut v, format!("{name} ({count})"))
                .rich_tip_with(|| format!("{name}\n{}", tr("docker.palette_show_desc")))
                .changed()
            {
                app.toggle_palette(i);
            }
            if ui
                .small_button(tr("docker.edit"))
                .rich_tip(crate::ui::tips::pair(
                    "docker.edit",
                    "docker.palette_edit_desc",
                ))
                .clicked()
            {
                app.dialog = crate::ui::dialogs::Dialog::PaletteEditor(
                    crate::ui::dialogs::PaletteEditorState {
                        palette: i,
                        ..Default::default()
                    },
                );
            }
        });
    }
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(tr("menu.window.palette_open"))
            .rich_tip(crate::ui::tips::keyed("menu.window.palette_open"))
            .clicked()
        {
            app.open_palette_file();
        }
        if ui
            .button(tr("menu.window.palette_from_document"))
            .rich_tip(crate::ui::tips::keyed("menu.window.palette_from_document"))
            .clicked()
        {
            app.palette_from_document();
        }
        if ui
            .button(tr("menu.window.palette_from_selection"))
            .rich_tip(crate::ui::tips::keyed("menu.window.palette_from_selection"))
            .clicked()
        {
            app.palette_from_selection();
        }
    });
    let dp = app.doc().palette.clone();
    ui.label(trf(
        "docker.document_palette_n",
        &[("n", &dp.len().to_string())],
    ));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
        for c in dp {
            let [r, g, b] = c.to_rgb8();
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
            let resp = resp.rich_tip_with(|| {
                format!(
                    "{}\n{}",
                    crate::app::color_description(c),
                    tr("docker.palette_chip_desc")
                )
            });
            ui.painter()
                .rect_filled(rect, 1.0, egui::Color32::from_rgb(r, g, b));
            if resp.clicked() {
                app.apply_fill(Fill::Solid(c));
            }
            if resp.secondary_clicked() {
                app.apply_outline_color(Some(c));
            }
        }
    });
}

fn current_effect<F: Fn(&Effect) -> bool>(app: &App, f: F) -> Option<Effect> {
    app.selected_shapes()
        .first()
        .and_then(|s| s.effects.iter().find(|e| f(e)).cloned())
}

fn lens(app: &mut App, ui: &mut Ui) {
    if let Some(Effect::Lens(l)) = current_effect(app, |e| matches!(e, Effect::Lens(_))) {
        if app.lens_synced_to != app.selection.first().copied() {
            app.lens = crate::lens::LensSettings::from_lens(&l);
            app.lens_synced_to = app.selection.first().copied();
        }
    }
    let keys = crate::lens::LENS_KEYS;
    egui::ComboBox::from_id_salt("lens_kind")
        .selected_text(tr(keys[app.lens.kind]))
        .show_ui(ui, |ui| {
            for (i, k) in keys.iter().enumerate() {
                if ui.selectable_label(app.lens.kind == i, tr(k)).clicked() {
                    app.lens.kind = i;
                }
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("docker.lens_kind"));
    let l = &mut app.lens;
    match l.kind {
        1 => {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut l.rate, -100.0..=100.0).text(tr("docker.rate")),
            ))
            .rich_tip(crate::ui::tips::pair(
                "docker.rate",
                "docker.lens_rate_desc",
            ));
        }
        2 | 3 | 10 => {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut l.rate, 0.0..=100.0).text(tr("docker.rate")),
            ))
            .rich_tip(crate::ui::tips::pair(
                "docker.rate",
                "docker.lens_rate_desc",
            ));
            ui.horizontal(|ui| {
                ui.label(tr("docker.color"));
                color_button(ui, &mut l.color);
            });
        }
        4 => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.from"));
                color_button(ui, &mut l.color_to);
                ui.label(tr("docker.to"));
                color_button(ui, &mut l.color);
            });
        }
        5 => {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut l.rate, -100.0..=100.0).text(tr("docker.rate")),
            ))
            .rich_tip(crate::ui::tips::pair(
                "docker.rate",
                "docker.lens_rate_desc",
            ));
        }
        6 => {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut l.rate, 0.0..=100.0).text(tr("docker.palette_rotation")),
            ))
            .rich_tip(crate::ui::tips::keyed("docker.palette_rotation"));
        }
        8 => {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut l.amount, 1.0..=10.0).text(tr("docker.amount")),
            ))
            .rich_tip(crate::ui::tips::pair(
                "docker.amount",
                "docker.lens_amount_desc",
            ));
        }
        9 | 11 => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.color"));
                color_button(ui, &mut l.color);
            });
        }
        12 => {
            // The effects, in the order they run, each with its default
            // settings; Effects > Effect Picker adds them too.
            let mut remove = None;
            for (i, e) in l.effects.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}. {}", i + 1, tr(&format!("fx.{}", e.id))));
                    if ui
                        .small_button("\u{00D7}")
                        .rich_tip(crate::ui::tips::keyed("docker.lens_remove_effect"))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                l.effects.remove(i);
            }
            egui::ComboBox::from_id_salt("lens_fx_add")
                .selected_text(tr("lens.add_effect"))
                .height(360.0)
                .show_ui(ui, |ui| {
                    for (group, list) in crate::fx::groups() {
                        ui.label(
                            egui::RichText::new(tr(group))
                                .color(Tokens::text_dim())
                                .size(11.0),
                        );
                        for fx in list.iter() {
                            if ui
                                .selectable_label(false, tr(&format!("fx.{}", fx.id)))
                                .clicked()
                            {
                                l.effects.push(crate::fx::new_effect(fx.id));
                            }
                        }
                    }
                })
                .response
                .rich_tip(crate::ui::tips::keyed("lens.add_effect"));
        }
        _ => {}
    }
    ui.checkbox(&mut l.frozen, tr("docker.frozen"))
        .rich_tip(crate::ui::tips::keyed("docker.frozen"));
    let has = !app.selection.is_empty();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.apply")))
            .rich_tip(crate::ui::tips::pair(
                "docker.apply",
                "docker.lens_apply_desc",
            ))
            .clicked()
        {
            match app.lens.to_lens() {
                Some(lens) => {
                    if app.lens.frozen {
                        app.push_effect(Effect::Lens(lens), true);
                        app.flatten_effects();
                    } else {
                        app.push_effect(Effect::Lens(lens), true);
                    }
                }
                None => {
                    app.remove_effects_of_kind(&Effect::Lens(tracedraw_core::live::Lens::Invert))
                }
            }
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.remove")))
            .rich_tip(crate::ui::tips::pair(
                "docker.remove",
                "docker.lens_remove_desc",
            ))
            .clicked()
        {
            app.remove_effects_of_kind(&Effect::Lens(tracedraw_core::live::Lens::Invert));
        }
    });
    dim(ui, tr("docker.lens_hint"));
}

/// A small picture button of the Blend docker; true when clicked.
fn blend_pic(
    ui: &mut Ui,
    tip: &str,
    on: bool,
    enabled: bool,
    draw: impl Fn(&egui::Painter, egui::Rect, Color32),
) -> bool {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(26.0, 24.0), egui::Sense::click());
    let resp = resp.rich_tip(tip);
    let p = ui.painter();
    if on {
        p.rect_filled(r, 2.0, crate::theme::Tokens::tool_hover());
        p.rect_stroke(
            r,
            2.0,
            egui::Stroke::new(1.0, Tokens::ACCENT),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() && enabled {
        p.rect_stroke(
            r,
            2.0,
            egui::Stroke::new(1.0, Tokens::border()),
            egui::StrokeKind::Inside,
        );
    }
    let ink = if enabled {
        Tokens::text()
    } else {
        Tokens::text_dim()
    };
    draw(p, r.shrink(5.0), ink);
    enabled && resp.clicked()
}

use egui::Color32;

/// The Blend docker: steps or spacing, rotation with Loop, the path
/// options, the acceleration of objects and colours (with Apply to
/// sizing, Link acceleration and Reset), the way the colours turn, and
/// Apply for two selected objects.
fn blend(app: &mut App, ui: &mut Ui) {
    use crate::ui::docker_form::section;
    use tracedraw_core::live::{BlendOptions, ColorBlend};
    let cur = app.selected_blends().into_iter().next().map(|(_, e)| e);
    let (mut steps, mut accel_o, mut accel_c, mut rot, mut rotate_on_path, has_path, mut opt) =
        match &cur {
            Some(Effect::Blend {
                steps,
                accel_objects,
                accel_colors,
                rotation,
                rotate_on_path,
                path,
                options,
                ..
            }) => (
                *steps,
                *accel_objects,
                *accel_colors,
                *rotation,
                *rotate_on_path,
                path.is_some(),
                *options,
            ),
            _ => (
                app.blend_steps,
                0.0,
                0.0,
                0.0,
                false,
                false,
                app.blend_options,
            ),
        };
    let mut changed = false;
    let mut spacing_mm = opt.spacing.unwrap_or(app.blend_spacing_mm);
    // Steps or spacing: spacing needs a path.
    ui.horizontal(|ui| {
        let by_spacing = opt.spacing.is_some();
        if blend_pic(
            ui,
            &tr("docker.blend_by_steps"),
            !by_spacing,
            true,
            |p, r, c| {
                let s = egui::Stroke::new(1.3, c);
                let pts = [
                    r.left_bottom(),
                    egui::pos2(r.left() + r.width() / 3.0, r.bottom()),
                    egui::pos2(r.left() + r.width() / 3.0, r.center().y),
                    egui::pos2(r.left() + 2.0 * r.width() / 3.0, r.center().y),
                    egui::pos2(r.left() + 2.0 * r.width() / 3.0, r.top()),
                    r.right_top(),
                ];
                p.add(egui::Shape::line(pts.to_vec(), s));
            },
        ) && by_spacing
        {
            opt.spacing = None;
            changed = true;
        }
        if blend_pic(
            ui,
            &tr("docker.blend_by_spacing"),
            by_spacing,
            has_path,
            |p, r, c| {
                let s = egui::Stroke::new(1.3, c);
                let y = r.center().y;
                p.line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], s);
                for x in [r.left(), r.right()] {
                    p.line_segment([egui::pos2(x, y - 4.0), egui::pos2(x, y + 4.0)], s);
                }
            },
        ) && !by_spacing
        {
            opt.spacing = Some(spacing_mm);
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("docker.steps"));
        changed |= ui
            .add_enabled(
                opt.spacing.is_none(),
                crate::ui::field::NumField::new(&mut steps).range(1..=999),
            )
            .rich_tip(crate::ui::tips::pair(
                "docker.steps",
                "toolbar.blend_steps_desc",
            ))
            .changed();
    });
    ui.horizontal(|ui| {
        ui.label(tr("docker.blend_spacing"));
        let units = app.units;
        let mut v = units.from_mm(spacing_mm);
        if ui
            .add_enabled(
                opt.spacing.is_some(),
                crate::ui::field::NumField::new(&mut v)
                    .speed(0.1)
                    .max_decimals(3)
                    .suffix(format!(" {}", units.short())),
            )
            .rich_tip(crate::ui::tips::keyed("docker.blend_spacing"))
            .rich_disabled_tip(crate::ui::tips::keyed("docker.blend_spacing"))
            .changed()
        {
            spacing_mm = units.to_mm(v).max(0.01);
            opt.spacing = Some(spacing_mm);
            changed = true;
        }
    });
    app.blend_spacing_mm = spacing_mm;
    ui.horizontal(|ui| {
        ui.label(tr("docker.rotation"));
        changed |= ui
            .add(
                crate::ui::field::NumField::new(&mut rot)
                    .range(-3600.0..=3600.0)
                    .speed(1.0)
                    .max_decimals(1)
                    .suffix("\u{b0}"),
            )
            .rich_tip(crate::ui::tips::pair(
                "docker.rotation",
                "toolbar.blend_rotation_desc",
            ))
            .changed();
    });
    changed |= ui
        .add_enabled(
            rot != 0.0,
            egui::Checkbox::new(&mut opt.loop_rotation, tr("docker.blend_loop")),
        )
        .rich_tip(crate::ui::tips::keyed("docker.blend_loop"))
        .rich_disabled_tip(crate::ui::tips::keyed("docker.blend_loop"))
        .changed();
    changed |= ui
        .add_enabled(
            has_path,
            egui::Checkbox::new(&mut opt.full_path, tr("docker.blend_full_path")),
        )
        .rich_tip(crate::ui::tips::keyed("docker.blend_full_path"))
        .rich_disabled_tip(crate::ui::tips::keyed("docker.blend_full_path"))
        .changed();
    changed |= ui
        .add_enabled(
            has_path,
            egui::Checkbox::new(&mut rotate_on_path, tr("docker.rotate_all_objects")),
        )
        .rich_tip(crate::ui::tips::keyed("docker.rotate_all_objects"))
        .rich_disabled_tip(crate::ui::tips::keyed("docker.rotate_all_objects"))
        .changed();
    section(ui, Some(&tr("docker.blend_acceleration")));
    ui.spacing_mut().slider_width = (ui.available_width() - 80.0).clamp(80.0, 200.0);
    ui.horizontal(|ui| {
        ui.label(tr("docker.blend_object"));
        if ui
            .add(crate::ui::Rail(
                egui::Slider::new(&mut accel_o, -1.0..=1.0).show_value(false),
            ))
            .rich_tip(crate::ui::tips::pair(
                "docker.blend_object",
                "docker.object_acceleration_desc",
            ))
            .changed()
        {
            changed = true;
            if app.blend_link_accel {
                accel_c = accel_o;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("docker.blend_color"));
        if ui
            .add_enabled(
                !app.blend_link_accel,
                crate::ui::Rail(egui::Slider::new(&mut accel_c, -1.0..=1.0).show_value(false)),
            )
            .rich_tip(crate::ui::tips::pair(
                "docker.blend_color",
                "docker.color_acceleration_desc",
            ))
            .changed()
        {
            changed = true;
        }
    });
    changed |= ui
        .checkbox(&mut opt.accel_sizing, tr("docker.blend_apply_sizing"))
        .rich_tip(crate::ui::tips::keyed("docker.blend_apply_sizing"))
        .changed();
    let mut link = app.blend_link_accel;
    if ui
        .checkbox(&mut link, tr("docker.blend_link_accel"))
        .rich_tip(crate::ui::tips::keyed("docker.blend_link_accel"))
        .changed()
    {
        app.blend_link_accel = link;
        if link && accel_c != accel_o {
            accel_c = accel_o;
            changed = true;
        }
    }
    // One row high: laid out right to left over the whole rest of the
    // panel, it pushed Color Blend out of sight (#815).
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 24.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            if ui
                .add_enabled(
                    accel_o != 0.0 || accel_c != 0.0,
                    egui::Button::new(tr("docker.reset")).min_size(egui::vec2(90.0, 22.0)),
                )
                .rich_tip(crate::ui::tips::pair(
                    "docker.reset",
                    "docker.blend_reset_desc",
                ))
                .clicked()
            {
                accel_o = 0.0;
                accel_c = 0.0;
                changed = true;
            }
        },
    );
    section(ui, Some(&tr("docker.blend_color_blend")));
    ui.horizontal(|ui| {
        for (how, key) in [
            (ColorBlend::Direct, "docker.blend_direct"),
            (ColorBlend::Clockwise, "docker.blend_clockwise"),
            (
                ColorBlend::CounterClockwise,
                "docker.blend_counterclockwise",
            ),
        ] {
            let on = opt.color_blend == how;
            if blend_pic(ui, &tr(key), on, true, |p, r, c| {
                let s = egui::Stroke::new(1.3, c);
                let ctr = r.center();
                let rad = r.width().min(r.height()) / 2.0;
                match how {
                    ColorBlend::Direct => {
                        p.line_segment([r.left_bottom(), r.right_top()], s);
                    }
                    _ => {
                        let pts: Vec<egui::Pos2> = (0..=12)
                            .map(|k| {
                                let a = std::f32::consts::PI * (0.15 + 1.4 * k as f32 / 12.0);
                                let a = if how == ColorBlend::Clockwise { -a } else { a };
                                ctr + egui::vec2(a.cos(), a.sin()) * rad
                            })
                            .collect();
                        let end = *pts.last().unwrap_or(&ctr);
                        p.add(egui::Shape::line(pts, s));
                        p.circle_filled(end, 2.2, c);
                    }
                }
            }) && !on
            {
                opt.color_blend = how;
                changed = true;
            }
        }
    });
    // Apply: two objects selected and no blend yet, a blend between them
    // with these settings (#815).
    let two = cur.is_none() && app.selection.len() == 2;
    let apply = ui
        .allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 28.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.add_enabled(
                    two,
                    egui::Button::new(tr("docker.apply")).min_size(egui::vec2(90.0, 22.0)),
                )
                .rich_tip(crate::ui::tips::pair(
                    "docker.apply",
                    "docker.blend_apply_desc",
                ))
                .rich_disabled_tip(crate::ui::tips::pair(
                    "docker.apply",
                    "docker.blend_apply_desc",
                ))
                .clicked()
            },
        )
        .inner;
    app.blend_steps = steps;
    app.blend_options = BlendOptions { ..opt };
    if apply {
        app.blend_selection(steps, accel_o, accel_c, rot);
    }
    if let Some(Effect::Blend { .. }) = cur {
        if changed {
            app.change_blends("Blend", |e| {
                let mut e = e.clone();
                if let Effect::Blend {
                    steps: s,
                    accel_objects,
                    accel_colors,
                    rotation,
                    rotate_on_path: r,
                    options,
                    ..
                } = &mut e
                {
                    *s = steps;
                    *accel_objects = accel_o;
                    *accel_colors = accel_c;
                    *rotation = rot;
                    *r = rotate_on_path;
                    *options = opt;
                }
                Some(e)
            });
        }
        section(ui, None);
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr("docker.new_path"))
                .rich_tip(crate::ui::tips::keyed("docker.new_path"))
                .clicked()
            {
                app.pending_blend_path = true;
                app.status = tr("status.click_blend_path");
            }
            if ui
                .button(tr("docker.clear_blend"))
                .rich_tip(crate::ui::tips::keyed("docker.clear_blend"))
                .clicked()
            {
                app.clear_blend();
            }
            if ui
                .button(tr("docker.break_apart"))
                .rich_tip(crate::ui::tips::pair(
                    "docker.break_apart",
                    "toolbar.effect_break_apart_desc",
                ))
                .clicked()
            {
                app.break_effect_groups();
            }
        });
    } else {
        let two = app.selection.len() == 2;
        let tip = crate::ui::tips::pair("docker.apply", "docker.blend_apply_desc");
        if crate::ui::docker_form::wide_button(ui, &tr("docker.apply"), two, &tip) {
            app.blend_selection(steps, accel_o, accel_c, rot);
        }
        if !two {
            dim(ui, tr("docker.blend_hint"));
        }
    }
}

fn contour(app: &mut App, ui: &mut Ui) {
    crate::ui::contour_bar::contour_controls(app, ui);
    crate::ui::contour_bar::contour_actions(app, ui);
}

fn extrude(app: &mut App, ui: &mut Ui) {
    crate::ui::extrude_bar::extrude_docker(app, ui);
}

fn bevel(app: &mut App, ui: &mut Ui) {
    let cur = current_effect(app, |e| matches!(e, Effect::Bevel { .. }));
    let b = &mut app.bevel;
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .selectable_label(b.style == 0, tr("docker.soft_edge"))
            .rich_tip(crate::ui::tips::keyed("docker.soft_edge"))
            .clicked()
            .then(|| b.style = 0)
            .is_some();
        changed |= ui
            .selectable_label(b.style == 1, tr("docker.emboss"))
            .rich_tip(crate::ui::tips::keyed("docker.emboss"))
            .clicked()
            .then(|| b.style = 1)
            .is_some();
    });
    changed |= ui
        .add(crate::ui::Rail(
            egui::Slider::new(&mut b.distance, 0.1..=50.0)
                .text(tr("docker.distance"))
                .logarithmic(true),
        ))
        .rich_tip(crate::ui::tips::pair(
            "docker.distance",
            "docker.bevel_distance_desc",
        ))
        .changed();
    changed |= ui
        .add(crate::ui::Rail(
            egui::Slider::new(&mut b.light_angle, 0.0..=360.0)
                .text(tr("docker.light_angle"))
                .suffix("°"),
        ))
        .rich_tip(crate::ui::tips::pair(
            "docker.light_angle",
            "extrude.light_angle_desc",
        ))
        .changed();
    changed |= ui
        .add(crate::ui::Rail(
            egui::Slider::new(&mut b.intensity, 0.0..=100.0).text(tr("docker.intensity")),
        ))
        .rich_tip(crate::ui::tips::pair(
            "docker.intensity",
            "extrude.light_intensity_desc",
        ))
        .changed();
    ui.horizontal(|ui| {
        ui.label(tr("docker.shadow_color"));
        changed |= color_button(ui, &mut b.shadow_color);
        ui.label(tr("docker.light_color"));
        changed |= color_button(ui, &mut b.light_color);
    });
    let bv = app.bevel.clone();
    let effect = Effect::Bevel {
        distance: bv.distance,
        light_angle: bv.light_angle,
        intensity: bv.intensity,
        style: bv.style,
        shadow_color: bv.shadow_color,
        light_color: bv.light_color,
    };
    let has = !app.selection.is_empty();
    if cur.is_some() && changed {
        app.push_effect(effect.clone(), true);
    }
    ui.horizontal(|ui| {
        if cur.is_none()
            && ui
                .add_enabled(has, egui::Button::new(tr("docker.apply")))
                .rich_tip(crate::ui::tips::pair(
                    "docker.apply",
                    "docker.bevel_apply_desc",
                ))
                .clicked()
        {
            app.push_effect(effect.clone(), true);
        }
        if cur.is_some()
            && ui
                .button(tr("docker.remove"))
                .rich_tip(crate::ui::tips::pair(
                    "docker.remove",
                    "docker.bevel_remove_desc",
                ))
                .clicked()
        {
            app.remove_effects_of_kind(&effect);
        }
    });
}

fn brush_strokes(app: &mut App, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        for m in crate::media::MediaMode::ALL {
            if ui
                .selectable_label(app.media_mode == m, tr(m.key()))
                .rich_tip_with(|| format!("{}\n{}", tr(m.key()), tr(&format!("{}_desc", m.key()))))
                .clicked()
            {
                app.media_mode = m;
            }
        }
    });
    let u = app.units;
    let mut w = u.from_mm(app.media_width);
    if ui
        .add(crate::ui::Rail(
            egui::Slider::new(&mut w, 0.1..=u.from_mm(50.0))
                .text(tr("docker.stroke_width"))
                .logarithmic(true),
        ))
        .rich_tip(crate::ui::tips::pair(
            "docker.stroke_width",
            "docker.media_width_desc",
        ))
        .changed()
    {
        app.media_width = u.to_mm(w);
    }
    match app.media_mode {
        crate::media::MediaMode::Calligraphic => {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut app.media_angle, 0.0..=180.0)
                    .text(tr("docker.nib_angle"))
                    .suffix("°"),
            ))
            .rich_tip(crate::ui::tips::keyed("docker.nib_angle"));
        }
        crate::media::MediaMode::Preset | crate::media::MediaMode::Brush => {
            ui.horizontal_wrapped(|ui| {
                for (i, k) in crate::media::BRUSH_KEYS.iter().enumerate() {
                    if ui
                        .selectable_label(app.media_preset == i, tr(k))
                        .rich_tip_with(|| format!("{}\n{}", tr(k), tr("docker.media_preset_desc")))
                        .clicked()
                    {
                        app.media_preset = i;
                    }
                }
            });
        }
        crate::media::MediaMode::Sprayer => {
            ui.horizontal_wrapped(|ui| {
                for (i, k) in crate::media::SPRAY_KEYS.iter().enumerate() {
                    if ui
                        .selectable_label(app.media_preset == i, tr(k))
                        .rich_tip_with(|| format!("{}\n{}", tr(k), tr("docker.media_preset_desc")))
                        .clicked()
                    {
                        app.media_preset = i;
                    }
                }
            });
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut app.media_spacing, 1.0..=50.0).text(tr("docker.spacing")),
            ))
            .rich_tip(crate::ui::tips::pair(
                "docker.spacing",
                "docker.spray_spacing_desc",
            ));
        }
        crate::media::MediaMode::Expression => {
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut app.media_pressure, 0.0..=1.0).text(tr("docker.pressure")),
            ))
            .rich_tip(crate::ui::tips::pair(
                "docker.pressure",
                "docker.pressure_desc",
            ));
        }
    }
    ui.horizontal(|ui| {
        ui.label(tr("docker.smoothing"));
        ui.add(crate::ui::Rail(egui::Slider::new(
            &mut app.media_smoothing,
            0.0..=100.0,
        )))
        .rich_tip(crate::ui::tips::pair(
            "docker.smoothing",
            "docker.smoothing_desc",
        ));
    });
    let has_curve = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Path { .. }));
    if ui
        .add_enabled(has_curve, egui::Button::new(tr("docker.apply_to_curve")))
        .rich_tip(crate::ui::tips::keyed("docker.apply_to_curve"))
        .rich_disabled_tip(crate::ui::tips::keyed("docker.apply_to_curve"))
        .clicked()
    {
        app.apply_media_to_selection();
    }
    dim(ui, tr("docker.brush_strokes_hint"));
}

/// A square icon button drawn by `paint`, with its tip.
fn icon_btn(
    ui: &mut Ui,
    enabled: bool,
    tip: &str,
    paint: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (r, resp) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), sense);
    if enabled && resp.hovered() {
        ui.painter()
            .rect_filled(r, 2.0, crate::theme::Tokens::tool_hover());
    }
    let color = if enabled {
        crate::theme::Tokens::icon()
    } else {
        crate::theme::Tokens::icon_off()
    };
    paint(ui.painter(), r.shrink(5.0), color);
    resp.rich_tip(tip)
}

/// Bitmaps > Bitmap Mask: ten colour slots (in use, colour, tolerance),
/// hide or show the colours in use, Apply on the selected bitmaps.
fn bitmap_mask(app: &mut App, ui: &mut Ui) {
    use crate::bitmap_fx::MASK_SLOTS;
    let has_bitmap = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Bitmap { .. }));
    // A colour sampled with the eyedropper goes into the picked slot.
    if app.mask.picking {
        if let Some(c) = app.eyedropper_color {
            if app.mask.seen != Some(c) {
                let i = app.mask.picked.min(MASK_SLOTS - 1);
                app.mask.slots[i].color = c;
                app.mask.slots[i].on = true;
                app.mask.seen = Some(c);
            }
        }
    }
    ui.horizontal(|ui| {
        if icon_btn(ui, true, &tr("docker.mask_eyedropper"), |p, r, c| {
            crate::ui::icons::draw(p, r, crate::tools::Tool::ColorEyedropper, c)
        })
        .clicked()
        {
            app.mask.picking = true;
            app.mask.seen = app.eyedropper_color;
            app.set_tool(crate::tools::Tool::ColorEyedropper);
        }
        if icon_btn(ui, has_bitmap, &tr("docker.mask_from_bitmap"), |p, r, c| {
            // A bitmap with a grid of samples.
            p.rect_stroke(r, 0.0, egui::Stroke::new(1.2, c), egui::StrokeKind::Inside);
            for i in 1..3 {
                let t = i as f32 / 3.0;
                p.line_segment(
                    [
                        r.lerp_inside(egui::vec2(t, 0.0)),
                        r.lerp_inside(egui::vec2(t, 1.0)),
                    ],
                    egui::Stroke::new(0.8, c),
                );
                p.line_segment(
                    [
                        r.lerp_inside(egui::vec2(0.0, t)),
                        r.lerp_inside(egui::vec2(1.0, t)),
                    ],
                    egui::Stroke::new(0.8, c),
                );
            }
        })
        .clicked()
        {
            app.mask_colors_from_bitmap();
        }
    });
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(0xC8)))
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("mask_slots")
                .max_height(200.0)
                .show(ui, |ui| {
                    for i in 0..MASK_SLOTS {
                        let picked = app.mask.picked == i;
                        let row = ui.horizontal(|ui| {
                            let slot = &mut app.mask.slots[i];
                            ui.checkbox(&mut slot.on, "")
                                .rich_tip(crate::ui::tips::keyed("docker.mask_slot_on"));
                            let (r, resp) = ui.allocate_exact_size(
                                egui::vec2((ui.available_width() - 40.0).max(60.0), 16.0),
                                egui::Sense::click(),
                            );
                            let resp = resp.rich_tip(crate::ui::tips::keyed("docker.mask_slot"));
                            let [cr, cg, cb] = slot.color.to_rgb8();
                            ui.painter().rect(
                                r,
                                0.0,
                                egui::Color32::from_rgb(cr, cg, cb),
                                egui::Stroke::new(
                                    if picked { 2.0 } else { 1.0 },
                                    if picked {
                                        crate::theme::Tokens::ACCENT
                                    } else {
                                        egui::Color32::from_gray(0xA0)
                                    },
                                ),
                                egui::StrokeKind::Inside,
                            );
                            ui.label(slot.tolerance.to_string());
                            resp.clicked()
                        });
                        if row.inner {
                            app.mask.picked = i;
                        }
                    }
                });
        });
    ui.horizontal(|ui| {
        if icon_btn(ui, true, &tr("docker.mask_save"), |p, r, c| {
            crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Save, c)
        })
        .clicked()
        {
            let text = app.mask.to_text();
            if let Some(path) = crate::files::Dialog::new()
                .add_filter(tr("docker.mask_file"), &["msk"])
                .set_file_name("mask.msk".to_string())
                .save_path()
            {
                if let Err(e) = crate::files::write(&path, text) {
                    app.status = e.to_string();
                }
            }
        }
        if icon_btn(ui, true, &tr("docker.mask_open"), |p, r, c| {
            crate::ui::icons::draw_action(p, r, crate::ui::icons::Action::Open, c)
        })
        .clicked()
        {
            crate::files::Dialog::new()
                .add_filter(tr("docker.mask_file"), &["msk"])
                .pick_file(app, |app, path| match crate::files::read(&path) {
                    Ok(bytes) => {
                        if !app.mask.read_text(&String::from_utf8_lossy(&bytes)) {
                            app.status = tr("docker.mask_bad_file");
                        }
                    }
                    Err(e) => app.status = e.to_string(),
                });
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let i = app.mask.picked.min(MASK_SLOTS - 1);
            let used = app.mask.slots[i] != crate::bitmap_fx::MaskSlot::default();
            if icon_btn(ui, used, &tr("docker.mask_delete"), |p, r, c| {
                // A bin.
                let s = egui::Stroke::new(1.2, c);
                p.line_segment(
                    [
                        r.left_top() + egui::vec2(0.0, 3.0),
                        r.right_top() + egui::vec2(0.0, 3.0),
                    ],
                    s,
                );
                p.rect_stroke(
                    egui::Rect::from_min_max(
                        r.left_top() + egui::vec2(2.5, 4.0),
                        r.right_bottom() - egui::vec2(2.5, 0.0),
                    ),
                    1.0,
                    s,
                    egui::StrokeKind::Inside,
                );
            })
            .clicked()
            {
                app.mask.slots[i] = Default::default();
            }
        });
    });
    ui.add_space(4.0);
    ui.label(tr("docker.tolerance"));
    ui.horizontal(|ui| {
        let i = app.mask.picked.min(MASK_SLOTS - 1);
        let tol = &mut app.mask.slots[i].tolerance;
        let tip = crate::ui::tips::pair("docker.tolerance", "docker.mask_tolerance_desc");
        ui.add(crate::ui::Rail(
            egui::Slider::new(tol, 0..=100).show_value(false),
        ))
        .rich_tip(&tip);
        ui.add(
            crate::ui::field::NumField::new(tol)
                .range(0..=100)
                .suffix(" %"),
        )
        .rich_tip(&tip);
    });
    ui.horizontal(|ui| {
        ui.label(tr("docker.mask_colors"));
        ui.vertical(|ui| {
            ui.radio_value(&mut app.mask.hide, true, tr("docker.mask_hide"))
                .rich_tip(crate::ui::tips::keyed("docker.mask_hide"));
            ui.radio_value(&mut app.mask.hide, false, tr("docker.mask_show"))
                .rich_tip(crate::ui::tips::keyed("docker.mask_show"));
        });
    });
    ui.add_space(4.0);
    let can = has_bitmap && !app.mask.active().is_empty();
    if ui
        .add_enabled(
            can,
            egui::Button::new(tr("docker.apply")).min_size(egui::vec2(ui.available_width(), 26.0)),
        )
        .rich_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.mask_apply_desc",
        ))
        .rich_disabled_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.mask_apply_desc",
        ))
        .clicked()
    {
        app.apply_bitmap_mask();
    }
    if !has_bitmap {
        dim(ui, tr("docker.bitmap_mask_hint"));
    }
}

fn object_data(app: &mut App, ui: &mut Ui) {
    let Some(s) = app.selected_shapes().into_iter().next() else {
        dim(ui, tr("docker.select_one_object"));
        return;
    };
    let mut data = s.data.clone();
    let mut changed = false;
    let mut remove = None;
    egui::Grid::new("objdata").num_columns(3).show(ui, |ui| {
        ui.strong(tr("docker.field"));
        ui.strong(tr("docker.value"));
        ui.label("");
        ui.end_row();
        for (i, (k, v)) in data.iter_mut().enumerate() {
            changed |= ui
                .add(egui::TextEdit::singleline(k).desired_width(90.0))
                .rich_tip(crate::ui::tips::pair(
                    "docker.field",
                    "docker.data_field_desc",
                ))
                .lost_focus();
            changed |= ui
                .add(egui::TextEdit::singleline(v).desired_width(120.0))
                .rich_tip(crate::ui::tips::pair(
                    "docker.value",
                    "docker.data_value_desc",
                ))
                .lost_focus();
            if ui
                .small_button("x")
                .rich_tip(crate::ui::tips::keyed("docker.data_remove"))
                .clicked()
            {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        data.remove(i);
        changed = true;
    }
    ui.horizontal(|ui| {
        if ui
            .button(tr("docker.add_field"))
            .rich_tip(crate::ui::tips::keyed("docker.add_field"))
            .clicked()
        {
            data.push((format!("Field {}", data.len() + 1), String::new()));
            changed = true;
        }
        for k in ["Name", "Cost", "Comments", "CDRStaticID"] {
            if !data.iter().any(|(f, _)| f == k)
                && ui
                    .small_button(k)
                    .rich_tip_with(|| format!("{k}\n{}", tr("docker.data_add_named_desc")))
                    .clicked()
            {
                data.push((k.into(), String::new()));
                changed = true;
            }
        }
    });
    if changed {
        app.run(Command::SetObjectData { shape: s.id, data });
    }
    // Summary of a cost field across the selection.
    let total: f64 = app
        .selected_shapes()
        .iter()
        .filter_map(|s| {
            s.data
                .iter()
                .find(|(k, _)| k == "Cost")
                .and_then(|(_, v)| v.parse::<f64>().ok())
        })
        .sum();
    if total > 0.0 {
        ui.label(trf("docker.total_cost", &[("v", &format!("{total:.2}"))]));
    }
}

fn links(app: &mut App, ui: &mut Ui) {
    let shapes = app.selected_shapes();
    if shapes.is_empty() {
        dim(ui, tr("docker.select_one_object"));
        return;
    }
    let mut link = shapes[0].link.clone().unwrap_or_default();
    ui.label(tr("docker.hyperlink"));
    if ui
        .text_edit_singleline(&mut link)
        .rich_tip(crate::ui::tips::keyed("docker.hyperlink_field"))
        .lost_focus()
    {
        let ids = app.selection.clone();
        app.run(Command::SetLink {
            shapes: ids,
            link: (!link.is_empty()).then_some(link.clone()),
        });
    }
    ui.horizontal(|ui| {
        for (k, prefix) in [
            ("docker.link_web", "https://"),
            ("docker.link_mail", "mailto:"),
            ("docker.link_page", "#page="),
        ] {
            if ui
                .small_button(tr(k))
                .rich_tip_with(|| format!("{}\n{}", tr(k), tr(&format!("{k}_desc"))))
                .clicked()
                && !link.starts_with(prefix)
            {
                let ids = app.selection.clone();
                app.run(Command::SetLink {
                    shapes: ids,
                    link: Some(format!("{prefix}{link}")),
                });
            }
        }
        if ui
            .small_button(tr("docker.remove"))
            .rich_tip(crate::ui::tips::pair(
                "docker.remove",
                "docker.link_remove_desc",
            ))
            .clicked()
        {
            let ids = app.selection.clone();
            app.run(Command::SetLink {
                shapes: ids,
                link: None,
            });
        }
    });
    ui.separator();
    ui.strong(tr("docker.bookmarks"));
    let named: Vec<(ShapeId, String)> = app
        .doc()
        .all_layers()
        .flat_map(|l| &l.shapes)
        .filter_map(|s| s.name.clone().map(|n| (s.id, n)))
        .collect();
    for (id, n) in named {
        if ui
            .small_button(&n)
            .rich_tip_with(|| format!("{n}\n{}", tr("docker.bookmark_desc")))
            .clicked()
        {
            app.select(vec![id]);
            app.zoom_to_selection();
        }
    }
}

fn symbols(app: &mut App, ui: &mut Ui) {
    let has = !app.selection.is_empty();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.new_symbol")))
            .rich_tip(crate::ui::tips::keyed("docker.new_symbol"))
            .rich_disabled_tip(crate::ui::tips::keyed("docker.new_symbol"))
            .clicked()
        {
            app.create_symbol_from_selection();
        }
        if ui
            .button(tr("docker.revert_to_objects"))
            .rich_tip(crate::ui::tips::keyed("docker.revert_to_objects"))
            .clicked()
        {
            app.revert_symbol_instances();
        }
    });
    let syms: Vec<(usize, String)> = app
        .doc()
        .symbols
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.name.clone()))
        .collect();
    for (i, name) in syms {
        ui.horizontal(|ui| {
            ui.label(&name);
            if ui
                .small_button(tr("docker.insert"))
                .rich_tip(crate::ui::tips::pair(
                    "docker.insert",
                    "docker.insert_symbol_desc",
                ))
                .clicked()
            {
                app.insert_symbol_instance(i);
            }
            let uses = app
                .doc()
                .all_layers()
                .flat_map(|l| &l.shapes)
                .filter(|s| matches!(s.kind, ShapeKind::SymbolInstance { index } if index == i))
                .count();
            dim(ui, trf("docker.instances_n", &[("n", &uses.to_string())]));
        });
    }
    if app.doc().symbols.is_empty() {
        dim(ui, tr("docker.symbols_hint"));
    }
}

fn pages(app: &mut App, ui: &mut Ui) {
    let pages: Vec<(tracedraw_core::PageId, String, tracedraw_core::Size)> = app
        .doc()
        .pages
        .iter()
        .map(|p| (p.id, p.name.clone(), p.size))
        .collect();
    let n = pages.len();
    ui.horizontal(|ui| {
        if ui
            .button(tr("menu.layout.insert_page"))
            .rich_tip(crate::ui::tips::keyed("menu.layout.insert_page"))
            .clicked()
        {
            app.add_page();
        }
        if ui
            .add_enabled(n > 1, egui::Button::new(tr("menu.layout.delete_page")))
            .rich_tip(crate::ui::tips::keyed("menu.layout.delete_page"))
            .rich_disabled_tip(crate::ui::tips::keyed("menu.layout.delete_page"))
            .clicked()
        {
            app.ask_delete_page();
        }
        if ui
            .button(tr("menu.layout.duplicate_page"))
            .rich_tip(crate::ui::tips::keyed("menu.layout.duplicate_page"))
            .clicked()
        {
            app.dialog = crate::ui::dialogs::Dialog::duplicate_page();
        }
        if ui
            .selectable_label(app.page_sorter, tr("menu.view.page_sorter"))
            .rich_tip(crate::ui::tips::keyed("menu.view.page_sorter"))
            .clicked()
        {
            app.page_sorter = !app.page_sorter;
        }
    });
    for (i, (id, name, size)) in pages.iter().enumerate() {
        ui.horizontal(|ui| {
            let active = *id == app.page;
            if ui
                .selectable_label(active, format!("{}. {name}", i + 1))
                .rich_tip_with(|| format!("{name}\n{}", tr("docker.page_list_desc")))
                .clicked()
            {
                app.goto_page(i);
            }
            dim(
                ui,
                format!(
                    "{} x {}",
                    app.units.from_mm(size.width).round(),
                    app.units.from_mm(size.height).round()
                ),
            );
            if i > 0
                && ui
                    .small_button("\u{25B2}")
                    .rich_tip(crate::ui::tips::keyed("docker.page_move_up"))
                    .clicked()
            {
                app.run(Command::MovePage {
                    page: *id,
                    to: i - 1,
                });
            }
            if i + 1 < n
                && ui
                    .small_button("\u{25BC}")
                    .rich_tip(crate::ui::tips::keyed("docker.page_move_down"))
                    .clicked()
            {
                app.run(Command::MovePage {
                    page: *id,
                    to: i + 1,
                });
            }
        });
    }
}

fn guidelines(app: &mut App, ui: &mut Ui) {
    crate::ui::layout_options::guidelines_docker(app, ui);
}

fn fonts(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(tr("dialog.filter"));
        ui.text_edit_singleline(&mut app.font_filter)
            .rich_tip(crate::ui::tips::keyed("docker.font_filter"));
    });
    let filter = app.font_filter.to_lowercase();
    let used = app.doc().font_families_used();
    if !used.is_empty() {
        ui.strong(tr("docker.fonts_in_document"));
        for f in &used {
            let missing = !app.font_installed(f);
            let carried = !missing && tracedraw_text::fonts().is_document_family(f);
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.text_font == *f, f)
                    .rich_tip(tr("docker.font_apply_hint"))
                    .clicked()
                {
                    app.apply_text_font(f);
                }
                if missing {
                    let label = ui
                        .colored_label(egui::Color32::from_rgb(200, 100, 0), tr("docker.missing"));
                    if let Some(s) = tracedraw_text::fonts().substitute(f) {
                        label.rich_tip(trf("docker.missing_hint", &[("f", &s)]));
                    }
                } else if carried {
                    ui.weak(tr("docker.embedded"))
                        .rich_tip(tr("docker.embedded_hint"));
                }
            });
        }
        ui.separator();
    }
    let families: Vec<String> = app
        .font_families
        .iter()
        .filter(|f| filter.is_empty() || f.to_lowercase().contains(&filter))
        .cloned()
        .collect();
    ui.label(trf("dialog.fonts_n", &[("n", &families.len().to_string())]));
    egui::ScrollArea::vertical().show(ui, |ui| {
        for f in families {
            if ui
                .selectable_label(app.text_font == f, &f)
                .rich_tip(tr("docker.font_apply_hint"))
                .clicked()
            {
                app.apply_text_font(&f);
            }
        }
    });
}

/// Corners: fillet, scallop or chamfer the corners of the selected
/// objects (or of the nodes chosen with the Shape tool), with a preview of
/// the result on the drawing.
fn corners(app: &mut App, ui: &mut Ui) {
    use crate::corners::CornerOp;
    let u = app.units;
    let field = |ui: &mut Ui, label: &str, tip: String, mm: &mut f64| {
        ui.horizontal(|ui| {
            ui.label(label);
            let mut v = u.world_from_mm(*mm);
            if ui
                .add_sized(
                    [90.0, 22.0],
                    crate::ui::field::NumField::new(&mut v)
                        .range(0.0..=100_000.0)
                        .max_decimals(3)
                        .suffix(format!(" {}", u.short()))
                        .unit_mm(u.mm()),
                )
                .rich_tip(tip)
                .changed()
            {
                *mm = u.world_to_mm(v);
            }
        });
    };
    ui.strong(tr("docker.corners_operation"));
    for (op, key) in [
        (CornerOp::Fillet, "docker.corners_fillet"),
        (CornerOp::Scallop, "docker.corners_scallop"),
        (CornerOp::Chamfer, "docker.corners_chamfer"),
    ] {
        ui.radio_value(&mut app.corners.op, op, tr(key))
            .rich_tip(crate::ui::tips::keyed(key));
    }
    ui.add_space(6.0);
    let mut c = app.corners;
    if c.op == CornerOp::Chamfer {
        ui.strong(tr("docker.corners_chamfer_distance"));
        field(
            ui,
            "A:",
            crate::ui::tips::keyed("docker.corners_a"),
            &mut c.a,
        );
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!c.lock, |ui| {
                ui.label("B:");
                let mut v = u.world_from_mm(if c.lock { c.a } else { c.b });
                if ui
                    .add_sized(
                        [90.0, 22.0],
                        crate::ui::field::NumField::new(&mut v)
                            .range(0.0..=100_000.0)
                            .max_decimals(3)
                            .suffix(format!(" {}", u.short()))
                            .unit_mm(u.mm()),
                    )
                    .rich_tip(crate::ui::tips::keyed("docker.corners_b"))
                    .rich_disabled_tip(crate::ui::tips::keyed("docker.corners_b"))
                    .changed()
                {
                    c.b = u.world_to_mm(v);
                }
            });
            if crate::ui::propbar::pic_button(
                ui,
                crate::ui::propbar::Pic::Lock(c.lock),
                &tr("docker.corners_lock"),
                true,
                c.lock,
            )
            .clicked()
            {
                c.lock = !c.lock;
            }
        });
    } else {
        field(
            ui,
            &tr("docker.corners_radius"),
            crate::ui::tips::pair("docker.corners_radius", "docker.corners_radius_desc"),
            &mut c.radius,
        );
    }
    app.corners = c;
    // Preview what Apply would do.
    let preview = app.corner_preview();
    let valid = !preview.is_empty();
    app.docker_preview = preview;
    ui.add_space(8.0);
    if ui
        .add_enabled(valid, egui::Button::new(tr("docker.apply")))
        .rich_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.corners_apply_desc",
        ))
        .rich_disabled_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.corners_apply_desc",
        ))
        .clicked()
    {
        app.apply_corners();
        app.docker_preview.clear();
    }
}

/// Join Curves: the joint (Extend, Chamfer, Fillet, Bezier Curve), the
/// gap tolerance and the fillet radius.
fn join_curves(app: &mut App, ui: &mut Ui) {
    use crate::corners::JoinKind;
    let u = app.units;
    let label = |k: JoinKind| {
        tr(match k {
            JoinKind::Extend => "docker.join_extend",
            JoinKind::Chamfer => "docker.join_chamfer",
            JoinKind::Fillet => "docker.join_fillet",
            JoinKind::Bezier => "docker.join_bezier",
        })
    };
    let mut j = app.join_settings;
    ui.horizontal(|ui| {
        ui.label(tr("docker.join_mode"));
        egui::ComboBox::from_id_salt("join_mode")
            .selected_text(label(j.mode))
            .width(140.0)
            .show_ui(ui, |ui| {
                for k in [
                    JoinKind::Extend,
                    JoinKind::Chamfer,
                    JoinKind::Fillet,
                    JoinKind::Bezier,
                ] {
                    ui.selectable_value(&mut j.mode, k, label(k));
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "docker.join_mode_tip",
                "docker.join_mode_desc",
            ));
    });
    let field = |ui: &mut Ui, key: &str, mm: &mut f64| {
        ui.horizontal(|ui| {
            ui.label(tr(key));
            let mut v = u.world_from_mm(*mm);
            if ui
                .add_sized(
                    [90.0, 22.0],
                    crate::ui::field::NumField::new(&mut v)
                        .range(0.0..=100_000.0)
                        .max_decimals(3)
                        .suffix(format!(" {}", u.short()))
                        .unit_mm(u.mm()),
                )
                .rich_tip(crate::ui::tips::pair(key, &format!("{key}_desc")))
                .changed()
            {
                *mm = u.world_to_mm(v);
            }
        });
    };
    field(ui, "docker.join_gap", &mut j.gap);
    if j.mode == JoinKind::Fillet {
        field(ui, "docker.corners_radius", &mut j.radius);
    }
    app.join_settings = j;
    let curves = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Path { .. }));
    ui.add_space(8.0);
    if ui
        .add_enabled(curves, egui::Button::new(tr("docker.apply")))
        .rich_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.join_apply_desc",
        ))
        .rich_disabled_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.join_apply_desc",
        ))
        .clicked()
    {
        app.join_curves();
    }
}

/// Fit Objects to Path: spacing (even or a fixed distance), the start
/// offset, turning the objects with the path and the direction, with a
/// preview of where the objects go; the last selected object is the path
/// (#1027).
fn fit_to_path(app: &mut App, ui: &mut Ui) {
    use crate::ops2::FitSpacing;
    let u = app.units;
    let mut f = app.fit_path;
    let field = |ui: &mut Ui, key: &str, mm: &mut f64, min: f64| {
        ui.horizontal(|ui| {
            ui.label(tr(key));
            let mut v = u.world_from_mm(*mm);
            if ui
                .add_sized(
                    [90.0, 22.0],
                    crate::ui::field::NumField::new(&mut v)
                        .range(min..=100_000.0)
                        .max_decimals(3)
                        .suffix(format!(" {}", u.short()))
                        .unit_mm(u.mm()),
                )
                .rich_tip(crate::ui::tips::pair(key, &format!("{key}_desc")))
                .changed()
            {
                *mm = u.world_to_mm(v);
            }
        });
    };
    dim(ui, tr("docker.fit_to_path_hint"));
    ui.strong(tr("docker.fit_to_path_spacing"));
    ui.radio_value(
        &mut f.spacing,
        FitSpacing::Even,
        tr("docker.fit_to_path_even"),
    )
    .rich_tip(crate::ui::tips::keyed("docker.fit_to_path_even"));
    ui.radio_value(
        &mut f.spacing,
        FitSpacing::Distance,
        tr("docker.fit_to_path_distance"),
    )
    .rich_tip(crate::ui::tips::keyed("docker.fit_to_path_distance"));
    if f.spacing == FitSpacing::Distance {
        field(
            ui,
            "docker.fit_to_path_distance_value",
            &mut f.distance,
            0.0,
        );
    }
    field(ui, "docker.fit_to_path_offset", &mut f.offset, -100_000.0);
    ui.checkbox(&mut f.rotate, tr("docker.fit_to_path_rotate"))
        .rich_tip(crate::ui::tips::keyed("docker.fit_to_path_rotate"));
    ui.checkbox(&mut f.reverse, tr("docker.fit_to_path_reverse"))
        .rich_tip(crate::ui::tips::keyed("docker.fit_to_path_reverse"));
    app.fit_path = f;
    let preview = app.fit_objects_to_path_preview();
    let valid = app.selection.len() >= 2;
    app.docker_preview.extend(preview);
    ui.add_space(8.0);
    if ui
        .add_enabled(valid, egui::Button::new(tr("docker.apply")))
        .rich_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.fit_to_path_apply_desc",
        ))
        .rich_disabled_tip(crate::ui::tips::pair(
            "docker.apply",
            "docker.fit_to_path_apply_desc",
        ))
        .clicked()
    {
        app.fit_objects_to_path();
        app.docker_preview.clear();
    }
}

#[cfg(test)]
mod blend_docker_tests {
    use super::*;

    /// The Blend panel shows its Color Blend choices and an Apply button
    /// in a panel of ordinary height, and Apply blends the two selected
    /// objects (#815).
    #[test]
    fn the_blend_panel_shows_color_blend_and_applies() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let r = |app: &mut App, x: f64| {
            app.new_shape(tracedraw_core::ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(x, 10.0, x + 20.0, 30.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect")
        };
        let a = r(&mut app, 10.0);
        let b = r(&mut app, 100.0);
        app.select(vec![a, b]);
        let run = |app: &mut App, events: Vec<egui::Event>| -> Vec<(String, egui::Rect)> {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(300.0, 600.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| blend(app, ui));
            out.textures_delta.clear();
            let mut texts = Vec::new();
            for s in &out.shapes {
                if let egui::Shape::Text(t) = &s.shape {
                    texts.push((t.galley.text().to_string(), t.visual_bounding_rect()));
                }
            }
            texts
        };
        run(&mut app, Vec::new());
        let texts = run(&mut app, Vec::new());
        let find = |texts: &[(String, egui::Rect)], key: &str| {
            texts.iter().find(|(t, _)| *t == tr(key)).map(|(_, r)| *r)
        };
        let color = find(&texts, "docker.blend_color_blend").expect("Color Blend shown");
        let apply = find(&texts, "docker.apply").expect("Apply shown");
        assert!(
            color.bottom() < 600.0 && apply.bottom() < 600.0,
            "{color:?} {apply:?}"
        );
        let at = apply.center();
        run(&mut app, vec![egui::Event::PointerMoved(at)]);
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        run(&mut app, vec![button(true)]);
        run(&mut app, vec![button(false)]);
        let s = app.doc().find_shape(a).expect("start");
        assert!(
            s.effects.iter().any(|e| matches!(e, Effect::Blend { .. })),
            "{:?}",
            s.effects
        );
    }
}
