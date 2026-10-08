//! Application state and UI layout. Every document mutation goes through
//! `Engine::run` with a `Command`, so the UI stays thin and replayable.

use crate::canvas::{self, RenderOptions};
use crate::view::View;
use egui::{Color32, Key, Modifiers, Sense};
use std::path::PathBuf;
use traco_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, Point, Rect},
    Color, Command, Engine, Fill, PageId, ShapeId, Stroke,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rect,
    Ellipse,
    Polygon,
    Pan,
}

impl Tool {
    fn label(self) -> &'static str {
        match self {
            Tool::Select => "Pick",
            Tool::Rect => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Polygon => "Polygon",
            Tool::Pan => "Pan",
        }
    }
    fn shortcut(self) -> Key {
        match self {
            Tool::Select => Key::Space,
            Tool::Rect => Key::F6,
            Tool::Ellipse => Key::F7,
            Tool::Polygon => Key::Y,
            Tool::Pan => Key::H,
        }
    }
}

#[derive(Debug, Clone)]
enum Drag {
    None,
    /// Drawing a new shape from `start` (page space).
    Create { start: Point, current: Point },
    /// Moving the selection; `last` is the previous pointer position.
    Move { last: Point, total: traco_core::geometry::Vec2 },
    /// Rubber-band selection.
    Marquee { start: Point, current: Point },
}

pub struct App {
    engine: Engine,
    page: PageId,
    view: View,
    tool: Tool,
    selection: Vec<ShapeId>,
    drag: Drag,
    file: Option<PathBuf>,
    status: String,
    fit_pending: bool,
    // Property defaults for new objects.
    default_fill: Option<Color>,
    default_stroke: Option<Stroke>,
    polygon_points: u32,
    polygon_sharpness: f64,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let engine = Engine::default();
        let page = engine.document().pages[0].id;
        let mut app = App {
            engine,
            page,
            view: View::default(),
            tool: Tool::Select,
            selection: Vec::new(),
            drag: Drag::None,
            file: None,
            status: "Ready".into(),
            fit_pending: true,
            default_fill: None,
            default_stroke: Some(Stroke::default()),
            polygon_points: 5,
            polygon_sharpness: 0.0,
        };
        if let Some(p) = open {
            app.open_path(p);
        }
        app
    }

    // ----- file handling -------------------------------------------------

    fn open_dialog(&mut self) {
        let picked = rfd::FileDialog::new()
            .add_filter("All supported", &["cdr", "traco"])
            .add_filter("the editor", &["cdr"])
            .add_filter("Traco", &["traco"])
            .pick_file();
        if let Some(p) = picked {
            self.open_path(p);
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let result = if ext == "cdr" {
            traco_cdr::open(&path).map(|(doc, report)| {
                let ver = report.version.map(|v| v.name()).unwrap_or_default();
                let msg = format!(
                    "{ver}: {} page(s), {} shape(s), {} skipped, {} warning(s)",
                    report.pages,
                    report.shapes,
                    report.skipped_objects,
                    report.warnings.len()
                );
                for w in &report.warnings {
                    log::warn!("cdr: {w}");
                }
                (doc, msg)
            })
            .map_err(|e| e.to_string())
        } else {
            traco_io::load_native(&path).map(|d| (d, "Opened".to_string())).map_err(|e| e.to_string())
        };
        match result {
            Ok((doc, msg)) => {
                self.page = doc.pages[0].id;
                self.engine.replace(doc);
                self.selection.clear();
                self.file = if ext == "cdr" { None } else { Some(path.clone()) };
                self.status = format!("{}: {msg}", path.display());
                self.fit_pending = true;
            }
            Err(e) => self.status = format!("Could not open {}: {e}", path.display()),
        }
    }

    fn save(&mut self, save_as: bool) {
        let path = if save_as || self.file.is_none() {
            rfd::FileDialog::new().add_filter("Traco", &["traco"]).set_file_name("drawing.traco").save_file()
        } else {
            self.file.clone()
        };
        let Some(path) = path else { return };
        match traco_io::save_native(self.engine.document(), &path) {
            Ok(()) => {
                self.engine.mark_saved();
                self.file = Some(path.clone());
                self.status = format!("Saved {}", path.display());
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    fn export_svg(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("SVG", &["svg"]).set_file_name("drawing.svg").save_file() else {
            return;
        };
        let idx = self.engine.document().pages.iter().position(|p| p.id == self.page).unwrap_or(0);
        match traco_io::save_svg(self.engine.document(), idx, &path) {
            Ok(()) => self.status = format!("Exported {}", path.display()),
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }

    fn new_document(&mut self) {
        let doc = traco_core::Document::default();
        self.page = doc.pages[0].id;
        self.engine.replace(doc);
        self.selection.clear();
        self.file = None;
        self.fit_pending = true;
        self.status = "New document".into();
    }

    // ----- commands --------------------------------------------------------

    fn run(&mut self, cmd: Command) {
        if let Err(e) = self.engine.run(&cmd) {
            self.status = format!("{}: {e}", cmd.label());
        }
    }

    fn active_layer(&self) -> Option<traco_core::LayerId> {
        self.engine.document().page(self.page).ok().and_then(|p| p.layers.last()).map(|l| l.id)
    }

    fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = std::mem::take(&mut self.selection);
        self.run(Command::DeleteShapes { shapes });
    }

    fn create_shape(&mut self, a: Point, b: Point) {
        let rect = Rect::from_points(a, b);
        if rect.width() < 0.1 || rect.height() < 0.1 {
            return;
        }
        let kind = match self.tool {
            Tool::Rect => ShapeKind::Rect { rect, radius: 0.0 },
            Tool::Ellipse => ShapeKind::Ellipse { rect },
            Tool::Polygon => ShapeKind::Polygon { rect, points: self.polygon_points, sharpness: self.polygon_sharpness },
            _ => return,
        };
        let Some(layer) = self.active_layer() else { return };
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(id, kind);
        shape.fill = match self.default_fill {
            Some(c) => Fill::Solid(c),
            None => Fill::None,
        };
        shape.stroke = self.default_stroke.clone();
        self.run(Command::AddShape { layer, shape });
        self.selection = vec![id];
    }

    fn hit_test(&self, p: Point) -> Option<ShapeId> {
        let doc = self.engine.document();
        let page = doc.page(self.page).ok()?;
        // Top-most first.
        for layer in page.layers.iter().rev() {
            if !layer.visible || layer.locked {
                continue;
            }
            for s in layer.shapes.iter().rev() {
                if s.locked || !s.visible {
                    continue;
                }
                let tol = 2.0 / self.view.zoom as f64;
                if s.bounds().inflate(tol, tol).contains(p) {
                    return Some(s.id);
                }
            }
        }
        None
    }

    // ----- UI --------------------------------------------------------------

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New    Ctrl+N").clicked() {
                    self.new_document();
                    ui.close();
                }
                if ui.button("Open...    Ctrl+O").clicked() {
                    self.open_dialog();
                    ui.close();
                }
                ui.separator();
                if ui.button("Save    Ctrl+S").clicked() {
                    self.save(false);
                    ui.close();
                }
                if ui.button("Save As...").clicked() {
                    self.save(true);
                    ui.close();
                }
                if ui.button("Export SVG...    Ctrl+E").clicked() {
                    self.export_svg();
                    ui.close();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                let undo = self.engine.undo_label().map(|l| format!("Undo {l}")).unwrap_or("Undo".into());
                let redo = self.engine.redo_label().map(|l| format!("Redo {l}")).unwrap_or("Redo".into());
                if ui.add_enabled(self.engine.undo_label().is_some(), egui::Button::new(format!("{undo}    Ctrl+Z"))).clicked() {
                    let _ = self.engine.undo();
                    self.selection.clear();
                    ui.close();
                }
                if ui.add_enabled(self.engine.redo_label().is_some(), egui::Button::new(format!("{redo}    Ctrl+Shift+Z"))).clicked() {
                    let _ = self.engine.redo();
                    self.selection.clear();
                    ui.close();
                }
                ui.separator();
                if ui.add_enabled(!self.selection.is_empty(), egui::Button::new("Delete    Del")).clicked() {
                    self.delete_selection();
                    ui.close();
                }
                if ui.add_enabled(self.selection.len() > 1, egui::Button::new("Group    Ctrl+G")).clicked() {
                    self.group_selection();
                    ui.close();
                }
                if ui.add_enabled(self.selection.len() == 1, egui::Button::new("Ungroup    Ctrl+U")).clicked() {
                    self.ungroup_selection();
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Zoom to Page    Shift+F4").clicked() {
                    self.fit_pending = true;
                    ui.close();
                }
                if ui.button("Zoom In    +").clicked() {
                    self.view.zoom_at(self.view.origin, 1.25);
                    ui.close();
                }
                if ui.button("Zoom Out    -").clicked() {
                    self.view.zoom_at(self.view.origin, 0.8);
                    ui.close();
                }
            });
        });
    }

    fn group_selection(&mut self) {
        if self.selection.len() < 2 {
            return;
        }
        let shapes = std::mem::take(&mut self.selection);
        self.run(Command::Group { shapes });
        // The new group is the last-added id.
        let doc = self.engine.document();
        if let Ok(p) = doc.page(self.page) {
            if let Some(g) = p.layers.iter().flat_map(|l| &l.shapes).filter(|s| matches!(s.kind, ShapeKind::Group { .. })).max_by_key(|s| s.id) {
                self.selection = vec![g.id];
            }
        }
    }

    fn ungroup_selection(&mut self) {
        if let [group] = self.selection[..] {
            self.selection.clear();
            self.run(Command::Ungroup { group });
        }
    }

    fn toolbox(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tools");
        for t in [Tool::Select, Tool::Rect, Tool::Ellipse, Tool::Polygon, Tool::Pan] {
            if ui.selectable_label(self.tool == t, t.label()).clicked() {
                self.tool = t;
            }
        }
        if self.tool == Tool::Polygon {
            ui.separator();
            ui.add(egui::Slider::new(&mut self.polygon_points, 3..=24).text("points"));
            ui.add(egui::Slider::new(&mut self.polygon_sharpness, 0.0..=0.9).text("sharpness"));
        }
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Object Properties");
        let doc = self.engine.document();
        let selected: Vec<Shape> = self.selection.iter().filter_map(|id| doc.shape(*id).ok().map(|(_, s)| s.clone())).collect();

        if selected.is_empty() {
            ui.label("No selection. Defaults for new objects:");
            let mut has_fill = self.default_fill.is_some();
            if ui.checkbox(&mut has_fill, "Fill").changed() {
                self.default_fill = if has_fill { Some(Color::rgb8(200, 200, 200)) } else { None };
            }
            if let Some(c) = &mut self.default_fill {
                color_picker(ui, "Fill colour", c);
            }
            let mut has_stroke = self.default_stroke.is_some();
            if ui.checkbox(&mut has_stroke, "Outline").changed() {
                self.default_stroke = if has_stroke { Some(Stroke::default()) } else { None };
            }
            if let Some(s) = &mut self.default_stroke {
                stroke_editor(ui, s);
            }
            return;
        }

        let first = &selected[0];
        ui.label(format!("{} object(s) selected", selected.len()));
        let b = first.bounds();
        ui.monospace(format!("x {:.2}  y {:.2} mm", b.x0, b.y0));
        ui.monospace(format!("w {:.2}  h {:.2} mm", b.width(), b.height()));
        ui.separator();

        // Fill
        let mut fill_color = match &first.fill {
            Fill::Solid(c) => Some(*c),
            _ => None,
        };
        let mut has_fill = fill_color.is_some();
        let mut changed = false;
        if ui.checkbox(&mut has_fill, "Fill").changed() {
            fill_color = if has_fill { Some(Color::rgb8(200, 200, 200)) } else { None };
            changed = true;
        }
        if let Some(c) = &mut fill_color {
            changed |= color_picker(ui, "Fill colour", c);
        }
        if changed {
            let fill = fill_color.map(Fill::Solid).unwrap_or(Fill::None);
            let shapes = self.selection.clone();
            self.run(Command::SetFill { shapes, fill });
        }

        // Outline
        let mut stroke = first.stroke.clone();
        let mut has_stroke = stroke.is_some();
        let mut changed = false;
        if ui.checkbox(&mut has_stroke, "Outline").changed() {
            stroke = if has_stroke { Some(Stroke::default()) } else { None };
            changed = true;
        }
        if let Some(s) = &mut stroke {
            changed |= stroke_editor(ui, s);
        }
        if changed {
            let shapes = self.selection.clone();
            self.run(Command::SetStroke { shapes, stroke });
        }
    }

    fn layers_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Objects");
        let doc = self.engine.document();
        let Ok(page) = doc.page(self.page) else { return };
        let mut toggles: Vec<(traco_core::LayerId, bool)> = Vec::new();
        let mut click: Option<ShapeId> = None;
        for layer in page.layers.iter().rev() {
            let mut vis = layer.visible;
            ui.horizontal(|ui| {
                if ui.checkbox(&mut vis, "").changed() {
                    toggles.push((layer.id, vis));
                }
                ui.strong(&layer.name);
            });
            for s in layer.shapes.iter().rev() {
                let name = s.name.clone().unwrap_or_else(|| match &s.kind {
                    ShapeKind::Rect { .. } => "Rectangle".into(),
                    ShapeKind::Ellipse { .. } => "Ellipse".into(),
                    ShapeKind::Polygon { .. } => "Polygon".into(),
                    ShapeKind::Path { .. } => "Curve".into(),
                    ShapeKind::Text { .. } => "Text".into(),
                    ShapeKind::Group { children } => format!("Group of {}", children.len()),
                });
                let sel = self.selection.contains(&s.id);
                if ui.selectable_label(sel, format!("    {name}")).clicked() {
                    click = Some(s.id);
                }
            }
        }
        for (layer, visible) in toggles {
            self.run(Command::SetLayerVisible { layer, visible });
        }
        if let Some(id) = click {
            if ui.input(|i| i.modifiers.shift) {
                if !self.selection.contains(&id) {
                    self.selection.push(id);
                }
            } else {
                self.selection = vec![id];
            }
        }
    }

    fn pages_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let pages: Vec<(PageId, String)> = self.engine.document().pages.iter().map(|p| (p.id, p.name.clone())).collect();
            for (id, name) in pages {
                if ui.selectable_label(self.page == id, name).clicked() {
                    self.page = id;
                    self.selection.clear();
                    self.fit_pending = true;
                }
            }
            if ui.small_button("+").clicked() {
                let size = self.engine.document().page(self.page).map(|p| p.size).unwrap_or(traco_core::document::paper::A4);
                self.run(Command::AddPage { name: None, size });
            }
            ui.separator();
            ui.label(&self.status);
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(70));

        if self.fit_pending {
            if let Ok(p) = self.engine.document().page(self.page) {
                self.view.fit(p.rect(), rect);
            }
            self.fit_pending = false;
        }

        // Zoom with the wheel, pan with middle drag or the Pan tool.
        let hover = response.hover_pos();
        let (scroll, zoom_delta, mods) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers));
        if let Some(h) = hover {
            if zoom_delta != 1.0 {
                self.view.zoom_at(h, zoom_delta);
            } else if mods.ctrl && scroll.y != 0.0 {
                self.view.zoom_at(h, if scroll.y > 0.0 { 1.1 } else { 1.0 / 1.1 });
            } else if scroll != egui::Vec2::ZERO {
                self.view.pan(scroll);
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) || (self.tool == Tool::Pan && response.dragged()) {
            self.view.pan(response.drag_delta());
        }

        let pointer_page = hover.map(|h| self.view.to_page(h));

        // Tool interaction (primary button only).
        if self.tool != Tool::Pan {
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(p) = pointer_page {
                    match self.tool {
                        Tool::Select => {
                            match self.hit_test(p) {
                                Some(id) => {
                                    if mods.shift {
                                        if !self.selection.contains(&id) {
                                            self.selection.push(id);
                                        }
                                    } else if !self.selection.contains(&id) {
                                        self.selection = vec![id];
                                    }
                                    self.drag = Drag::Move { last: p, total: traco_core::geometry::Vec2::ZERO };
                                }
                                None => {
                                    if !mods.shift {
                                        self.selection.clear();
                                    }
                                    self.drag = Drag::Marquee { start: p, current: p };
                                }
                            }
                        }
                        _ => self.drag = Drag::Create { start: p, current: p },
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(p) = pointer_page {
                    match &mut self.drag {
                        Drag::Create { current, .. } | Drag::Marquee { current, .. } => *current = p,
                        Drag::Move { last, total } => {
                            let d = p - *last;
                            *last = p;
                            *total += d;
                        }
                        Drag::None => {}
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                let drag = std::mem::replace(&mut self.drag, Drag::None);
                match drag {
                    Drag::Create { start, current } => self.create_shape(start, current),
                    Drag::Move { total, .. } => {
                        if total.hypot() > 1e-6 && !self.selection.is_empty() {
                            let shapes = self.selection.clone();
                            self.run(Command::TransformShapes { shapes, transform: Affine::translate(total) });
                        }
                    }
                    Drag::Marquee { start, current } => {
                        let r = Rect::from_points(start, current);
                        if let Ok(page) = self.engine.document().page(self.page) {
                            for s in page.layers.iter().filter(|l| l.visible && !l.locked).flat_map(|l| &l.shapes) {
                                if r.contains_rect(s.bounds()) && !self.selection.contains(&s.id) {
                                    self.selection.push(s.id);
                                }
                            }
                        }
                    }
                    Drag::None => {}
                }
            }
            if response.clicked_by(egui::PointerButton::Primary) && self.tool == Tool::Select {
                if let Some(p) = pointer_page {
                    match self.hit_test(p) {
                        Some(id) if mods.shift => {
                            if let Some(i) = self.selection.iter().position(|s| *s == id) {
                                self.selection.remove(i);
                            } else {
                                self.selection.push(id);
                            }
                        }
                        Some(id) => self.selection = vec![id],
                        None => self.selection.clear(),
                    }
                }
            }
        }

        // Draw. While moving, draw the selection displaced without committing.
        let doc = self.engine.document();
        let preview_offset = match &self.drag {
            Drag::Move { total, .. } => *total,
            _ => traco_core::geometry::Vec2::ZERO,
        };
        if preview_offset.hypot() > 0.0 {
            let mut doc2 = doc.clone();
            for id in &self.selection {
                if let Ok(s) = doc2.shape_mut(*id) {
                    s.transform = Affine::translate(preview_offset) * s.transform;
                }
            }
            canvas::draw_page(&painter, &doc2, self.page, &self.view, &RenderOptions { selected: self.selection.clone() });
        } else {
            canvas::draw_page(&painter, doc, self.page, &self.view, &RenderOptions { selected: self.selection.clone() });
        }

        // Rubber bands.
        match &self.drag {
            Drag::Create { start, current } | Drag::Marquee { start, current } => {
                let r = self.view.rect_to_screen(Rect::from_points(*start, *current));
                painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(0, 120, 215)), egui::epaint::StrokeKind::Outside);
            }
            _ => {}
        }

        // Rulers-lite: pointer position in mm.
        if let Some(p) = pointer_page {
            painter.text(
                rect.left_bottom() + egui::vec2(8.0, -8.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{:.2}, {:.2} mm   {:.0}%", p.x, p.y, self.view.zoom / 3.7795 * 100.0),
                egui::FontId::monospace(12.0),
                Color32::from_gray(220),
            );
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());
        let consume = |k: Key, m: Modifiers| input.key_pressed(k) && input.modifiers.matches_logically(m);
        if consume(Key::Z, Modifiers::COMMAND) {
            let _ = self.engine.undo();
            self.selection.clear();
        }
        if consume(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT) || consume(Key::Y, Modifiers::COMMAND) {
            let _ = self.engine.redo();
            self.selection.clear();
        }
        if consume(Key::S, Modifiers::COMMAND) {
            self.save(false);
        }
        if consume(Key::O, Modifiers::COMMAND) {
            self.open_dialog();
        }
        if consume(Key::N, Modifiers::COMMAND) {
            self.new_document();
        }
        if consume(Key::E, Modifiers::COMMAND) {
            self.export_svg();
        }
        if consume(Key::G, Modifiers::COMMAND) {
            self.group_selection();
        }
        if consume(Key::U, Modifiers::COMMAND) {
            self.ungroup_selection();
        }
        if consume(Key::A, Modifiers::COMMAND) {
            if let Ok(p) = self.engine.document().page(self.page) {
                self.selection = p.layers.iter().flat_map(|l| &l.shapes).map(|s| s.id).collect();
            }
        }
        if consume(Key::Delete, Modifiers::NONE) || consume(Key::Backspace, Modifiers::NONE) {
            if !ctx.egui_wants_keyboard_input() {
                self.delete_selection();
            }
        }
        if consume(Key::F4, Modifiers::SHIFT) {
            self.fit_pending = true;
        }
        if !ctx.egui_wants_keyboard_input() {
            for t in [Tool::Select, Tool::Rect, Tool::Ellipse, Tool::Polygon, Tool::Pan] {
                if consume(t.shortcut(), Modifiers::NONE) {
                    self.tool = t;
                }
            }
            if consume(Key::Escape, Modifiers::NONE) {
                self.selection.clear();
                self.tool = Tool::Select;
            }
            // Nudge with arrows (the editor default 0.1 in = 2.54 mm; we use 1 mm).
            let nudge = if input.modifiers.shift { 10.0 } else { 1.0 };
            let mut d = traco_core::geometry::Vec2::ZERO;
            if input.key_pressed(Key::ArrowLeft) {
                d.x -= nudge;
            }
            if input.key_pressed(Key::ArrowRight) {
                d.x += nudge;
            }
            if input.key_pressed(Key::ArrowUp) {
                d.y += nudge;
            }
            if input.key_pressed(Key::ArrowDown) {
                d.y -= nudge;
            }
            if d.hypot() > 0.0 && !self.selection.is_empty() {
                let shapes = self.selection.clone();
                self.run(Command::TransformShapes { shapes, transform: Affine::translate(d) });
            }
        }
    }
}

fn color_picker(ui: &mut egui::Ui, label: &str, c: &mut Color) -> bool {
    let [r, g, b] = c.to_rgb8();
    let mut rgb = [r, g, b];
    let changed = ui.horizontal(|ui| {
        ui.label(label);
        ui.color_edit_button_srgb(&mut rgb).changed()
    })
    .inner;
    if changed {
        *c = Color::rgb8(rgb[0], rgb[1], rgb[2]);
    }
    changed
}

fn stroke_editor(ui: &mut egui::Ui, s: &mut Stroke) -> bool {
    let mut changed = color_picker(ui, "Outline colour", &mut s.color);
    let mut hair = s.width <= Stroke::HAIRLINE + 1e-9;
    if ui.checkbox(&mut hair, "Hairline").changed() {
        s.width = if hair { Stroke::HAIRLINE } else { 0.5 };
        changed = true;
    }
    if !hair {
        changed |= ui.add(egui::Slider::new(&mut s.width, 0.1..=20.0).text("width mm").logarithmic(true)).changed();
    }
    changed
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        self.shortcuts(ctx);

        let title = format!(
            "{}{} - Traco",
            if self.engine.is_dirty() { "*" } else { "" },
            self.file.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or(&self.engine.document().title)
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("pages").show(ui, |ui| self.pages_bar(ui));
        egui::Panel::left("toolbox").default_size(120.0).show(ui, |ui| self.toolbox(ui));
        egui::Panel::right("dockers").default_size(260.0).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.properties(ui);
                ui.separator();
                self.layers_panel(ui);
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| self.canvas(ui));
    }
}
