//! Dockers added with the full workspace: Align and Distribute, Shaping,
//! Step and Repeat, Text, Glyphs, Color, Color Styles, Object Styles,
//! Find and Replace, Scripts, Palettes, Lens, Blend, Contour, Envelope,
//! Extrude, Bevel, Brush Strokes, Bitmap Mask, Object Data, Links,
//! Symbols, Pages, Guidelines, Fonts.

use crate::app::{App, DockerTab};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::Ui;
use tracedraw_core::geometry::Shape as _;
use tracedraw_core::{document::ShapeKind, live::Effect, Color, Command, Fill, ShapeId};

fn color_button(ui: &mut Ui, c: &mut Color) -> bool {
    let [r, g, b] = c.to_rgb8();
    let mut rgb = [r, g, b];
    if ui.color_edit_button_srgb(&mut rgb).changed() {
        *c = Color::rgb8(rgb[0], rgb[1], rgb[2]);
        true
    } else {
        false
    }
}

fn dim(ui: &mut Ui, text: String) {
    ui.label(egui::RichText::new(text).color(Tokens::TEXT_DIM).size(11.0));
}

pub fn show(app: &mut App, ui: &mut Ui, tab: DockerTab) {
    match tab {
        DockerTab::AlignDistribute => align_distribute(app, ui),
        DockerTab::Shaping => shaping(app, ui),
        DockerTab::StepAndRepeat => step_and_repeat(app, ui),
        DockerTab::Text => text(app, ui),
        DockerTab::Glyphs => glyphs(app, ui),
        DockerTab::Color => color(app, ui),
        DockerTab::ColorStyles => color_styles(app, ui),
        DockerTab::ObjectStyles => object_styles(app, ui),
        DockerTab::FindReplace => find_replace(app, ui),
        DockerTab::Scripts => scripts(app, ui),
        DockerTab::Palettes => palettes(app, ui),
        DockerTab::Lens => lens(app, ui),
        DockerTab::Blend => blend(app, ui),
        DockerTab::Contour => contour(app, ui),
        DockerTab::Envelope => envelope(app, ui),
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
        _ => {}
    }
}

fn align_distribute(app: &mut App, ui: &mut Ui) {
    use crate::ops::{Align, Distribute};
    use crate::ops2::AlignTo;
    let has = !app.selection.is_empty();
    ui.strong(tr("docker.align"));
    ui.horizontal_wrapped(|ui| {
        for (key, a) in [
            ("menu.object.align_left", Align::Left),
            ("menu.object.align_centers_h", Align::CenterH),
            ("menu.object.align_right", Align::Right),
            ("menu.object.align_top", Align::Top),
            ("menu.object.align_centers_v", Align::CenterV),
            ("menu.object.align_bottom", Align::Bottom),
        ] {
            if ui.add_enabled(has, egui::Button::new(tr(key))).clicked() {
                app.align(a);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("docker.align_to"));
        for (k, v) in [
            ("docker.align_active", AlignTo::ActiveObject),
            ("docker.align_page_edge", AlignTo::PageEdge),
            ("docker.align_page_center", AlignTo::PageCenter),
            ("docker.align_grid", AlignTo::Grid),
        ] {
            if ui.selectable_label(app.align_to == v, tr(k)).clicked() {
                app.align_to = v;
            }
        }
    });
    if ui
        .add_enabled(has, egui::Button::new(tr("menu.object.center_to_page")))
        .clicked()
    {
        app.align(Align::CenterPage);
    }
    ui.separator();
    ui.strong(tr("docker.distribute"));
    let three = app.selection.len() >= 3;
    ui.horizontal_wrapped(|ui| {
        for (key, d) in [
            ("menu.object.distribute_centers_h", Distribute::CentersH),
            ("menu.object.distribute_spacing_h", Distribute::SpacingH),
            ("menu.object.distribute_centers_v", Distribute::CentersV),
            ("menu.object.distribute_spacing_v", Distribute::SpacingV),
        ] {
            if ui.add_enabled(three, egui::Button::new(tr(key))).clicked() {
                app.distribute(d);
            }
        }
    });
    if !three {
        dim(ui, tr("docker.distribute_hint"));
    }
}

fn shaping(app: &mut App, ui: &mut Ui) {
    use crate::ops::Shaping;
    let has = !app.selection.is_empty();
    let two = app.selection.len() >= 2;
    for (key, op, need_two, hint) in [
        ("menu.object.weld", Shaping::Weld, true, "docker.weld_hint"),
        ("menu.object.trim", Shaping::Trim, true, "docker.trim_hint"),
        (
            "menu.object.intersect",
            Shaping::Intersect,
            true,
            "docker.intersect_hint",
        ),
        (
            "menu.object.simplify",
            Shaping::Simplify,
            false,
            "docker.simplify_hint",
        ),
        (
            "menu.object.front_minus_back",
            Shaping::FrontMinusBack,
            true,
            "docker.front_minus_back_hint",
        ),
        (
            "menu.object.back_minus_front",
            Shaping::BackMinusFront,
            true,
            "docker.back_minus_front_hint",
        ),
        (
            "menu.object.boundary",
            Shaping::Boundary,
            false,
            "docker.boundary_hint",
        ),
    ] {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(if need_two { two } else { has }, egui::Button::new(tr(key)))
                .clicked()
            {
                app.shaping(op);
            }
            dim(ui, tr(hint));
        });
    }
    ui.separator();
    ui.checkbox(
        &mut app.shaping_keep_source,
        tr("docker.leave_original_source"),
    );
    ui.checkbox(
        &mut app.shaping_keep_target,
        tr("docker.leave_original_target"),
    );
}

fn step_and_repeat(app: &mut App, ui: &mut Ui) {
    let u = app.units;
    let sr = &mut app.step_repeat;
    ui.horizontal(|ui| {
        ui.label(tr("docker.copies"));
        ui.add(egui::DragValue::new(&mut sr.copies).range(1..=999));
    });
    ui.strong(tr("docker.horizontal_settings"));
    ui.horizontal(|ui| {
        ui.radio_value(&mut sr.mode_x, 0, tr("docker.offset"));
        ui.radio_value(&mut sr.mode_x, 1, tr("docker.spacing"));
    });
    let mut dx = u.from_mm(sr.dx);
    if ui
        .add(
            egui::DragValue::new(&mut dx)
                .speed(0.5)
                .suffix(format!(" {}", u.short())),
        )
        .changed()
    {
        sr.dx = u.to_mm(dx);
    }
    ui.strong(tr("docker.vertical_settings"));
    ui.horizontal(|ui| {
        ui.radio_value(&mut sr.mode_y, 0, tr("docker.offset"));
        ui.radio_value(&mut sr.mode_y, 1, tr("docker.spacing"));
    });
    let mut dy = u.from_mm(sr.dy);
    if ui
        .add(
            egui::DragValue::new(&mut dy)
                .speed(0.5)
                .suffix(format!(" {}", u.short())),
        )
        .changed()
    {
        sr.dy = u.to_mm(dy);
    }
    let has = !app.selection.is_empty();
    if ui
        .add_enabled(has, egui::Button::new(tr("docker.apply")))
        .clicked()
    {
        app.step_and_repeat_apply();
    }
}

fn text(app: &mut App, ui: &mut Ui) {
    let texts: Vec<_> = app
        .selected_shapes()
        .into_iter()
        .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
        .collect();
    ui.strong(tr("docker.character"));
    let mut changed = false;
    egui::ComboBox::from_id_salt("dk_font")
        .selected_text(app.text_font.clone())
        .width(180.0)
        .show_ui(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for f in app.font_families.clone() {
                        if ui.selectable_label(app.text_font == f, &f).clicked() {
                            app.text_font = f;
                            changed = true;
                        }
                    }
                });
        });
    ui.horizontal(|ui| {
        changed |= ui
            .add(
                egui::DragValue::new(&mut app.text_size_pt)
                    .range(1.0..=999.0)
                    .suffix(" pt"),
            )
            .changed();
        if ui
            .selectable_label(app.text_bold, egui::RichText::new("B").strong())
            .clicked()
        {
            app.text_bold = !app.text_bold;
            changed = true;
        }
        if ui
            .selectable_label(app.text_italic, egui::RichText::new("I").italics())
            .clicked()
        {
            app.text_italic = !app.text_italic;
            changed = true;
        }
    });
    if changed {
        app.apply_text_style();
    }
    // Span-level attributes of the first selected text.
    if let Some(t) = texts.first() {
        if let ShapeKind::Text {
            spans, para, frame, ..
        } = &t.kind
        {
            let mut sp = spans.first().cloned().unwrap_or_else(|| {
                tracedraw_core::TextSpan::new("", &app.text_font, app.text_size_pt)
            });
            let mut sc = false;
            ui.horizontal(|ui| {
                ui.label(tr("docker.tracking"));
                sc |= ui
                    .add(
                        egui::DragValue::new(&mut sp.tracking_pct)
                            .range(-50.0..=200.0)
                            .suffix(" %"),
                    )
                    .changed();
                ui.label(tr("docker.baseline_shift"));
                sc |= ui
                    .add(
                        egui::DragValue::new(&mut sp.baseline_shift_pt)
                            .range(-100.0..=100.0)
                            .suffix(" pt"),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                sc |= ui
                    .checkbox(&mut sp.underline, tr("docker.underline"))
                    .changed();
                sc |= ui
                    .checkbox(&mut sp.strikethrough, tr("docker.strikethrough"))
                    .changed();
            });
            ui.horizontal(|ui| {
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
                let new_spans: Vec<_> = spans
                    .iter()
                    .map(|s| tracedraw_core::TextSpan {
                        tracking_pct: sp.tracking_pct,
                        baseline_shift_pt: sp.baseline_shift_pt,
                        underline: sp.underline,
                        strikethrough: sp.strikethrough,
                        features: sp.features.clone(),
                        ..s.clone()
                    })
                    .collect();
                app.set_text_spans(t.id, new_spans);
            }
            ui.separator();
            ui.strong(tr("docker.paragraph"));
            let mut p = para.clone();
            let mut pc = false;
            ui.horizontal(|ui| {
                for (a, k) in [
                    (tracedraw_core::TextAlign::Left, "docker.align_left_short"),
                    (
                        tracedraw_core::TextAlign::Center,
                        "docker.align_center_short",
                    ),
                    (tracedraw_core::TextAlign::Right, "docker.align_right_short"),
                    (
                        tracedraw_core::TextAlign::Justify,
                        "docker.align_justify_short",
                    ),
                ] {
                    if ui.selectable_label(app.text_align == a, tr(k)).clicked() {
                        app.text_align = a;
                        app.apply_text_style();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("docker.leading"));
                pc |= ui
                    .add(
                        egui::DragValue::new(&mut p.leading_pct)
                            .range(50.0..=400.0)
                            .suffix(" %"),
                    )
                    .changed();
            });
            let u = app.units;
            for (k, v) in [
                ("docker.space_before", &mut p.space_before),
                ("docker.space_after", &mut p.space_after),
                ("docker.first_line_indent", &mut p.first_line_indent),
                ("docker.left_indent", &mut p.left_indent),
                ("docker.right_indent", &mut p.right_indent),
            ] {
                let mut x = u.from_mm(*v);
                ui.horizontal(|ui| {
                    ui.label(tr(k));
                    if ui
                        .add(
                            egui::DragValue::new(&mut x)
                                .speed(0.2)
                                .suffix(format!(" {}", u.short())),
                        )
                        .changed()
                    {
                        *v = u.to_mm(x);
                        pc = true;
                    }
                });
            }
            if frame.is_some() {
                ui.horizontal(|ui| {
                    ui.label(tr("dialog.columns"));
                    pc |= ui
                        .add(egui::DragValue::new(&mut p.columns).range(1..=8))
                        .changed();
                    pc |= ui
                        .checkbox(&mut p.bullets, tr("menu.text.bullets"))
                        .changed();
                    pc |= ui
                        .checkbox(&mut p.hyphenate, tr("docker.hyphenate"))
                        .changed();
                    pc |= ui
                        .checkbox(&mut p.fit_to_frame, tr("menu.text.frame_fit_text"))
                        .changed();
                });
            }
            if pc {
                app.set_paragraph_style(t.id, p);
            }
        }
    } else {
        dim(ui, tr("docker.text_hint"));
    }
}

fn glyphs(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(tr("docker.font"));
        egui::ComboBox::from_id_salt("gl_font")
            .selected_text(app.text_font.clone())
            .width(160.0)
            .show_ui(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for f in app.font_families.clone() {
                            if ui.selectable_label(app.text_font == f, &f).clicked() {
                                app.text_font = f;
                            }
                        }
                    });
            });
    });
    ui.horizontal(|ui| {
        ui.label(tr("docker.filter"));
        ui.text_edit_singleline(&mut app.glyph_filter);
    });
    let blocks: [(&str, u32, u32); 8] = [
        ("Basic Latin", 0x20, 0x7E),
        ("Latin-1", 0xA0, 0xFF),
        ("Latin Extended", 0x100, 0x17F),
        ("Greek", 0x391, 0x3C9),
        ("Cyrillic", 0x410, 0x44F),
        ("Punctuation", 0x2010, 0x205E),
        ("Currency", 0x20A0, 0x20BF),
        ("Arrows and symbols", 0x2190, 0x21FF),
    ];
    let filter = app.glyph_filter.to_lowercase();
    let mut insert: Option<char> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (name, from, to) in blocks {
            if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
                continue;
            }
            ui.label(egui::RichText::new(name).strong().size(11.0));
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
                for cp in from..=to {
                    let Some(ch) = char::from_u32(cp) else {
                        continue;
                    };
                    if ch.is_control() {
                        continue;
                    }
                    let b = egui::Button::new(egui::RichText::new(ch.to_string()).size(14.0))
                        .min_size(egui::vec2(22.0, 22.0));
                    if ui.add(b).on_hover_text(format!("U+{cp:04X}")).clicked() {
                        insert = Some(ch);
                    }
                }
            });
        }
    });
    if let Some(ch) = insert {
        if app.text_edit.is_some() {
            app.insert_text(&ch.to_string());
        } else {
            let c = app.page_rect().center();
            app.start_text(c, None);
            app.text_insert(&ch.to_string());
            app.finish_text();
        }
    }
}

fn color(app: &mut App, ui: &mut Ui) {
    let models = ["RGB", "CMYK", "HSB", "HSL", "Lab", "Grayscale", "YIQ"];
    ui.horizontal(|ui| {
        ui.label(tr("docker.model"));
        egui::ComboBox::from_id_salt("col_model")
            .selected_text(models[app.color_model])
            .show_ui(ui, |ui| {
                for (i, m) in models.iter().enumerate() {
                    if ui.selectable_label(app.color_model == i, *m).clicked() {
                        app.color_model = i;
                        app.mixer_color = app.mixer_color.convert_to(m);
                    }
                }
            });
    });
    let mut c = app.mixer_color;
    let mut changed = false;
    macro_rules! slider {
        ($v:expr, $label:expr, $max:expr) => {
            ui.horizontal(|ui| {
                ui.label($label);
                let mut x = *$v * $max;
                if ui
                    .add(egui::Slider::new(&mut x, 0.0..=$max).show_value(true))
                    .changed()
                {
                    *$v = x / $max;
                    changed = true;
                }
            });
        };
    }
    match &mut c {
        Color::Rgb { r, g, b } => {
            slider!(r, "R", 255.0);
            slider!(g, "G", 255.0);
            slider!(b, "B", 255.0);
        }
        Color::Cmyk { c, m, y, k } => {
            slider!(c, "C", 100.0);
            slider!(m, "M", 100.0);
            slider!(y, "Y", 100.0);
            slider!(k, "K", 100.0);
        }
        Color::Hsb { h, s, b } => {
            slider!(h, "H", 360.0);
            slider!(s, "S", 100.0);
            slider!(b, "B", 100.0);
        }
        Color::Hsl { h, s, l } => {
            slider!(h, "H", 360.0);
            slider!(s, "S", 100.0);
            slider!(l, "L", 100.0);
        }
        Color::Lab { l, a, b } => {
            ui.horizontal(|ui| {
                ui.label("L");
                changed |= ui.add(egui::Slider::new(l, 0.0..=100.0)).changed();
            });
            ui.horizontal(|ui| {
                ui.label("a");
                changed |= ui.add(egui::Slider::new(a, -128.0..=127.0)).changed();
            });
            ui.horizontal(|ui| {
                ui.label("b");
                changed |= ui.add(egui::Slider::new(b, -128.0..=127.0)).changed();
            });
        }
        Color::Gray { v } => {
            slider!(v, "L", 100.0);
        }
        Color::Yiq { y, i, q } => {
            slider!(y, "Y", 255.0);
            slider!(i, "I", 255.0);
            slider!(q, "Q", 255.0);
        }
        Color::Registration => {}
    }
    let [r, g, b] = c.to_rgb8();
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(60.0, 28.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 2.0, egui::Color32::from_rgb(r, g, b));
        let mut hex = c.to_hex();
        if ui
            .add(egui::TextEdit::singleline(&mut hex).desired_width(70.0))
            .lost_focus()
        {
            if let Some(nc) = Color::from_hex(&hex) {
                c = nc.convert_to(models[app.color_model]);
                changed = true;
            }
        }
        if !c.in_cmyk_gamut(6.0) {
            ui.label(egui::RichText::new("\u{26A0}").color(egui::Color32::from_rgb(200, 120, 0)))
                .on_hover_text(tr("docker.gamut_warning"));
        }
    });
    if changed {
        app.mixer_color = c;
    }
    ui.horizontal(|ui| {
        if ui.button(tr("docker.fill")).clicked() {
            app.apply_fill(Fill::Solid(c));
        }
        if ui.button(tr("docker.outline")).clicked() {
            app.apply_outline_color(Some(c));
        }
        if ui.button(tr("docker.add_to_palette")).clicked() {
            let mut colors = app.doc().palette.clone();
            if !colors.contains(&c) {
                colors.push(c);
            }
            app.run(Command::SetDocumentPalette { colors });
        }
        if ui.button(tr("docker.eyedropper")).clicked() {
            app.set_tool(crate::tools::Tool::ColorEyedropper);
        }
    });
    if let Some(ec) = app.eyedropper_color {
        if ui
            .button(trf("docker.use_sampled", &[("c", &ec.to_hex())]))
            .clicked()
        {
            app.mixer_color = ec.convert_to(models[app.color_model]);
        }
    }
}

fn color_styles(app: &mut App, ui: &mut Ui) {
    let styles = app.doc().color_styles.clone();
    ui.horizontal(|ui| {
        if ui.button(tr("docker.new_from_selection")).clicked() {
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
        if ui.button(tr("docker.new_color_style")).clicked() {
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
                .lost_focus();
            if ui.small_button(tr("docker.fill")).clicked() {
                app.apply_fill(Fill::Solid(cs.color));
            }
            if ui.small_button(tr("docker.outline")).clicked() {
                app.apply_outline_color(Some(cs.color));
            }
            if ui.small_button("x").clicked() {
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
            if ui.button(tr(h.key())).clicked() {
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

fn object_styles(app: &mut App, ui: &mut Ui) {
    let styles = app.doc().object_styles.clone();
    if ui
        .add_enabled(
            !app.selection.is_empty(),
            egui::Button::new(tr("docker.new_style_from_selection")),
        )
        .clicked()
    {
        if let Some(s) = app.selected_shapes().first() {
            let mut st = styles.clone();
            st.push(tracedraw_core::ObjectStyle {
                name: format!("Style {}", st.len() + 1),
                fill: s.fill.clone(),
                stroke: s.stroke.clone(),
            });
            app.run(Command::SetObjectStyles { styles: st });
        }
    }
    let mut st = styles.clone();
    let mut changed = false;
    let mut remove = None;
    for (i, os) in st.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let swatch = crate::app::fill_preview_color(&os.fill).unwrap_or(egui::Color32::WHITE);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, swatch);
            changed |= ui
                .add(egui::TextEdit::singleline(&mut os.name).desired_width(90.0))
                .lost_focus();
            if ui.small_button(tr("docker.apply")).clicked() {
                let shapes = app.selection.clone();
                let cmds = vec![
                    Command::SetFill {
                        shapes: shapes.clone(),
                        fill: os.fill.clone(),
                    },
                    Command::SetStroke {
                        shapes,
                        stroke: os.stroke.clone(),
                    },
                ];
                let _ = app.engine.run_batch("Apply Style", &cmds);
            }
            if ui
                .small_button(tr("docker.update_from_selection"))
                .clicked()
            {
                if let Some(s) = app.selected_shapes().first() {
                    os.fill = s.fill.clone();
                    os.stroke = s.stroke.clone();
                    changed = true;
                }
            }
            if ui.small_button("x").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        st.remove(i);
        changed = true;
    }
    if changed {
        app.run(Command::SetObjectStyles { styles: st });
    }
    if styles.is_empty() {
        dim(ui, tr("docker.object_styles_hint"));
    }
}

fn find_replace(app: &mut App, ui: &mut Ui) {
    let mut st = std::mem::take(&mut app.find_state);
    ui.horizontal(|ui| {
        ui.radio_value(&mut st.kind, 0, tr("dialog.find_text"));
        ui.radio_value(&mut st.kind, 1, tr("dialog.find_objects"));
    });
    ui.horizontal(|ui| {
        ui.label(tr("dialog.find"));
        ui.text_edit_singleline(&mut st.find);
    });
    if st.kind == 0 {
        ui.horizontal(|ui| {
            ui.label(tr("dialog.replace_with"));
            ui.text_edit_singleline(&mut st.replace);
        });
        ui.checkbox(&mut st.match_case, tr("dialog.match_case"));
        ui.checkbox(&mut st.whole_word, tr("dialog.whole_word"));
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("dialog.find_next")).clicked() {
            st.results =
                crate::export::find_shapes(app, &st.find, st.kind, st.match_case, st.whole_word);
            if !st.results.is_empty() {
                st.cursor = (st.cursor + 1) % st.results.len();
                let id = st.results[st.cursor];
                app.select(vec![id]);
            }
        }
        if ui.button(tr("dialog.find_all")).clicked() {
            st.results =
                crate::export::find_shapes(app, &st.find, st.kind, st.match_case, st.whole_word);
            app.select(st.results.clone());
        }
        if st.kind == 0 {
            if ui.button(tr("dialog.replace")).clicked() {
                crate::export::replace_text(
                    app,
                    &st.find,
                    &st.replace,
                    st.match_case,
                    st.whole_word,
                    false,
                );
            }
            if ui.button(tr("dialog.replace_all")).clicked() {
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
    ui.horizontal(|ui| {
        if ui.button(tr("docker.run")).clicked() {
            let src = app.script_source.clone();
            app.run_script(&src);
        }
        if ui
            .selectable_label(app.recording.is_some(), tr("docker.record"))
            .clicked()
        {
            app.toggle_recording();
        }
        if ui.button(tr("docker.load")).clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("JavaScript", &["js"])
                .pick_file()
            {
                if let Ok(s) = std::fs::read_to_string(p) {
                    app.script_source = s;
                }
            }
        }
        if ui.button(tr("docker.save")).clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("JavaScript", &["js"])
                .set_file_name("macro.js")
                .save_file()
            {
                let _ = std::fs::write(p, &app.script_source);
            }
        }
        if ui.button(tr("docker.clear")).clicked() {
            app.scripts_output.clear();
        }
    });
    if app.script_source.is_empty() {
        app.script_source = "// Example\nconst r = ActiveLayer.CreateRectangle(20, 20, 60, 40);\nr.Fill.ApplyUniformFill(Color.CMYK(0, 100, 100, 0));\nprint('Objects: ' + ActivePage.Shapes.Count);\n".into();
    }
    egui::ScrollArea::vertical()
        .id_salt("script_src")
        .max_height(220.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut app.script_source)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(10),
            );
        });
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
    dim(ui, tr("docker.scripts_hint"));
}

fn palettes(app: &mut App, ui: &mut Ui) {
    let n = app.palettes.len();
    for i in 0..n {
        let name = app.palettes[i].display_name();
        let count = app.palettes[i].colors.len();
        let on = app.visible_palettes.contains(&i);
        ui.horizontal(|ui| {
            let mut v = on;
            if ui.checkbox(&mut v, format!("{name} ({count})")).changed() {
                app.toggle_palette(i);
            }
            if ui.small_button(tr("docker.edit")).clicked() {
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
        if ui.button(tr("menu.window.palette_open")).clicked() {
            app.open_palette_file();
        }
        if ui.button(tr("menu.window.palette_from_document")).clicked() {
            app.palette_from_document();
        }
        if ui
            .button(tr("menu.window.palette_from_selection"))
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
        });
    let l = &mut app.lens;
    match l.kind {
        1 => {
            ui.add(egui::Slider::new(&mut l.rate, -100.0..=100.0).text(tr("docker.rate")));
        }
        2 | 3 | 10 => {
            ui.add(egui::Slider::new(&mut l.rate, 0.0..=100.0).text(tr("docker.rate")));
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
            ui.add(egui::Slider::new(&mut l.rate, -100.0..=100.0).text(tr("docker.rate")));
        }
        6 => {
            ui.add(egui::Slider::new(&mut l.rate, 0.0..=100.0).text(tr("docker.palette_rotation")));
        }
        8 => {
            ui.add(egui::Slider::new(&mut l.amount, 1.0..=10.0).text(tr("docker.amount")));
        }
        9 | 11 => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.color"));
                color_button(ui, &mut l.color);
            });
        }
        _ => {}
    }
    ui.checkbox(&mut l.frozen, tr("docker.frozen"));
    let has = !app.selection.is_empty();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.apply")))
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
            .clicked()
        {
            app.remove_effects_of_kind(&Effect::Lens(tracedraw_core::live::Lens::Invert));
        }
    });
    dim(ui, tr("docker.lens_hint"));
}

fn blend(app: &mut App, ui: &mut Ui) {
    let cur = current_effect(app, |e| matches!(e, Effect::Blend { .. }));
    let (mut steps, mut accel_o, mut accel_c, mut rot, mut rotate_on_path) = match &cur {
        Some(Effect::Blend {
            steps,
            accel_objects,
            accel_colors,
            rotation,
            rotate_on_path,
            ..
        }) => (
            *steps,
            *accel_objects,
            *accel_colors,
            *rotation,
            *rotate_on_path,
        ),
        _ => (app.blend_steps, 0.0, 0.0, 0.0, false),
    };
    let mut changed = false;
    changed |= ui
        .add(egui::Slider::new(&mut steps, 1..=200).text(tr("docker.steps")))
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(&mut rot, -360.0..=360.0)
                .text(tr("docker.rotation"))
                .suffix("°"),
        )
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut accel_o, -1.0..=1.0).text(tr("docker.object_acceleration")))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut accel_c, -1.0..=1.0).text(tr("docker.color_acceleration")))
        .changed();
    changed |= ui
        .checkbox(&mut rotate_on_path, tr("docker.rotate_all_objects"))
        .changed();
    app.blend_steps = steps;
    if let Some(Effect::Blend { end, path, .. }) = cur {
        if changed {
            app.push_effect(
                Effect::Blend {
                    end,
                    steps,
                    accel_objects: accel_o,
                    accel_colors: accel_c,
                    rotation: rot,
                    path,
                    rotate_on_path,
                },
                true,
            );
        }
        ui.horizontal(|ui| {
            if ui.button(tr("docker.new_path")).clicked() {
                app.pending_blend_path = true;
                app.status = tr("status.click_blend_path");
            }
            if ui.button(tr("docker.clear_blend")).clicked() {
                app.remove_effects_of_kind(&Effect::Blend {
                    end: Box::new(tracedraw_core::Shape::new(
                        ShapeId(0),
                        ShapeKind::Rect {
                            rect: Default::default(),
                            radius: 0.0,
                        },
                    )),
                    steps: 1,
                    accel_objects: 0.0,
                    accel_colors: 0.0,
                    rotation: 0.0,
                    path: None,
                    rotate_on_path: false,
                });
            }
            if ui.button(tr("docker.break_apart")).clicked() {
                app.flatten_effects();
            }
        });
    } else {
        let two = app.selection.len() == 2;
        if ui
            .add_enabled(two, egui::Button::new(tr("docker.apply")))
            .clicked()
        {
            app.blend_selection(steps, accel_o, accel_c, rot);
        }
        if !two {
            dim(ui, tr("docker.blend_hint"));
        }
    }
}

fn contour(app: &mut App, ui: &mut Ui) {
    let cur = current_effect(app, |e| matches!(e, Effect::Contour { .. }));
    let (
        mut steps,
        mut offset,
        mut outside,
        mut to_center,
        mut fill_to,
        mut outline_to,
        mut blend_mode,
    ) = match &cur {
        Some(Effect::Contour {
            steps,
            offset,
            outside,
            to_center,
            fill_to,
            outline_to,
            color_blend,
        }) => (
            *steps,
            *offset,
            *outside,
            *to_center,
            *fill_to,
            *outline_to,
            *color_blend,
        ),
        _ => (
            app.contour_steps,
            app.contour_offset,
            app.contour_direction == crate::tools2::ContourDirection::Outside,
            false,
            app.contour_color,
            None,
            0,
        ),
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui
            .selectable_label(to_center, tr("docker.to_center"))
            .clicked()
        {
            to_center = true;
            changed = true;
        }
        if ui
            .selectable_label(!to_center && !outside, tr("docker.inside"))
            .clicked()
        {
            to_center = false;
            outside = false;
            changed = true;
        }
        if ui
            .selectable_label(!to_center && outside, tr("docker.outside"))
            .clicked()
        {
            to_center = false;
            outside = true;
            changed = true;
        }
    });
    if !to_center {
        changed |= ui
            .add(
                egui::Slider::new(&mut steps, 1..=999)
                    .text(tr("docker.steps"))
                    .logarithmic(true),
            )
            .changed();
    }
    let u = app.units;
    let mut o = u.from_mm(offset);
    if ui
        .add(
            egui::Slider::new(&mut o, 0.01..=u.from_mm(50.0))
                .text(tr("docker.offset"))
                .logarithmic(true),
        )
        .changed()
    {
        offset = u.to_mm(o);
        changed = true;
    }
    ui.horizontal(|ui| {
        ui.label(tr("docker.fill_color"));
        changed |= color_button(ui, &mut fill_to);
        ui.label(tr("docker.outline_color"));
        let mut oc = outline_to.unwrap_or(fill_to);
        if color_button(ui, &mut oc) {
            outline_to = Some(oc);
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label(tr("docker.color_blend"));
        for (i, k) in [
            (0u8, "docker.direct"),
            (1, "docker.clockwise"),
            (2, "docker.counterclockwise"),
        ] {
            if ui.selectable_label(blend_mode == i, tr(k)).clicked() {
                blend_mode = i;
                changed = true;
            }
        }
    });
    app.contour_steps = steps;
    app.contour_offset = offset;
    app.contour_color = fill_to;
    app.contour_direction = if outside {
        crate::tools2::ContourDirection::Outside
    } else {
        crate::tools2::ContourDirection::Inside
    };
    let effect = Effect::Contour {
        steps,
        offset,
        outside,
        to_center,
        fill_to,
        outline_to,
        color_blend: blend_mode,
    };
    let has = !app.selection.is_empty();
    if cur.is_some() && changed {
        app.push_effect(effect.clone(), true);
    }
    ui.horizontal(|ui| {
        if cur.is_none()
            && ui
                .add_enabled(has, egui::Button::new(tr("docker.apply")))
                .clicked()
        {
            app.push_effect(effect.clone(), true);
        }
        if cur.is_some() {
            if ui.button(tr("docker.remove")).clicked() {
                app.remove_effects_of_kind(&effect);
            }
            if ui.button(tr("docker.break_apart")).clicked() {
                app.flatten_effects();
            }
        }
    });
}

fn envelope(app: &mut App, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        for m in crate::effects_ui::EnvelopeMode::ALL {
            if ui
                .selectable_label(app.envelope_mode == m, tr(m.key()))
                .clicked()
            {
                app.envelope_mode = m;
            }
        }
    });
    ui.checkbox(&mut app.envelope_keep_lines, tr("docker.keep_lines"));
    let has = !app.selection.is_empty();
    let cur = current_effect(app, |e| matches!(e, Effect::Envelope { .. }));
    ui.horizontal(|ui| {
        if cur.is_none()
            && ui
                .add_enabled(has, egui::Button::new(tr("docker.add_new")))
                .clicked()
        {
            for s in app.selected_shapes() {
                let b = s.local_path().bounding_box();
                let nodes = tracedraw_core::live::envelope_default(b);
                let mut effects = s.effects.clone();
                effects.push(Effect::Envelope {
                    nodes,
                    keep_lines: app.envelope_keep_lines,
                });
                app.run(Command::SetEffects {
                    shape: s.id,
                    effects,
                });
            }
            app.set_tool(crate::tools::Tool::Envelope);
        }
        if cur.is_some() {
            if ui.button(tr("docker.remove")).clicked() {
                app.remove_effects_of_kind(&Effect::Envelope {
                    nodes: Vec::new(),
                    keep_lines: false,
                });
            }
            if ui.button(tr("docker.break_apart")).clicked() {
                app.flatten_effects();
            }
        }
    });
    ui.strong(tr("docker.presets"));
    ui.horizontal_wrapped(|ui| {
        for (k, preset) in [
            ("docker.preset_arch", 0),
            ("docker.preset_bulge", 1),
            ("docker.preset_flag", 2),
            ("docker.preset_perspective", 3),
            ("docker.preset_circle", 4),
        ] {
            if ui.add_enabled(has, egui::Button::new(tr(k))).clicked() {
                app.apply_envelope_preset(preset);
            }
        }
    });
    dim(ui, tr("docker.envelope_hint"));
}

fn extrude(app: &mut App, ui: &mut Ui) {
    let cur = current_effect(app, |e| matches!(e, Effect::Extrude { .. }));
    if let Some(Effect::Extrude {
        depth,
        vanishing,
        amount,
        shade_from,
        shade_to,
        light_angle,
        light_intensity,
        bevel,
    }) = &cur
    {
        if app.extrude_synced_to != app.selection.first().copied() {
            app.extrude.depth = *depth;
            app.extrude.use_vanishing = vanishing.is_some();
            app.extrude.amount = *amount;
            app.extrude.shade = shade_from.is_some();
            if let Some(c) = shade_from {
                app.extrude.shade_from = *c;
            }
            if let Some(c) = shade_to {
                app.extrude.shade_to = *c;
            }
            app.extrude.light_angle = *light_angle;
            app.extrude.light_intensity = *light_intensity;
            app.extrude.bevel = *bevel;
            app.extrude_synced_to = app.selection.first().copied();
        }
    }
    let e = &mut app.extrude;
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .selectable_label(!e.use_vanishing, tr("docker.parallel"))
            .clicked()
            .then(|| e.use_vanishing = false)
            .is_some();
        changed |= ui
            .selectable_label(e.use_vanishing, tr("docker.vanishing_point"))
            .clicked()
            .then(|| e.use_vanishing = true)
            .is_some();
    });
    if e.use_vanishing {
        changed |= ui
            .add(egui::Slider::new(&mut e.amount, 0.0..=0.95).text(tr("docker.depth")))
            .changed();
    } else {
        ui.horizontal(|ui| {
            ui.label(tr("docker.depth"));
            changed |= ui
                .add(egui::DragValue::new(&mut e.depth.x).speed(0.2))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut e.depth.y).speed(0.2))
                .changed();
        });
    }
    changed |= ui.checkbox(&mut e.shade, tr("docker.shade")).changed();
    if e.shade {
        ui.horizontal(|ui| {
            ui.label(tr("docker.from"));
            changed |= color_button(ui, &mut e.shade_from);
            ui.label(tr("docker.to"));
            changed |= color_button(ui, &mut e.shade_to);
        });
    }
    changed |= ui
        .add(
            egui::Slider::new(&mut e.light_angle, 0.0..=360.0)
                .text(tr("docker.light_angle"))
                .suffix("°"),
        )
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(&mut e.light_intensity, 0.0..=100.0)
                .text(tr("docker.light_intensity")),
        )
        .changed();
    let center = app
        .selection_bounds()
        .map(|b| b.center())
        .unwrap_or_default();
    let ex = app.extrude.clone();
    let effect = Effect::Extrude {
        depth: ex.depth,
        vanishing: ex
            .use_vanishing
            .then_some(tracedraw_core::Point::new(center.x + 60.0, center.y + 40.0)),
        amount: ex.amount,
        shade_from: ex.shade.then_some(ex.shade_from),
        shade_to: ex.shade.then_some(ex.shade_to),
        light_angle: ex.light_angle,
        light_intensity: ex.light_intensity,
        bevel: ex.bevel,
    };
    let has = !app.selection.is_empty();
    if cur.is_some() && changed {
        app.push_effect(effect.clone(), true);
    }
    ui.horizontal(|ui| {
        if cur.is_none()
            && ui
                .add_enabled(has, egui::Button::new(tr("docker.apply")))
                .clicked()
        {
            app.push_effect(effect.clone(), true);
        }
        if cur.is_some() {
            if ui.button(tr("docker.remove")).clicked() {
                app.remove_effects_of_kind(&effect);
            }
            if ui.button(tr("docker.break_apart")).clicked() {
                app.flatten_effects();
            }
        }
    });
}

fn bevel(app: &mut App, ui: &mut Ui) {
    let cur = current_effect(app, |e| matches!(e, Effect::Bevel { .. }));
    let b = &mut app.bevel;
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui
            .selectable_label(b.style == 0, tr("docker.soft_edge"))
            .clicked()
            .then(|| b.style = 0)
            .is_some();
        changed |= ui
            .selectable_label(b.style == 1, tr("docker.emboss"))
            .clicked()
            .then(|| b.style = 1)
            .is_some();
    });
    changed |= ui
        .add(
            egui::Slider::new(&mut b.distance, 0.1..=50.0)
                .text(tr("docker.distance"))
                .logarithmic(true),
        )
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(&mut b.light_angle, 0.0..=360.0)
                .text(tr("docker.light_angle"))
                .suffix("°"),
        )
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut b.intensity, 0.0..=100.0).text(tr("docker.intensity")))
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
                .clicked()
        {
            app.push_effect(effect.clone(), true);
        }
        if cur.is_some() && ui.button(tr("docker.remove")).clicked() {
            app.remove_effects_of_kind(&effect);
        }
    });
}

fn brush_strokes(app: &mut App, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        for m in crate::media::MediaMode::ALL {
            if ui
                .selectable_label(app.media_mode == m, tr(m.key()))
                .clicked()
            {
                app.media_mode = m;
            }
        }
    });
    let u = app.units;
    let mut w = u.from_mm(app.media_width);
    if ui
        .add(
            egui::Slider::new(&mut w, 0.1..=u.from_mm(50.0))
                .text(tr("docker.stroke_width"))
                .logarithmic(true),
        )
        .changed()
    {
        app.media_width = u.to_mm(w);
    }
    match app.media_mode {
        crate::media::MediaMode::Calligraphic => {
            ui.add(
                egui::Slider::new(&mut app.media_angle, 0.0..=180.0)
                    .text(tr("docker.nib_angle"))
                    .suffix("°"),
            );
        }
        crate::media::MediaMode::Preset | crate::media::MediaMode::Brush => {
            ui.horizontal_wrapped(|ui| {
                for (i, k) in crate::media::BRUSH_KEYS.iter().enumerate() {
                    if ui.selectable_label(app.media_preset == i, tr(k)).clicked() {
                        app.media_preset = i;
                    }
                }
            });
        }
        crate::media::MediaMode::Sprayer => {
            ui.horizontal_wrapped(|ui| {
                for (i, k) in crate::media::SPRAY_KEYS.iter().enumerate() {
                    if ui.selectable_label(app.media_preset == i, tr(k)).clicked() {
                        app.media_preset = i;
                    }
                }
            });
            ui.add(
                egui::Slider::new(&mut app.media_spacing, 1.0..=50.0).text(tr("docker.spacing")),
            );
        }
        crate::media::MediaMode::Expression => {
            ui.add(
                egui::Slider::new(&mut app.media_pressure, 0.0..=1.0).text(tr("docker.pressure")),
            );
        }
    }
    ui.horizontal(|ui| {
        ui.label(tr("docker.smoothing"));
        ui.add(egui::Slider::new(&mut app.media_smoothing, 0.0..=100.0));
    });
    let has_curve = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Path { .. }));
    if ui
        .add_enabled(has_curve, egui::Button::new(tr("docker.apply_to_curve")))
        .clicked()
    {
        app.apply_media_to_selection();
    }
    dim(ui, tr("docker.brush_strokes_hint"));
}

fn bitmap_mask(app: &mut App, ui: &mut Ui) {
    let has_bitmap = app
        .selected_shapes()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Bitmap { .. }));
    ui.label(tr("docker.bitmap_mask_hint"));
    let mut remove = None;
    for (i, c) in app.mask_colors.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            color_button(ui, c);
            ui.label(c.to_hex());
            if ui.small_button("x").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        app.mask_colors.remove(i);
    }
    ui.horizontal(|ui| {
        if ui.button(tr("docker.add_color")).clicked() {
            app.mask_colors
                .push(app.eyedropper_color.unwrap_or(Color::WHITE));
        }
        if ui.button(tr("docker.eyedropper")).clicked() {
            app.set_tool(crate::tools::Tool::ColorEyedropper);
        }
    });
    ui.add(egui::Slider::new(&mut app.mask_tolerance, 0..=255).text(tr("docker.tolerance")));
    if ui
        .add_enabled(
            has_bitmap && !app.mask_colors.is_empty(),
            egui::Button::new(tr("docker.apply")),
        )
        .clicked()
    {
        let colors = app.mask_colors.clone();
        let tol = app.mask_tolerance;
        app.bitmap_color_mask(&colors, tol);
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
                .lost_focus();
            changed |= ui
                .add(egui::TextEdit::singleline(v).desired_width(120.0))
                .lost_focus();
            if ui.small_button("x").clicked() {
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
        if ui.button(tr("docker.add_field")).clicked() {
            data.push((format!("Field {}", data.len() + 1), String::new()));
            changed = true;
        }
        for k in ["Name", "Cost", "Comments", "CDRStaticID"] {
            if !data.iter().any(|(f, _)| f == k) && ui.small_button(k).clicked() {
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
    if ui.text_edit_singleline(&mut link).lost_focus() {
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
            if ui.small_button(tr(k)).clicked() && !link.starts_with(prefix) {
                let ids = app.selection.clone();
                app.run(Command::SetLink {
                    shapes: ids,
                    link: Some(format!("{prefix}{link}")),
                });
            }
        }
        if ui.small_button(tr("docker.remove")).clicked() {
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
        if ui.small_button(&n).clicked() {
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
            .clicked()
        {
            app.create_symbol_from_selection();
        }
        if ui.button(tr("docker.revert_to_objects")).clicked() {
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
            if ui.small_button(tr("docker.insert")).clicked() {
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
        if ui.button(tr("menu.layout.insert_page")).clicked() {
            app.add_page();
        }
        if ui
            .add_enabled(n > 1, egui::Button::new(tr("menu.layout.delete_page")))
            .clicked()
        {
            app.delete_page();
        }
        if ui.button(tr("menu.layout.duplicate_page")).clicked() {
            let page = app.page;
            app.run(Command::DuplicatePage { page });
        }
        if ui
            .selectable_label(app.page_sorter, tr("menu.view.page_sorter"))
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
            if i > 0 && ui.small_button("\u{25B2}").clicked() {
                app.run(Command::MovePage {
                    page: *id,
                    to: i - 1,
                });
            }
            if i + 1 < n && ui.small_button("\u{25BC}").clicked() {
                app.run(Command::MovePage {
                    page: *id,
                    to: i + 1,
                });
            }
        });
    }
}

fn guidelines(app: &mut App, ui: &mut Ui) {
    guidelines_editor(app, ui);
}

/// Guideline list shared by the docker and Options > Guidelines: horizontal,
/// vertical and angled guides with editable positions, plus presets.
pub fn guidelines_editor(app: &mut App, ui: &mut Ui) {
    use tracedraw_core::document::Guide;
    let page = app.page;
    let guides = app
        .doc()
        .page(page)
        .map(|p| p.guides.clone())
        .unwrap_or_default();
    let u = app.units;
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.show_guides, tr("options.show_guidelines"));
        ui.checkbox(&mut app.snap.guides, tr("options.snap_to_guidelines"));
    });
    let unit_field = |ui: &mut Ui, v: &mut f64| -> bool {
        ui.add(
            egui::DragValue::new(v)
                .speed(0.5)
                .suffix(format!(" {}", u.short())),
        )
        .changed()
    };
    for (i, g) in guides.iter().enumerate() {
        ui.horizontal(|ui| {
            let selected = app.selected_guide == Some(i);
            let label = match g {
                Guide::Horizontal { .. } => tr("options.horizontal"),
                Guide::Vertical { .. } => tr("options.vertical"),
                Guide::Angled { .. } => tr("options.angled"),
            };
            if ui.selectable_label(selected, label).clicked() {
                app.selected_guide = Some(i);
            }
            let mut ng = None;
            match *g {
                Guide::Horizontal { y } => {
                    let mut v = u.from_mm(y);
                    if unit_field(ui, &mut v) {
                        ng = Some(Guide::Horizontal { y: u.to_mm(v) });
                    }
                }
                Guide::Vertical { x } => {
                    let mut v = u.from_mm(x);
                    if unit_field(ui, &mut v) {
                        ng = Some(Guide::Vertical { x: u.to_mm(v) });
                    }
                }
                Guide::Angled { x, y, angle } => {
                    let (mut vx, mut vy, mut a) = (u.from_mm(x), u.from_mm(y), angle);
                    let cx = unit_field(ui, &mut vx);
                    let cy = unit_field(ui, &mut vy);
                    let ca = ui
                        .add(egui::DragValue::new(&mut a).speed(1.0).suffix("°"))
                        .changed();
                    if cx || cy || ca {
                        ng = Some(Guide::Angled {
                            x: u.to_mm(vx),
                            y: u.to_mm(vy),
                            angle: a,
                        });
                    }
                }
            }
            if let Some(guide) = ng {
                app.run(Command::MoveGuide {
                    page,
                    index: i,
                    guide,
                });
            }
            if ui.small_button("x").clicked() {
                app.run(Command::DeleteGuide { page, index: i });
            }
        });
    }
    let size = app.page_size();
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("options.add_horizontal")).clicked() {
            app.run(Command::AddGuide {
                page,
                guide: Guide::Horizontal {
                    y: size.height / 2.0,
                },
            });
        }
        if ui.button(tr("options.add_vertical")).clicked() {
            app.run(Command::AddGuide {
                page,
                guide: Guide::Vertical {
                    x: size.width / 2.0,
                },
            });
        }
        if ui.button(tr("options.add_angled")).clicked() {
            app.run(Command::AddGuide {
                page,
                guide: Guide::Angled {
                    x: size.width / 2.0,
                    y: size.height / 2.0,
                    angle: 45.0,
                },
            });
        }
        if ui.button(tr("docker.clear_all")).clicked() {
            for i in (0..guides.len()).rev() {
                app.run(Command::DeleteGuide { page, index: i });
            }
        }
    });
    ui.label(
        egui::RichText::new(tr("options.guide_presets"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("options.preset_margins")).clicked() {
            for g in [
                Guide::Horizontal { y: 10.0 },
                Guide::Horizontal {
                    y: size.height - 10.0,
                },
                Guide::Vertical { x: 10.0 },
                Guide::Vertical {
                    x: size.width - 10.0,
                },
            ] {
                app.run(Command::AddGuide { page, guide: g });
            }
        }
        if ui.button(tr("options.preset_columns")).clicked() {
            // Three equal columns between 10 mm margins with a 10 mm gutter.
            let inner = size.width - 20.0;
            let col = (inner - 20.0) / 3.0;
            let mut x = 10.0;
            for _ in 0..3 {
                app.run(Command::AddGuide {
                    page,
                    guide: Guide::Vertical { x },
                });
                app.run(Command::AddGuide {
                    page,
                    guide: Guide::Vertical { x: x + col },
                });
                x += col + 10.0;
            }
        }
        if ui.button(tr("options.preset_center")).clicked() {
            for g in [
                Guide::Horizontal {
                    y: size.height / 2.0,
                },
                Guide::Vertical {
                    x: size.width / 2.0,
                },
            ] {
                app.run(Command::AddGuide { page, guide: g });
            }
        }
    });
}

fn fonts(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(tr("dialog.filter"));
        ui.text_edit_singleline(&mut app.font_filter);
    });
    let filter = app.font_filter.to_lowercase();
    let used: std::collections::BTreeSet<String> = app
        .doc()
        .all_layers()
        .flat_map(|l| &l.shapes)
        .filter_map(|s| match &s.kind {
            ShapeKind::Text { spans, .. } => Some(
                spans
                    .iter()
                    .map(|sp| sp.font_family.clone())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect();
    if !used.is_empty() {
        ui.strong(tr("docker.fonts_in_document"));
        for f in &used {
            let missing = !app.font_families.contains(f);
            ui.horizontal(|ui| {
                if ui.selectable_label(app.text_font == *f, f).clicked() {
                    app.apply_text_font(f);
                }
                if missing {
                    ui.colored_label(egui::Color32::from_rgb(200, 100, 0), tr("docker.missing"));
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
                .on_hover_text(tr("docker.font_apply_hint"))
                .clicked()
            {
                app.apply_text_font(&f);
            }
        }
    });
}
