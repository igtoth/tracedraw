//! Dockers: Properties, Objects (layer manager) and Hints.

use crate::app::{App, DockerTab};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::tools::Tool;
use egui::Ui;
use tracedraw_core::{
    document::ShapeKind, Arrowhead, Color, Command, Fill, Fountain, FountainKind, Pattern,
    PatternTile, Stop, Stroke, Texture, TextureKind,
};

/// The vertical strip of docker tabs on the right edge of the window.
pub fn tab_strip(app: &mut App, ui: &mut Ui) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
    ui.add_space(4.0);
    let mut tabs = DockerTab::default_strip();
    for t in &app.open_dockers {
        if !tabs.contains(t) {
            tabs.push(*t);
        }
    }
    if !tabs.contains(&app.docker_tab) {
        tabs.push(app.docker_tab);
    }
    for tab in tabs {
        let name = tr(tab.key());
        let active = app.show_dockers && app.docker_tab == tab;
        let h = 14.0 + name.chars().count() as f32 * 7.0;
        let (r, resp) = ui.allocate_exact_size(egui::vec2(26.0, h), egui::Sense::click());
        if active {
            ui.painter().rect_filled(r, 2.0, Tokens::TOOL_ACTIVE);
        } else if resp.hovered() {
            ui.painter().rect_filled(r, 2.0, Tokens::TOOL_HOVER);
        }
        let galley = ui.painter().layout_no_wrap(
            name.clone(),
            egui::FontId::proportional(11.0),
            Tokens::TEXT,
        );
        let w = galley.size().x;
        let gh = galley.size().y;
        let mut ts = egui::epaint::TextShape::new(
            egui::pos2(r.center().x + gh / 2.0 - 1.0, r.center().y - w / 2.0),
            galley,
            Tokens::TEXT,
        );
        ts.angle = std::f32::consts::FRAC_PI_2;
        ui.painter().add(ts);
        if resp.clicked() {
            if active {
                app.show_dockers = false;
            } else {
                app.show_dockers = true;
                app.docker_tab = tab;
            }
        }
        resp.context_menu(|ui| {
            if ui.button(tr("docker.close_docker")).clicked() {
                app.open_dockers.retain(|t| *t != tab);
                if app.docker_tab == tab {
                    app.show_dockers = false;
                }
                ui.close();
            }
        });
    }
    ui.menu_button(
        egui::RichText::new("+").size(14.0).color(Tokens::TEXT_DIM),
        |ui| {
            for tab in DockerTab::ALL {
                if ui.button(tr(tab.key())).clicked() {
                    if !app.open_dockers.contains(&tab) {
                        app.open_dockers.push(tab);
                    }
                    app.show_dockers = true;
                    app.docker_tab = tab;
                    ui.close();
                }
            }
        },
    );
}

pub fn dockers(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        let name = tr(app.docker_tab.key());
        ui.label(egui::RichText::new(name).size(12.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new("✕").frame(false))
                .on_hover_text(tr("docker.close_docker"))
                .clicked()
            {
                app.show_dockers = false;
            }
            let _ = ui
                .add(egui::Button::new("»").frame(false))
                .on_hover_text(tr("docker.collapse"));
        });
    });
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| match app.docker_tab {
        DockerTab::Properties => properties(app, ui),
        DockerTab::Objects => objects(app, ui),
        DockerTab::Hints => hints(app, ui),
        DockerTab::Transformations => transformations(app, ui),
        DockerTab::Undo => undo_docker(app, ui),
        other => crate::ui::dockers2::show(app, ui, other),
    });
}

fn color_row(ui: &mut Ui, label: &str, c: &mut Color) -> bool {
    let [r, g, b] = c.to_rgb8();
    let mut rgb = [r, g, b];
    let changed = ui
        .horizontal(|ui| {
            ui.label(label);
            let ch = ui.color_edit_button_srgb(&mut rgb).changed();
            ui.label(
                egui::RichText::new(crate::app::color_description(*c))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
            ch
        })
        .inner;
    if changed {
        *c = Color::rgb8(rgb[0], rgb[1], rgb[2]);
    }
    changed
}

fn properties(app: &mut App, ui: &mut Ui) {
    let shapes = app.selected_shapes();
    if shapes.is_empty() {
        ui.label(egui::RichText::new(tr("docker.no_objects_selected")).color(Tokens::TEXT_DIM));
        ui.add_space(6.0);
        ui.strong(tr("docker.default_properties"));
        let mut fill = app.default_fill.clone();
        if fill_editor(ui, &mut fill) {
            app.default_fill = fill;
        }
        ui.separator();
        let mut stroke = app.default_stroke.clone();
        if outline_editor(ui, &mut stroke) {
            app.default_stroke = stroke;
        }
        return;
    }
    let first = &shapes[0];
    ui.strong(format!(
        "{}{}",
        kind_name(&first.kind),
        if shapes.len() > 1 {
            trf(
                "docker.and_n_more",
                &[("n", &(shapes.len() - 1).to_string())],
            )
        } else {
            String::new()
        }
    ));
    let b = app.selection_bounds().unwrap_or_default();
    ui.label(
        egui::RichText::new(format!(
            "x {:.2}  y {:.2}  w {:.2}  h {:.2} {}",
            app.units.from_mm(b.x0),
            app.units.from_mm(b.y0),
            app.units.from_mm(b.width()),
            app.units.from_mm(b.height()),
            app.units.short()
        ))
        .monospace()
        .size(11.0),
    );

    ui.separator();
    ui.collapsing(tr("docker.fill"), |ui| {
        let mut fill = first.fill.clone();
        if fill_editor(ui, &mut fill) {
            app.apply_fill(fill);
        }
    });
    ui.collapsing(tr("docker.outline"), |ui| {
        let mut stroke = first.stroke.clone();
        if outline_editor(ui, &mut stroke) {
            let shapes = app.selection.clone();
            app.run(Command::SetStroke { shapes, stroke });
        }
    });
    if let ShapeKind::Text { spans, .. } = &first.kind {
        ui.collapsing(tr("docker.character"), |ui| {
            if let Some(sp) = spans.first() {
                ui.label(format!(
                    "{} {} pt{}{}",
                    sp.font_family,
                    sp.size_pt,
                    if sp.bold {
                        format!(" {}", tr("docker.bold"))
                    } else {
                        String::new()
                    },
                    if sp.italic {
                        format!(" {}", tr("docker.italic"))
                    } else {
                        String::new()
                    }
                ));
            }
            ui.label(
                egui::RichText::new(tr("docker.edit_with_text_tool"))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
        });
    }
    ui.collapsing(tr("docker.summary"), |ui| {
        for s in &shapes {
            ui.label(format!(
                "{}  id {}  {}",
                kind_name(&s.kind),
                s.id.raw(),
                s.name.clone().unwrap_or_default()
            ));
        }
    });
}

fn fill_editor(ui: &mut Ui, fill: &mut Fill) -> bool {
    let mut changed = false;
    let mut kind = match fill {
        Fill::None => 0,
        Fill::Solid(_) => 1,
        Fill::Fountain(_) => 2,
        Fill::Pattern(_) => 3,
        Fill::Texture(_) => 4,
        Fill::Mesh(_) => 5,
    };
    ui.horizontal_wrapped(|ui| {
        for (i, n) in [
            "docker.fill_none",
            "docker.fill_uniform",
            "docker.fill_fountain",
            "docker.fill_pattern",
            "docker.fill_texture",
            "docker.fill_mesh",
        ]
        .iter()
        .enumerate()
        {
            if ui.selectable_label(kind == i, tr(n)).clicked() && kind != i {
                kind = i;
                changed = true;
            }
        }
    });
    if changed {
        let base = fill
            .preview_color()
            .unwrap_or(Color::cmyk_pct(0.0, 0.0, 0.0, 20.0));
        *fill = match kind {
            0 => Fill::None,
            1 => Fill::Solid(base),
            2 => Fill::linear(base, Color::WHITE, 0.0),
            3 => Fill::Pattern(Pattern::TwoColor {
                tile: PatternTile::Checker,
                front: base,
                back: Color::WHITE,
                size_mm: 10.0,
            }),
            4 => Fill::Texture(Texture {
                kind: TextureKind::Clouds,
                color_a: base,
                color_b: Color::WHITE,
                scale: 20.0,
                seed: 1,
            }),
            _ => Fill::Mesh(tracedraw_core::Mesh::new(
                tracedraw_core::geometry::Rect::new(0.0, 0.0, 100.0, 100.0),
                2,
                2,
                base,
            )),
        };
    }
    match fill {
        Fill::None => {}
        Fill::Solid(c) => changed |= color_row(ui, &tr("docker.colour"), c),
        Fill::Fountain(f) => changed |= fountain_editor(ui, f),
        Fill::Pattern(p) => changed |= pattern_editor(ui, p),
        Fill::Texture(t) => changed |= texture_editor(ui, t),
        Fill::Mesh(m) => {
            ui.label(trf(
                "docker.mesh_n",
                &[("r", &m.rows.to_string()), ("c", &m.cols.to_string())],
            ));
            ui.horizontal(|ui| {
                ui.label(tr("docker.grid"));
                let (mut r, mut c) = (m.rows, m.cols);
                if ui.add(egui::DragValue::new(&mut r).range(1..=50)).changed()
                    || ui.add(egui::DragValue::new(&mut c).range(1..=50)).changed()
                {
                    let b = m
                        .nodes
                        .iter()
                        .fold(None::<tracedraw_core::geometry::Rect>, |acc, n| {
                            Some(match acc {
                                None => tracedraw_core::geometry::Rect::from_points(n.pos, n.pos),
                                Some(r) => r.union_pt(n.pos),
                            })
                        })
                        .unwrap_or_default();
                    let base = m.nodes.first().map(|n| n.color).unwrap_or(Color::WHITE);
                    *m = tracedraw_core::Mesh::new(b, r, c, base);
                    changed = true;
                }
            });
            changed |= ui
                .checkbox(&mut m.smooth, tr("docker.smooth_colour"))
                .changed();
            ui.label(
                egui::RichText::new(tr("docker.mesh_hint"))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
        }
    }
    changed
}

fn fountain_editor(ui: &mut Ui, f: &mut Fountain) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(tr("docker.type"));
        for (k, n) in [
            (FountainKind::Linear, "fill.linear"),
            (FountainKind::Radial, "fill.radial"),
            (FountainKind::Conical, "fill.conical"),
            (FountainKind::Square, "fill.square"),
        ] {
            if ui.selectable_label(f.kind == k, tr(n)).clicked() && f.kind != k {
                f.kind = k;
                changed = true;
            }
        }
    });
    // Stops: position and colour, add after, remove (keep at least two).
    let mut remove = None;
    let mut insert = None;
    let n = f.stops.len();
    for i in 0..n {
        let stop = &mut f.stops[i];
        ui.horizontal(|ui| {
            let [r, g, b] = stop.color.to_rgb8();
            let mut rgb = [r, g, b];
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                stop.color = Color::rgb8(rgb[0], rgb[1], rgb[2]);
                changed = true;
            }
            let mut pct = stop.pos * 100.0;
            if ui
                .add(
                    egui::DragValue::new(&mut pct)
                        .range(0.0..=100.0)
                        .suffix("%")
                        .speed(1.0),
                )
                .changed()
            {
                stop.pos = pct / 100.0;
                changed = true;
            }
            if ui
                .small_button("+")
                .on_hover_text(tr("docker.add_stop"))
                .clicked()
            {
                insert = Some(i);
            }
            if n > 2
                && ui
                    .small_button("x")
                    .on_hover_text(tr("docker.remove_stop"))
                    .clicked()
            {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = insert {
        let next = f.stops.get(i + 1).map(|s| s.pos).unwrap_or(1.0);
        let pos = (f.stops[i].pos + next) / 2.0;
        let color = f.color_at(pos);
        f.stops.insert(i + 1, Stop { pos, color });
        changed = true;
    }
    if let Some(i) = remove {
        f.stops.remove(i);
        changed = true;
    }
    if matches!(
        f.kind,
        FountainKind::Linear | FountainKind::Conical | FountainKind::Square
    ) {
        changed |= ui
            .add(
                egui::Slider::new(&mut f.angle, -180.0..=180.0)
                    .text(tr("docker.angle_lc"))
                    .suffix("°"),
            )
            .drag_stopped();
    }
    if !matches!(f.kind, FountainKind::Linear) {
        ui.horizontal(|ui| {
            ui.label(tr("docker.centre_offset"));
            changed |= ui
                .add(
                    egui::DragValue::new(&mut f.offset.x)
                        .range(-1.0..=1.0)
                        .speed(0.01),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut f.offset.y)
                        .range(-1.0..=1.0)
                        .speed(0.01),
                )
                .changed();
        });
    }
    let mut pad = f.edge_pad * 100.0;
    if ui
        .add(
            egui::Slider::new(&mut pad, 0.0..=49.0)
                .text(tr("docker.edge_pad"))
                .suffix("%"),
        )
        .drag_stopped()
    {
        f.edge_pad = pad / 100.0;
        changed = true;
    }
    changed
}

fn pattern_editor(ui: &mut Ui, p: &mut Pattern) -> bool {
    let mut changed = false;
    match p {
        Pattern::TwoColor {
            tile,
            front,
            back,
            size_mm,
        } => {
            egui::ComboBox::from_label(tr("docker.tile"))
                .selected_text(tile.name())
                .show_ui(ui, |ui| {
                    for t in PatternTile::ALL {
                        if ui.selectable_label(*tile == t, t.name()).clicked() {
                            *tile = t;
                            changed = true;
                        }
                    }
                });
            changed |= color_row(ui, &tr("docker.front"), front);
            changed |= color_row(ui, &tr("docker.back"), back);
            changed |= ui
                .add(
                    egui::Slider::new(size_mm, 1.0..=100.0)
                        .text(tr("docker.tile_size_mm"))
                        .logarithmic(true),
                )
                .drag_stopped();
        }
        Pattern::Bitmap {
            width_px,
            height_px,
            size_mm,
            ..
        } => {
            ui.label(trf(
                "docker.bitmap_tile_n",
                &[("w", &width_px.to_string()), ("h", &height_px.to_string())],
            ));
            changed |= ui
                .add(
                    egui::Slider::new(size_mm, 1.0..=200.0)
                        .text(tr("docker.tile_size_mm"))
                        .logarithmic(true),
                )
                .drag_stopped();
        }
    }
    changed
}

fn texture_editor(ui: &mut Ui, t: &mut Texture) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (k, n) in [
            (TextureKind::Clouds, "texture.clouds"),
            (TextureKind::Marble, "texture.marble"),
            (TextureKind::Noise, "texture.noise"),
            (TextureKind::Wood, "texture.wood"),
        ] {
            if ui.selectable_label(t.kind == k, tr(n)).clicked() && t.kind != k {
                t.kind = k;
                changed = true;
            }
        }
    });
    changed |= color_row(ui, &tr("docker.colour_a"), &mut t.color_a);
    changed |= color_row(ui, &tr("docker.colour_b"), &mut t.color_b);
    changed |= ui
        .add(
            egui::Slider::new(&mut t.scale, 1.0..=200.0)
                .text(tr("docker.scale_mm"))
                .logarithmic(true),
        )
        .drag_stopped();
    ui.horizontal(|ui| {
        ui.label(tr("docker.seed"));
        changed |= ui.add(egui::DragValue::new(&mut t.seed)).changed();
        if ui.small_button(tr("docker.regenerate")).clicked() {
            t.seed = t.seed.wrapping_add(1);
            changed = true;
        }
    });
    changed
}

fn arrow_combo(ui: &mut Ui, id: &str, label: &str, a: &mut Arrowhead) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(format!("{label}: {}", a.name()))
        .show_ui(ui, |ui| {
            for k in Arrowhead::ALL {
                if ui.selectable_label(*a == k, k.name()).clicked() {
                    *a = k;
                    changed = true;
                }
            }
        });
    changed
}

fn outline_editor(ui: &mut Ui, stroke: &mut Option<Stroke>) -> bool {
    let mut changed = false;
    let mut has = stroke.is_some();
    if ui.checkbox(&mut has, tr("docker.outline")).changed() {
        *stroke = if has { Some(Stroke::default()) } else { None };
        changed = true;
    }
    if let Some(s) = stroke {
        changed |= color_row(ui, &tr("docker.colour"), &mut s.color);
        let mut hair = s.width <= Stroke::HAIRLINE + 1e-9;
        if ui.checkbox(&mut hair, tr("docker.hairline")).changed() {
            s.width = if hair { Stroke::HAIRLINE } else { 0.5 };
            changed = true;
        }
        if !hair {
            changed |= ui
                .add(
                    egui::Slider::new(&mut s.width, 0.1..=25.0)
                        .text(tr("docker.width_mm"))
                        .logarithmic(true),
                )
                .drag_stopped();
        }
        ui.horizontal(|ui| {
            ui.label(tr("docker.caps"));
            for (c, n) in [
                (tracedraw_core::LineCap::Butt, "docker.cap_butt"),
                (tracedraw_core::LineCap::Round, "docker.cap_round"),
                (tracedraw_core::LineCap::Square, "docker.cap_square"),
            ] {
                if ui.selectable_label(s.cap == c, tr(n)).clicked() {
                    s.cap = c;
                    changed = true;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("docker.corners"));
            for (j, n) in [
                (tracedraw_core::LineJoin::Miter, "docker.join_miter"),
                (tracedraw_core::LineJoin::Round, "docker.join_round"),
                (tracedraw_core::LineJoin::Bevel, "docker.join_bevel"),
            ] {
                if ui.selectable_label(s.join == j, tr(n)).clicked() {
                    s.join = j;
                    changed = true;
                }
            }
        });
        ui.horizontal(|ui| {
            changed |= arrow_combo(ui, "Start", &tr("docker.arrow_start"), &mut s.start_arrow);
            changed |= arrow_combo(ui, "End", &tr("docker.arrow_end"), &mut s.end_arrow);
        });
        ui.horizontal(|ui| {
            ui.label(tr("docker.dash"));
            for (d, n) in [
                (vec![], "docker.dash_solid"),
                (vec![4.0, 2.0], "docker.dash_dashed"),
                (vec![1.0, 1.0], "docker.dash_dotted"),
                (vec![6.0, 2.0, 1.0, 2.0], "docker.dash_dash_dot"),
            ] {
                if ui.selectable_label(s.dash == d, tr(n)).clicked() {
                    s.dash = d;
                    changed = true;
                }
            }
        });
        changed |= ui
            .add(egui::Slider::new(&mut s.stretch, 0.1..=1.0).text(tr("docker.nib_stretch")))
            .drag_stopped();
        changed |= ui
            .add(
                egui::Slider::new(&mut s.nib_angle, -90.0..=90.0)
                    .text(tr("docker.nib_angle_lc"))
                    .suffix("°"),
            )
            .drag_stopped();
        changed |= ui
            .checkbox(&mut s.behind_fill, tr("docker.behind_fill"))
            .changed();
        changed |= ui
            .checkbox(&mut s.scale_with_object, tr("docker.scale_with_object"))
            .changed();
    }
    changed
}

/// Stable, language-independent object type for scripts (`Shape.Type`).
pub fn kind_id(k: &ShapeKind) -> &'static str {
    match k {
        ShapeKind::Rect { .. } => "Rectangle",
        ShapeKind::Ellipse { .. } => "Ellipse",
        ShapeKind::Polygon { sharpness, .. } if *sharpness > 0.0 => "Star",
        ShapeKind::Polygon { .. } => "Polygon",
        ShapeKind::Path { .. } => "Curve",
        ShapeKind::Text { .. } => "Text",
        ShapeKind::Group { .. } => "Group",
        ShapeKind::Bitmap { .. } => "Bitmap",
        ShapeKind::ClipFrame { .. } => "ClipFrame",
        ShapeKind::Table(_) => "Table",
        ShapeKind::SymbolInstance { .. } => "Symbol",
    }
}

pub fn kind_name(k: &ShapeKind) -> String {
    match k {
        ShapeKind::Rect { .. } => tr("kind.rectangle"),
        ShapeKind::Ellipse { .. } => tr("kind.ellipse"),
        ShapeKind::Polygon {
            sharpness, points, ..
        } => {
            let n = points.to_string();
            if *sharpness > 0.0 {
                trf("kind.star_n", &[("n", &n)])
            } else {
                trf("kind.polygon_n", &[("n", &n)])
            }
        }
        ShapeKind::Path { path, .. } => {
            let n = path
                .elements()
                .iter()
                .filter(|e| !matches!(e, tracedraw_core::geometry::PathEl::ClosePath))
                .count();
            trf("kind.curve_n", &[("n", &n.to_string())])
        }
        ShapeKind::Text { spans, .. } => {
            let t: String = spans
                .iter()
                .map(|s| s.text.as_str())
                .collect::<String>()
                .chars()
                .take(20)
                .collect();
            trf("kind.artistic_text", &[("t", &t)])
        }
        ShapeKind::Group { children } => trf("kind.group_n", &[("n", &children.len().to_string())]),
        ShapeKind::Bitmap {
            width_px,
            height_px,
            ..
        } => trf(
            "kind.bitmap_n",
            &[("w", &width_px.to_string()), ("h", &height_px.to_string())],
        ),
        ShapeKind::ClipFrame { contents, .. } => {
            trf("kind.clip_frame_n", &[("n", &contents.len().to_string())])
        }
        ShapeKind::Table(t) => trf(
            "kind.table_n",
            &[("r", &t.rows().to_string()), ("c", &t.cols().to_string())],
        ),
        ShapeKind::SymbolInstance { index } => {
            trf("kind.symbol_n", &[("n", &(index + 1).to_string())])
        }
    }
}

fn objects(app: &mut App, ui: &mut Ui) {
    let doc = app.doc();
    let Ok(page) = doc.page(app.page) else { return };
    let mut toggles: Vec<(tracedraw_core::LayerId, bool, bool)> = Vec::new();
    let mut click: Option<tracedraw_core::ShapeId> = None;
    let mut add_layer = false;
    ui.horizontal(|ui| {
        ui.strong(&page.name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button(tr("docker.add_layer")).clicked() {
                add_layer = true;
            }
        });
    });
    let mut rename: Option<(tracedraw_core::LayerId, String)> = None;
    let mut delete: Option<tracedraw_core::LayerId> = None;
    let mut reorder: Option<(tracedraw_core::LayerId, usize)> = None;
    let nlayers = page.layers.len();
    for (li, layer) in page.layers.iter().enumerate().rev() {
        let mut vis = layer.visible;
        let mut locked = layer.locked;
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut vis, "")
                .on_hover_text(tr("docker.show_hide"))
                .changed()
            {
                toggles.push((layer.id, vis, locked));
            }
            if ui
                .selectable_label(locked, "L")
                .on_hover_text(tr("docker.lock_layer"))
                .clicked()
            {
                locked = !locked;
                toggles.push((layer.id, vis, locked));
            }
            let r = ui.strong(&layer.name);
            if r.double_clicked() {
                rename = Some((layer.id, layer.name.clone()));
            }
            r.context_menu(|ui| {
                if ui.button(tr("docker.rename")).clicked() {
                    rename = Some((layer.id, layer.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(li + 1 < nlayers, egui::Button::new(tr("docker.move_up")))
                    .clicked()
                {
                    reorder = Some((layer.id, li + 1));
                    ui.close();
                }
                if ui
                    .add_enabled(li > 0, egui::Button::new(tr("docker.move_down")))
                    .clicked()
                {
                    reorder = Some((layer.id, li - 1));
                    ui.close();
                }
                if ui
                    .add_enabled(nlayers > 1, egui::Button::new(tr("docker.delete_layer")))
                    .clicked()
                {
                    delete = Some(layer.id);
                    ui.close();
                }
            });
        });
        for s in layer.shapes.iter().rev() {
            let sel = app.selection.contains(&s.id);
            let fill = crate::app::fill_preview_color(&s.fill);
            ui.horizontal(|ui| {
                ui.add_space(18.0);
                let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                match fill {
                    Some(c) => {
                        ui.painter().rect_filled(r, 1.0, c);
                    }
                    None => {
                        ui.painter().rect_stroke(
                            r,
                            1.0,
                            egui::Stroke::new(1.0, Tokens::TEXT_DIM),
                            egui::epaint::StrokeKind::Inside,
                        );
                    }
                }
                if ui.selectable_label(sel, kind_name(&s.kind)).clicked() {
                    click = Some(s.id);
                }
            });
        }
    }
    for (layer, visible, locked) in toggles {
        app.run(Command::SetLayerVisible { layer, visible });
        app.run(Command::SetLayerLocked { layer, locked });
    }
    if let Some((layer, name)) = rename {
        app.dialog = crate::ui::dialogs::Dialog::RenameLayer { layer, name };
    }
    if let Some(layer) = delete {
        app.selection.clear();
        app.run(Command::DeleteLayer { layer });
    }
    if let Some((layer, index)) = reorder {
        app.run(Command::ReorderLayer { layer, index });
    }
    if add_layer {
        let page = app.page;
        let n = app
            .doc()
            .page(page)
            .map(|p| p.layers.len() + 1)
            .unwrap_or(1);
        app.run(Command::AddLayer {
            page,
            name: trf("docker.layer_n", &[("n", &n.to_string())]),
        });
    }
    if let Some(id) = click {
        if ui.input(|i| i.modifiers.shift) {
            if !app.selection.contains(&id) {
                app.selection.push(id);
            }
        } else {
            app.select(vec![id]);
        }
    }
}

fn hints(app: &mut App, ui: &mut Ui) {
    if app.tool == Tool::Pick && app.selection.is_empty() {
        ui.heading(tr("hint.home"));
        ui.add_space(4.0);
        ui.label(tr("hint.home_intro"));
        ui.add_space(4.0);
        for t in [
            "hint.topic_lines",
            "hint.topic_connector_lines",
            "hint.topic_dimension_lines",
            "hint.topic_shapes",
            "hint.topic_select",
            "hint.topic_move_scale",
            "hint.topic_rotate_skew",
            "hint.topic_shape_objects",
            "hint.topic_effects",
            "hint.topic_outline",
            "hint.topic_fill",
            "hint.topic_text",
            "hint.topic_help",
        ] {
            ui.horizontal(|ui| {
                ui.label("•");
                let _ = ui.link(egui::RichText::new(tr(t)).color(Tokens::ACCENT));
            });
        }
        ui.add_space(12.0);
        ui.separator();
        ui.heading(tr("hint.learn_more"));
        ui.label(egui::RichText::new(tr("hint.help_topic")).strong());
        let _ = ui.link(egui::RichText::new(tr("hint.app_help")).color(Tokens::ACCENT));
        return;
    }
    ui.heading(app.tool.name());
    ui.add_space(4.0);
    let key = match app.tool {
        Tool::Pick => "hint.pick",
        Tool::Shape => "hint.shape",
        Tool::Zoom => "hint.zoom",
        Tool::Pan => "hint.pan",
        Tool::Freehand => "hint.freehand",
        Tool::Bezier | Tool::Pen => "hint.bezier",
        Tool::Polyline | Tool::TwoPointLine => "hint.polyline",
        Tool::Rectangle => "hint.rectangle",
        Tool::Ellipse => "hint.ellipse",
        Tool::Polygon | Tool::Star => "hint.polygon",
        Tool::Text => "hint.text",
        Tool::InteractiveFill => "hint.interactive_fill",
        Tool::ColorEyedropper => "hint.eyedropper",
        Tool::Eraser => "hint.eraser",
        _ => "hint.roadmap",
    };
    ui.label(tr(key));
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(tr("hint.palette_footer"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
}

fn transformations(app: &mut App, ui: &mut Ui) {
    use crate::app::TransformTab;
    ui.horizontal(|ui| {
        for (tab, name) in [
            (TransformTab::Position, "docker.position"),
            (TransformTab::Rotate, "docker.rotate"),
            (TransformTab::Scale, "docker.scale"),
            (TransformTab::Size, "docker.size"),
            (TransformTab::Skew, "docker.skew"),
        ] {
            if ui
                .selectable_label(app.transform_tab == tab, tr(name))
                .clicked()
            {
                app.transform_tab = tab;
                app.transform_values = match tab {
                    TransformTab::Scale => [100.0, 100.0, 0.0, 0.0],
                    TransformTab::Size => {
                        let b = app.selection_bounds().unwrap_or_default();
                        [b.width(), b.height(), 0.0, 0.0]
                    }
                    _ => [0.0; 4],
                };
            }
        }
    });
    ui.separator();
    let has = !app.selection.is_empty();
    let u = app.units.short();
    let mut v = app.transform_values;
    let mut relative = true;
    match app.transform_tab {
        TransformTab::Position => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.x"));
                ui.add(
                    egui::DragValue::new(&mut v[0])
                        .speed(0.5)
                        .suffix(format!(" {u}")),
                );
                ui.label(tr("docker.y"));
                ui.add(
                    egui::DragValue::new(&mut v[1])
                        .speed(0.5)
                        .suffix(format!(" {u}")),
                );
            });
            ui.checkbox(&mut relative, tr("docker.relative_position"));
        }
        TransformTab::Rotate => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.angle_colon"));
                ui.add(egui::DragValue::new(&mut v[0]).speed(1.0).suffix("°"));
            });
            ui.label(
                egui::RichText::new(tr("docker.rotates_about_centre"))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
        }
        TransformTab::Scale => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.x"));
                ui.add(egui::DragValue::new(&mut v[0]).speed(1.0).suffix(" %"));
                ui.label(tr("docker.y"));
                ui.add(egui::DragValue::new(&mut v[1]).speed(1.0).suffix(" %"));
            });
            ui.horizontal(|ui| {
                if ui.button(tr("toolbar.mirror_h")).clicked() && has {
                    app.mirror(true);
                }
                if ui.button(tr("toolbar.mirror_v")).clicked() && has {
                    app.mirror(false);
                }
            });
        }
        TransformTab::Size => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.w"));
                ui.add(
                    egui::DragValue::new(&mut v[0])
                        .speed(0.5)
                        .suffix(format!(" {u}")),
                );
                ui.label(tr("docker.h"));
                ui.add(
                    egui::DragValue::new(&mut v[1])
                        .speed(0.5)
                        .suffix(format!(" {u}")),
                );
            });
        }
        TransformTab::Skew => {
            ui.horizontal(|ui| {
                ui.label(tr("docker.x"));
                ui.add(egui::DragValue::new(&mut v[0]).speed(1.0).suffix("°"));
                ui.label(tr("docker.y"));
                ui.add(egui::DragValue::new(&mut v[1]).speed(1.0).suffix("°"));
            });
        }
    }
    app.transform_values = v;
    ui.add_space(6.0);
    let apply = |app: &mut App, dup: bool| {
        let t = match app.transform_tab {
            TransformTab::Position => {
                let d = tracedraw_core::geometry::Vec2::new(
                    app.units.to_mm(v[0]),
                    app.units.to_mm(v[1]),
                );
                if relative {
                    tracedraw_core::geometry::Affine::translate(d)
                } else {
                    let b = app.selection_bounds().unwrap_or_default();
                    tracedraw_core::geometry::Affine::translate(d - b.center().to_vec2())
                }
            }
            TransformTab::Rotate => tracedraw_core::geometry::Affine::rotate(v[0].to_radians()),
            TransformTab::Scale => {
                tracedraw_core::geometry::Affine::scale_non_uniform(v[0] / 100.0, v[1] / 100.0)
            }
            TransformTab::Size => {
                let b = app.selection_bounds().unwrap_or_default();
                tracedraw_core::geometry::Affine::scale_non_uniform(
                    app.units.to_mm(v[0]) / b.width().max(1e-9),
                    app.units.to_mm(v[1]) / b.height().max(1e-9),
                )
            }
            TransformTab::Skew => tracedraw_core::geometry::Affine::skew(
                v[0].to_radians().tan(),
                v[1].to_radians().tan(),
            ),
        };
        if matches!(app.transform_tab, TransformTab::Position) {
            if dup {
                let saved = app.duplicate_offset;
                app.duplicate_offset = tracedraw_core::geometry::Vec2::ZERO;
                app.duplicate();
                app.duplicate_offset = saved;
            }
            app.transform_selection(t);
        } else {
            app.transform_about_center(t, dup);
        }
    };
    ui.horizontal(|ui| {
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.apply")))
            .clicked()
        {
            apply(app, false);
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.apply_to_duplicate")))
            .clicked()
        {
            apply(app, true);
        }
    });
}

fn undo_docker(app: &mut App, ui: &mut Ui) {
    let (undo, redo) = app.engine.history_labels();
    ui.label(
        egui::RichText::new(tr("docker.undo_hint"))
            .color(Tokens::TEXT_DIM)
            .size(11.0),
    );
    ui.add_space(4.0);
    let n = undo.len();
    let mut goto: Option<usize> = None;
    for (i, l) in undo.iter().enumerate() {
        let is_last = i + 1 == n;
        if ui.selectable_label(is_last, l.to_string()).clicked() && !is_last {
            goto = Some(n - 1 - i);
        }
    }
    if let Some(steps) = goto {
        for _ in 0..steps {
            app.undo();
        }
    }
    for l in redo.iter().rev() {
        let _ = ui.selectable_label(
            false,
            egui::RichText::new(l.to_string()).color(Tokens::TEXT_DIM),
        );
    }
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(n > 0, egui::Button::new(tr("docker.undo")))
            .clicked()
        {
            app.undo();
        }
        if ui
            .add_enabled(!redo.is_empty(), egui::Button::new(tr("docker.redo")))
            .clicked()
        {
            app.redo();
        }
    });
}
