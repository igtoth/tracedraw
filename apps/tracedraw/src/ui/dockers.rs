//! Dockers: Properties, Objects (layer manager) and Hints.

use crate::app::{App, DockerTab};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::Ui;
use tracedraw_core::{
    document::ShapeKind, Arrowhead, Color, Command, Fill, Fountain, FountainKind, Pattern,
    PatternTile, Stop, Stroke, Texture, TextureKind,
};

/// The vertical strip of docker tabs on the right edge of the window.
/// Height of a docker's title bar (it runs across the tab column too).
pub const TITLE_H: f32 = 27.0;
/// The title bar, the empty part of the tab column.
pub const TITLE_FILL: egui::Color32 = egui::Color32::from_gray(0xEA);
/// Tabs, and the open docker's tab.
const TAB_FILL: egui::Color32 = egui::Color32::from_gray(0xD8);
const TAB_ACTIVE: egui::Color32 = egui::Color32::from_gray(0xB2);
const TAB_HOVER: egui::Color32 = egui::Color32::from_gray(0xC8);
/// The line between a docker and the tab column.
const TAB_LINE: egui::Color32 = egui::Color32::from_gray(0xB2);

/// The docker tab column: the open dockers as vertical tabs (an icon on
/// top, the name running downwards), grey, the open one darker, then the
/// button that adds a docker.
pub fn tab_strip(app: &mut App, ui: &mut Ui) {
    let full = ui.max_rect();
    // The title bar's part and the line along the docker.
    ui.painter().rect_filled(full, 0.0, TITLE_FILL);
    ui.painter().vline(
        full.left() + 0.5,
        (full.top() + TITLE_H)..=full.bottom(),
        egui::Stroke::new(1.0, TAB_LINE),
    );
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
    ui.add_space(TITLE_H);
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
        let galley = ui.painter().layout_no_wrap(
            name.clone(),
            egui::FontId::proportional(12.0),
            Tokens::TEXT,
        );
        let w = galley.size().x;
        let gh = galley.size().y;
        let icon_h = if tab_icon(tab).is_some() { 24.0 } else { 0.0 };
        let h = 14.0 + icon_h + w;
        let (r, resp) = ui.allocate_exact_size(egui::vec2(full.width(), h), egui::Sense::click());
        let bg = egui::Rect::from_min_max(egui::pos2(r.min.x + 1.0, r.min.y), r.max);
        let fill = if active {
            TAB_ACTIVE
        } else if resp.hovered() {
            TAB_HOVER
        } else {
            TAB_FILL
        };
        ui.painter().rect_filled(bg, 0.0, fill);
        if let Some(icon) = tab_icon(tab) {
            let ir = egui::Rect::from_center_size(
                egui::pos2(bg.center().x, r.min.y + 7.0 + icon_h / 2.0),
                egui::vec2(16.0, 16.0),
            );
            icon(ui.painter(), ir);
        }
        let text_top = r.min.y + 7.0 + icon_h;
        let mut ts = egui::epaint::TextShape::new(
            egui::pos2(bg.center().x + gh / 2.0, text_top),
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
        egui::Popup::context_menu(&resp)
            .id(egui::Id::new(("docker_tab_menu", tab.key())))
            .style(crate::ui::menus::menu_popup_style)
            .show(|ui| {
                crate::ui::menus::body(ui, |ui| {
                    if crate::ui::menus::item(ui, "docker.close_docker", "", true) {
                        app.open_dockers.retain(|t| *t != tab);
                        if app.docker_tab == tab {
                            app.show_dockers = false;
                        }
                    }
                })
            });
    }
    // Add a docker.
    ui.add_space(8.0);
    let (r, resp) = ui.allocate_exact_size(egui::vec2(full.width(), 22.0), egui::Sense::click());
    let c = r.center() + egui::vec2(0.5, 0.0);
    if resp.hovered() {
        ui.painter().rect_filled(
            egui::Rect::from_center_size(c, egui::vec2(18.0, 18.0)),
            0.0,
            TAB_HOVER,
        );
    }
    let s = egui::Stroke::new(1.5, egui::Color32::from_gray(0x8C));
    ui.painter()
        .line_segment([c + egui::vec2(-5.0, 0.0), c + egui::vec2(5.0, 0.0)], s);
    ui.painter()
        .line_segment([c + egui::vec2(0.0, -5.0), c + egui::vec2(0.0, 5.0)], s);
    let resp = resp.on_hover_text(tr("docker.add_docker"));
    egui::Popup::menu(&resp)
        .id(egui::Id::new("docker_add_menu"))
        .style(crate::ui::menus::menu_popup_style)
        .show(|ui| {
            crate::ui::menus::body(ui, |ui| {
                for tab in DockerTab::ALL {
                    if crate::ui::menus::item(ui, tab.key(), "", true) {
                        if !app.open_dockers.contains(&tab) {
                            app.open_dockers.push(tab);
                        }
                        app.show_dockers = true;
                        app.docker_tab = tab;
                    }
                }
            })
        });
}

type TabIcon = fn(&egui::Painter, egui::Rect);

/// The small picture above a docker tab's name.
fn tab_icon(tab: DockerTab) -> Option<TabIcon> {
    match tab {
        DockerTab::Hints => Some(|p, r| {
            // A pointer with a question mark.
            let s = egui::Stroke::new(1.2, Tokens::ICON);
            let o = r.min + egui::vec2(2.0, 1.0);
            let pts = vec![
                o,
                o + egui::vec2(0.0, 11.0),
                o + egui::vec2(3.0, 8.0),
                o + egui::vec2(5.5, 13.0),
                o + egui::vec2(7.0, 12.2),
                o + egui::vec2(4.6, 7.4),
                o + egui::vec2(8.5, 7.4),
            ];
            p.add(egui::epaint::PathShape::convex_polygon(
                pts,
                Tokens::ICON,
                s,
            ));
            p.text(
                r.min + egui::vec2(12.5, 3.5),
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(10.0),
                Tokens::ICON,
            );
        }),
        DockerTab::Properties => Some(|p, r| {
            // A pen over a ruled sheet.
            let s = egui::Stroke::new(1.2, Tokens::ICON);
            let c = r.center();
            p.line_segment([c + egui::vec2(-6.0, 6.0), c + egui::vec2(5.0, -5.0)], s);
            p.line_segment([c + egui::vec2(-6.0, 6.0), c + egui::vec2(-3.0, 5.0)], s);
            p.line_segment([c + egui::vec2(3.0, -7.0), c + egui::vec2(7.0, -3.0)], s);
            for y in [2.0, 5.0] {
                p.line_segment([c + egui::vec2(1.0, y), c + egui::vec2(7.0, y)], s);
            }
        }),
        DockerTab::Objects => Some(|p, r| {
            // Three stacked sheets.
            let c = r.center();
            for (k, dy) in [(0u8, 4.0f32), (1, 0.0), (2, -4.0)] {
                let pts = vec![
                    c + egui::vec2(-7.0, dy),
                    c + egui::vec2(0.0, dy - 3.5),
                    c + egui::vec2(7.0, dy),
                    c + egui::vec2(0.0, dy + 3.5),
                ];
                let fill = if k == 2 {
                    Tokens::ICON
                } else {
                    egui::Color32::WHITE
                };
                p.add(egui::epaint::PathShape::convex_polygon(
                    pts,
                    fill,
                    egui::Stroke::new(1.0, Tokens::ICON),
                ));
            }
        }),
        _ => None,
    }
}

/// The open docker: its title bar (name, collapse and close), then its
/// page on white.
pub fn dockers(app: &mut App, ui: &mut Ui) {
    let full = ui.max_rect();
    let (bar, _) = ui.allocate_exact_size(egui::vec2(full.width(), TITLE_H), egui::Sense::hover());
    ui.painter().rect_filled(bar, 0.0, TITLE_FILL);
    ui.painter().text(
        egui::pos2(bar.left() + 7.0, bar.center().y),
        egui::Align2::LEFT_CENTER,
        tr(app.docker_tab.key()),
        egui::FontId::proportional(12.0),
        Tokens::TEXT,
    );
    let grey = egui::Color32::from_gray(0x8C);
    // Close, at the right end.
    let close = egui::Rect::from_center_size(
        egui::pos2(bar.right() - 12.0, bar.center().y),
        egui::vec2(20.0, 20.0),
    );
    let resp = ui
        .interact(close, egui::Id::new("docker_close"), egui::Sense::click())
        .on_hover_text(tr("docker.close_docker"));
    let col = if resp.hovered() { Tokens::TEXT } else { grey };
    let c = close.center();
    let s = egui::Stroke::new(1.5, col);
    ui.painter()
        .line_segment([c + egui::vec2(-4.5, -4.5), c + egui::vec2(4.5, 4.5)], s);
    ui.painter()
        .line_segment([c + egui::vec2(4.5, -4.5), c + egui::vec2(-4.5, 4.5)], s);
    if resp.clicked() {
        app.show_dockers = false;
    }
    // Collapse: two small triangles.
    let fold = egui::Rect::from_center_size(
        egui::pos2(bar.right() - 36.0, bar.center().y),
        egui::vec2(20.0, 20.0),
    );
    let resp = ui
        .interact(fold, egui::Id::new("docker_fold"), egui::Sense::click())
        .on_hover_text(tr("docker.collapse"));
    let col = if resp.hovered() { Tokens::TEXT } else { grey };
    for dx in [-3.5f32, 3.0] {
        let c = fold.center() + egui::vec2(dx, 0.0);
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + egui::vec2(-2.5, -4.5),
                c + egui::vec2(2.5, 0.0),
                c + egui::vec2(-2.5, 4.5),
            ],
            col,
            egui::Stroke::NONE,
        ));
    }
    if resp.clicked() {
        app.show_dockers = false;
    }
    let content = egui::Rect::from_min_max(egui::pos2(full.left(), bar.bottom()), full.max);
    ui.painter().rect_filled(content, 0.0, egui::Color32::WHITE);
    ui.scope_builder(
        egui::UiBuilder::new().max_rect(content.shrink2(egui::vec2(8.0, 6.0))),
        |ui| {
            // Hints scrolls its page above its own navigation bar.
            if app.docker_tab == DockerTab::Hints {
                crate::ui::hints::hints_docker(app, ui);
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| match app.docker_tab {
                DockerTab::Properties => properties(app, ui),
                DockerTab::Objects => objects(app, ui),
                DockerTab::Hints => {}
                DockerTab::Transformations => transformations(app, ui),
                DockerTab::Undo => undo_docker(app, ui),
                other => crate::ui::dockers2::show(app, ui, other),
            });
        },
    );
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
        let custom = app.settings.custom_arrowheads.clone();
        if outline_editor(ui, &mut stroke, &custom) {
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
    let o = app.to_ruler(tracedraw_core::geometry::Point::new(b.x0, b.y0));
    ui.label(
        egui::RichText::new(format!(
            "x {:.2}  y {:.2}  w {:.2}  h {:.2} {}",
            app.units.from_mm(o.x),
            app.units.from_mm(o.y),
            app.units.from_mm(b.width()),
            app.units.from_mm(b.height()),
            app.units.short()
        ))
        .monospace()
        .size(11.0),
    );

    ui.separator();
    let open = app.properties_open.take();
    egui::CollapsingHeader::new(tr("docker.fill"))
        .open(open.map(|o| o == 0).filter(|o| *o))
        .show(ui, |ui| {
            let mut fill = first.fill.clone();
            if fill_editor(ui, &mut fill) {
                app.apply_fill(fill);
            }
        });
    egui::CollapsingHeader::new(tr("docker.outline"))
        .open(open.map(|o| o == 1).filter(|o| *o))
        .show(ui, |ui| {
            let mut stroke = first.stroke.clone();
            let custom = app.settings.custom_arrowheads.clone();
            if outline_editor(ui, &mut stroke, &custom) {
                let shapes = app.selection.clone();
                app.run(Command::SetStroke { shapes, stroke });
            }
        });
    if let Some(c) = first.page_corners() {
        ui.collapsing(tr("kind.rectangle"), |ui| rectangle_section(app, ui, c));
    }
    if matches!(first.kind, ShapeKind::Bitmap { .. }) {
        let id = first.id;
        egui::CollapsingHeader::new(tr("docker.fx"))
            .default_open(true)
            .show(ui, |ui| crate::ui::effect_dialog::fx_section(app, ui, id));
    }
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

/// The Properties docker's rectangle part: corner style, the four corner
/// sizes, editing them together and relative corner scaling, as on the
/// property bar.
fn rectangle_section(app: &mut App, ui: &mut Ui, c: tracedraw_core::Corners) {
    use crate::ui::propbar::{edit_corners, CornerEdit};
    use tracedraw_core::{CornerKind, Corners};
    let mut edit = None;
    ui.horizontal(|ui| {
        for (kind, key) in [
            (CornerKind::Round, "toolbar.corner_round"),
            (CornerKind::Scallop, "toolbar.corner_scallop"),
            (CornerKind::Chamfer, "toolbar.corner_chamfer"),
        ] {
            if ui.radio(c.kind == kind, tr(key)).clicked() {
                edit = Some(CornerEdit::Kind(kind));
            }
        }
    });
    let u = app.units;
    egui::Grid::new("docker_corners")
        .num_columns(4)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for pair in [
                [
                    (Corners::TOP_LEFT, "toolbar.corner_top_left"),
                    (Corners::TOP_RIGHT, "toolbar.corner_top_right"),
                ],
                [
                    (Corners::BOTTOM_LEFT, "toolbar.corner_bottom_left"),
                    (Corners::BOTTOM_RIGHT, "toolbar.corner_bottom_right"),
                ],
            ] {
                for (i, key) in pair {
                    ui.label(tr(key));
                    let mut v = u.from_mm(c.radii[i]);
                    let r = ui.add(
                        egui::DragValue::new(&mut v)
                            .range(0.0..=10_000.0)
                            .speed(0.1)
                            .suffix(format!(" {}", u.short())),
                    );
                    if (r.drag_stopped() || r.lost_focus() || (r.changed() && !r.dragged()))
                        && (u.to_mm(v) - c.radii[i]).abs() > 1e-12
                    {
                        edit = Some(CornerEdit::Size(i, u.to_mm(v)));
                    }
                }
                ui.end_row();
            }
        });
    ui.checkbox(&mut app.corners_together, tr("toolbar.corners_together"));
    let mut relative = !c.fixed;
    if ui
        .checkbox(&mut relative, tr("toolbar.relative_corners"))
        .changed()
    {
        edit = Some(CornerEdit::Scaling(relative));
    }
    if let Some(e) = edit {
        edit_corners(app, e);
    }
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
        Pattern::Vector { shapes, tile } => {
            ui.label(trf(
                "docker.vector_tile_n",
                &[("n", &shapes.len().to_string())],
            ));
            let (mut w, mut h) = (tile.width, tile.height);
            changed |= ui
                .add(
                    egui::Slider::new(&mut w, 1.0..=200.0)
                        .text(tr("docker.tile_width"))
                        .logarithmic(true),
                )
                .drag_stopped();
            changed |= ui
                .add(
                    egui::Slider::new(&mut h, 1.0..=200.0)
                        .text(tr("docker.tile_height"))
                        .logarithmic(true),
                )
                .drag_stopped();
            if (w - tile.width).abs() > 1e-9 || (h - tile.height).abs() > 1e-9 {
                // Scale the tile content with its size so the drawing keeps its shape.
                let sx = w / tile.width.max(1e-6);
                let sy = h / tile.height.max(1e-6);
                for s in shapes.iter_mut() {
                    s.transform =
                        tracedraw_core::geometry::Affine::scale_non_uniform(sx, sy) * s.transform;
                }
                tile.width = w;
                tile.height = h;
            }
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

fn arrow_combo(
    ui: &mut Ui,
    id: &str,
    label: &str,
    a: &mut Arrowhead,
    custom: &[Arrowhead],
) -> bool {
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
            for (i, k) in custom.iter().enumerate() {
                let name = if k.name().is_empty() {
                    trf("docker.custom_arrowhead_n", &[("n", &(i + 1).to_string())])
                } else {
                    k.name().to_string()
                };
                if ui.selectable_label(a == k, name).clicked() {
                    *a = k.clone();
                    changed = true;
                }
            }
        });
    changed
}

fn outline_editor(ui: &mut Ui, stroke: &mut Option<Stroke>, custom: &[Arrowhead]) -> bool {
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
            changed |= arrow_combo(
                ui,
                "Start",
                &tr("docker.arrow_start"),
                &mut s.start_arrow,
                custom,
            );
            changed |= arrow_combo(ui, "End", &tr("docker.arrow_end"), &mut s.end_arrow, custom);
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
        ShapeKind::Polygon {
            complex: Some(_), ..
        } => "ComplexStar",
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
            sharpness,
            points,
            complex,
            ..
        } => {
            let n = points.to_string();
            if complex.is_some() {
                trf("kind.complex_star_n", &[("n", &n)])
            } else if *sharpness > 0.0 {
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
