//! Tools > Options (the application options and the Customization, Tools,
//! Global and Workspaces dialogs) and Layout > Document Options, laid out
//! as the target design's: a white page list on the left, the page on
//! the right, the help button at the bottom left, OK and Cancel at the
//! bottom right. Cancel puts back what the dialog changed.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::settings::{AutoCenter, NodeShape, NodeSize, Startup};
use crate::theme::Tokens;
use crate::ui::chrome;
use egui::{Context, Ui};
use tracedraw_core::{geometry::Size, Color, Command, Fill};

/// Which dialog a page belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsSet {
    App,
    Customization,
    Tools,
    Global,
    Workspaces,
    Document,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionsPage {
    // Tools > Options > TraceDraw.
    #[default]
    General,
    Display,
    Edit,
    NodesAndHandles,
    ClipFrame,
    Snapping,
    Save,
    Text,
    // Tools > Options > Customization.
    Appearance,
    Commands,
    CommandBars,
    ColorPalette,
    // Tools > Options > Tools.
    Pick,
    ZoomPan,
    Rectangle,
    Ellipse,
    Polygon,
    Spiral,
    GraphPaper,
    Table,
    Eraser,
    // Tools > Options > Global.
    GlobalGeneral,
    FileLocations,
    // Tools > Options > Workspaces.
    Workspace,
    // Layout > Document Options.
    DocumentGeneral,
    PageSize,
    Layout,
    Background,
    Bleed,
    Rulers,
    Grid,
    Guidelines,
}

impl OptionsSet {
    pub fn pages(self) -> &'static [OptionsPage] {
        use OptionsPage::*;
        match self {
            OptionsSet::App => &[
                General,
                Display,
                Edit,
                NodesAndHandles,
                ClipFrame,
                Snapping,
                Save,
                Text,
            ],
            OptionsSet::Customization => &[Appearance, Commands, CommandBars, ColorPalette],
            OptionsSet::Tools => &[
                Pick, ZoomPan, Rectangle, Ellipse, Polygon, Spiral, GraphPaper, Table, Eraser,
            ],
            OptionsSet::Global => &[GlobalGeneral, FileLocations],
            OptionsSet::Workspaces => &[Workspace],
            OptionsSet::Document => &[
                DocumentGeneral,
                PageSize,
                Layout,
                Background,
                Bleed,
                Rulers,
                Grid,
                Guidelines,
            ],
        }
    }

    fn title(self) -> String {
        match self {
            OptionsSet::App => trf("options.title_app", &[("app", "TraceDraw")]),
            OptionsSet::Customization => tr("options.title_customization"),
            OptionsSet::Tools => tr("options.title_tools"),
            OptionsSet::Global => tr("options.title_global"),
            OptionsSet::Workspaces => tr("options.title_workspaces"),
            OptionsSet::Document => tr("dialog.document_options"),
        }
    }
}

impl OptionsPage {
    pub fn set(self) -> OptionsSet {
        [
            OptionsSet::App,
            OptionsSet::Customization,
            OptionsSet::Tools,
            OptionsSet::Global,
            OptionsSet::Workspaces,
            OptionsSet::Document,
        ]
        .into_iter()
        .find(|s| s.pages().contains(&self))
        .unwrap_or(OptionsSet::App)
    }

    pub fn key(self) -> &'static str {
        use OptionsPage::*;
        match self {
            General | GlobalGeneral | DocumentGeneral => "options.general",
            Display => "options.display",
            Edit => "options.edit",
            NodesAndHandles => "options.nodes_and_handles",
            ClipFrame => "options.clip_frame",
            Snapping => "options.snapping",
            Save => "options.save",
            Text => "options.text",
            Appearance => "options.appearance",
            Commands => "options.commands",
            CommandBars => "options.command_bars",
            ColorPalette => "options.color_palette",
            Pick => "tool.pick",
            ZoomPan => "options.zoom_pan",
            Rectangle => "tool.rectangle",
            Ellipse => "tool.ellipse",
            Polygon => "tool.polygon",
            Spiral => "tool.spiral",
            GraphPaper => "tool.graph_paper",
            Table => "tool.table",
            Eraser => "tool.eraser",
            FileLocations => "options.file_locations",
            Workspace => "options.workspace",
            PageSize => "options.page_size",
            Layout => "options.layout",
            Background => "options.background",
            Bleed => "options.bleed_page",
            Rulers => "options.rulers",
            Grid => "options.grid",
            Guidelines => "options.guidelines",
        }
    }
}

/// What the options dialogs may change, kept when one opens so that
/// Cancel can put it back. Document changes are commands, undone back to
/// the depth the history had.
pub struct OptionsSnapshot {
    settings: crate::settings::Settings,
    language: String,
    units: Units,
    nudge_mm: f64,
    duplicate_offset: tracedraw_core::geometry::Vec2,
    show_rulers: bool,
    show_grid: bool,
    show_guides: bool,
    show_baseline_grid: bool,
    show_pixel_grid: bool,
    show_page_border: bool,
    show_status_bar: bool,
    show_standard_toolbar: bool,
    show_property_bar: bool,
    show_toolbox: bool,
    show_text_toolbar: bool,
    show_zoom_toolbar: bool,
    show_transform_toolbar: bool,
    proof_colors: bool,
    snap: crate::snap::SnapSettings,
    polygon_points: u32,
    star_sharpness: f64,
    rect_corners: tracedraw_core::Corners,
    corners_together: bool,
    ellipse_arc: Option<tracedraw_core::EllipseArc>,
    spiral_revolutions: u32,
    spiral_logarithmic: bool,
    graph_rows: u32,
    graph_cols: u32,
    table_rows: u32,
    table_cols: u32,
    eraser_width: f64,
    eraser_square: bool,
    text_font: String,
    text_size_pt: f64,
    text_hyphenation: bool,
    show_non_printing: bool,
    workspace: crate::app::Workspace,
    undo_depth: usize,
}

impl OptionsSnapshot {
    pub fn take(app: &App) -> Self {
        OptionsSnapshot {
            settings: app.settings.clone(),
            language: crate::i18n::language(),
            units: app.units,
            nudge_mm: app.nudge_mm,
            duplicate_offset: app.duplicate_offset,
            show_rulers: app.show_rulers,
            show_grid: app.show_grid,
            show_guides: app.show_guides,
            show_baseline_grid: app.show_baseline_grid,
            show_pixel_grid: app.show_pixel_grid,
            show_page_border: app.show_page_border,
            show_status_bar: app.show_status_bar,
            show_standard_toolbar: app.show_standard_toolbar,
            show_property_bar: app.show_property_bar,
            show_toolbox: app.show_toolbox,
            show_text_toolbar: app.show_text_toolbar,
            show_zoom_toolbar: app.show_zoom_toolbar,
            show_transform_toolbar: app.show_transform_toolbar,
            proof_colors: app.proof_colors,
            snap: app.snap,
            polygon_points: app.polygon_points,
            star_sharpness: app.star_sharpness,
            rect_corners: app.rect_corners,
            corners_together: app.corners_together,
            ellipse_arc: app.ellipse_arc,
            spiral_revolutions: app.spiral_revolutions,
            spiral_logarithmic: app.spiral_logarithmic,
            graph_rows: app.graph_rows,
            graph_cols: app.graph_cols,
            table_rows: app.table_rows,
            table_cols: app.table_cols,
            eraser_width: app.eraser_width,
            eraser_square: app.eraser_square,
            text_font: app.text_font.clone(),
            text_size_pt: app.text_size_pt,
            text_hyphenation: app.text_hyphenation,
            show_non_printing: app.show_non_printing,
            workspace: app.workspace,
            undo_depth: undo_depth(app),
        }
    }

    /// Put everything back (Cancel).
    pub fn restore(self, app: &mut App) {
        if app.has_document() {
            while undo_depth(app) > self.undo_depth {
                if app.engine.undo().is_err() {
                    break;
                }
            }
        }
        crate::i18n::set_language(&self.language);
        app.settings = self.settings;
        app.units = self.units;
        app.nudge_mm = self.nudge_mm;
        app.duplicate_offset = self.duplicate_offset;
        app.show_rulers = self.show_rulers;
        app.show_grid = self.show_grid;
        app.show_guides = self.show_guides;
        app.show_baseline_grid = self.show_baseline_grid;
        app.show_pixel_grid = self.show_pixel_grid;
        app.show_page_border = self.show_page_border;
        app.show_status_bar = self.show_status_bar;
        app.show_standard_toolbar = self.show_standard_toolbar;
        app.show_property_bar = self.show_property_bar;
        app.show_toolbox = self.show_toolbox;
        app.show_text_toolbar = self.show_text_toolbar;
        app.show_zoom_toolbar = self.show_zoom_toolbar;
        app.show_transform_toolbar = self.show_transform_toolbar;
        app.proof_colors = self.proof_colors;
        app.snap = self.snap;
        app.polygon_points = self.polygon_points;
        app.star_sharpness = self.star_sharpness;
        app.rect_corners = self.rect_corners;
        app.corners_together = self.corners_together;
        app.ellipse_arc = self.ellipse_arc;
        app.spiral_revolutions = self.spiral_revolutions;
        app.spiral_logarithmic = self.spiral_logarithmic;
        app.graph_rows = self.graph_rows;
        app.graph_cols = self.graph_cols;
        app.table_rows = self.table_rows;
        app.table_cols = self.table_cols;
        app.eraser_width = self.eraser_width;
        app.eraser_square = self.eraser_square;
        app.text_font = self.text_font;
        app.text_size_pt = self.text_size_pt;
        app.text_hyphenation = self.text_hyphenation;
        app.show_non_printing = self.show_non_printing;
        if app.workspace != self.workspace {
            app.set_workspace(self.workspace);
        }
        app.apply_runtime_settings();
    }
}

fn undo_depth(app: &App) -> usize {
    app.engine.history_labels().0.len()
}

/// Draw the options dialog of the set `app.options_page` belongs to.
pub fn options_dialog(app: &mut App, ctx: &Context, close: &mut bool) {
    if app.options_snapshot.is_none() {
        app.options_snapshot = Some(OptionsSnapshot::take(app));
    }
    let set = app.options_page.set();
    let mut ok = false;
    let mut cancel = false;
    let x_closed = chrome::dialog(
        ctx,
        "options_dialog",
        &set.title(),
        egui::vec2(887.0, 663.0),
        |ui| {
            let body = ui.max_rect();
            let list = egui::Rect::from_min_size(
                body.min + egui::vec2(14.0, 10.0),
                egui::vec2(225.0, 566.0),
            );
            let page = egui::Rect::from_min_size(
                body.min + egui::vec2(251.0, 10.0),
                egui::vec2(622.0, 566.0),
            );
            page_list(app, ui, list, set);
            ui.scope_builder(egui::UiBuilder::new().max_rect(page), |ui| {
                chrome::panel()
                    .inner_margin(egui::Margin {
                        left: 20,
                        right: 20,
                        top: 16,
                        bottom: 12,
                    })
                    .show(ui, |ui| {
                        ui.set_min_size(page.size() - egui::vec2(42.0, 30.0));
                        ui.set_max_width(page.width() - 42.0);
                        egui::ScrollArea::vertical()
                            .id_salt(("options_page", app.options_page.key()))
                            .auto_shrink([false, false])
                            .max_height(page.height() - 30.0)
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 7.0;
                                draw_page(app, ui);
                            });
                    });
            });
            let buttons = egui::Rect::from_min_size(
                body.min + egui::vec2(14.0, 10.0 + 566.0 + 15.0),
                egui::vec2(859.0, 27.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(buttons), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_sized([27.0, 27.0], egui::Button::new("?"))
                        .on_hover_text(tr("dialog.help"))
                        .clicked()
                    {
                        app.open_url(
                            "https://github.com/igtoth/tracedraw/blob/main/docs/behavior/README.md",
                        );
                    }
                    if set == OptionsSet::Document
                        && ui
                            .add_sized(
                                [150.0, 27.0],
                                egui::Button::new(tr("options.save_as_default")),
                            )
                            .clicked()
                    {
                        app.settings.document_defaults =
                            crate::settings::DocumentDefaults::of(app.doc());
                        app.status = tr("status.settings_saved");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if chrome::button(ui, &tr("dialog.cancel")).clicked() {
                            cancel = true;
                        }
                        ui.add_space(5.0);
                        if chrome::button(ui, &tr("dialog.ok")).clicked() {
                            ok = true;
                        }
                    });
                });
            });
        },
    );
    if ok {
        app.options_snapshot = None;
        app.apply_runtime_settings();
        app.save_settings();
        *close = true;
    } else if cancel || x_closed {
        if let Some(s) = app.options_snapshot.take() {
            s.restore(app);
        }
        *close = true;
    }
}

/// The page list: one row per page, the current one highlighted.
fn page_list(app: &mut App, ui: &mut Ui, rect: egui::Rect, set: OptionsSet) {
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, egui::Color32::WHITE);
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(0xD9)),
        egui::StrokeKind::Inside,
    );
    for (i, p) in set.pages().iter().enumerate() {
        let row = egui::Rect::from_min_size(
            rect.min + egui::vec2(5.0, 5.0 + i as f32 * 23.0),
            egui::vec2(rect.width() - 10.0, 22.0),
        );
        let resp = ui.interact(
            row,
            egui::Id::new(("options_list", p.key(), i)),
            egui::Sense::click(),
        );
        let selected = app.options_page == *p;
        if selected {
            ui.painter()
                .rect_filled(row, 0.0, egui::Color32::from_rgb(0xCC, 0xE8, 0xFF));
            ui.painter().rect_stroke(
                row,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(0x99, 0xD1, 0xFF)),
                egui::StrokeKind::Inside,
            );
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(row, 0.0, egui::Color32::from_rgb(0xE5, 0xF3, 0xFF));
        }
        ui.painter().text(
            row.left_center() + egui::vec2(5.0, 0.0),
            egui::Align2::LEFT_CENTER,
            tr(p.key()),
            egui::FontId::proportional(13.0),
            Tokens::TEXT,
        );
        if resp.clicked() {
            app.options_page = *p;
        }
    }
}

fn draw_page(app: &mut App, ui: &mut Ui) {
    use OptionsPage::*;
    match app.options_page {
        General => general_page(app, ui),
        Display => display_page(app, ui),
        Edit => edit_page(app, ui),
        NodesAndHandles => nodes_page(app, ui),
        ClipFrame => clip_frame_page(app, ui),
        Snapping => snapping_page(app, ui),
        Save => save_page(app, ui),
        Text => text_page(app, ui),
        Appearance => appearance_page(app, ui),
        Commands => commands_page(app, ui),
        CommandBars => command_bars_page(app, ui),
        ColorPalette => color_palette_page(app, ui),
        Pick => pick_page(app, ui),
        ZoomPan => zoom_pan_page(app, ui),
        Rectangle => rectangle_page(app, ui),
        Ellipse => ellipse_page(app, ui),
        Polygon => polygon_page(app, ui),
        Spiral => spiral_page(app, ui),
        GraphPaper => graph_paper_page(app, ui),
        Table => table_page(app, ui),
        Eraser => eraser_page(app, ui),
        GlobalGeneral => global_general_page(app, ui),
        FileLocations => file_locations_page(app, ui),
        Workspace => workspace_page(app, ui),
        DocumentGeneral => document_general_page(app, ui),
        PageSize => crate::ui::dialogs::page_size_page(app, ui),
        Layout => layout_page(app, ui),
        Background => background_page(app, ui),
        Bleed => bleed_page(app, ui),
        Rulers => crate::ui::layout_options::rulers_page(app, ui),
        Grid => crate::ui::layout_options::grid_page(app, ui),
        Guidelines => crate::ui::layout_options::guidelines_page(app, ui),
    }
}

/// A label right-aligned in a grid's first column.
fn row_label(ui: &mut Ui, text: &str) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(text);
    });
}

fn grid(id: &str) -> egui::Grid {
    egui::Grid::new(id).num_columns(2).spacing([8.0, 8.0])
}

fn combo<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: &str,
    value: &mut T,
    options: &[T],
    label: impl Fn(T) -> String,
    width: f32,
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(label(*value))
        .width(width)
        .show_ui(ui, |ui| {
            for o in options {
                if ui.selectable_label(*value == *o, label(*o)).clicked() {
                    *value = *o;
                    changed = true;
                }
            }
        });
    changed
}

fn rgb_button(ui: &mut Ui, rgb: &mut [u8; 3]) -> bool {
    ui.color_edit_button_srgb(rgb).changed()
}

// ----- TraceDraw options -------------------------------------------------

fn general_page(app: &mut App, ui: &mut Ui) {
    chrome::section(ui, &tr("options.getting_started"));
    ui.indent("gs", |ui| {
        grid("startup").show(ui, |ui| {
            row_label(ui, &trf("options.on_startup", &[("app", "TraceDraw")]));
            let mut s = app.settings.startup;
            if combo(ui, "startup", &mut s, &Startup::ALL, |v| tr(v.key()), 240.0) {
                app.settings.startup = s;
                app.settings.show_welcome_on_start = s == Startup::WelcomeScreen;
            }
            ui.end_row();
        });
        ui.checkbox(
            &mut app.settings.show_new_document_dialog,
            tr("options.show_new_document_dialog"),
        );
    });
    chrome::section(ui, &tr("options.undo_levels"));
    ui.indent("undo", |ui| {
        grid("undo_grid").show(ui, |ui| {
            row_label(ui, &tr("options.undo_regular"));
            ui.add_sized(
                [70.0, 22.0],
                egui::DragValue::new(&mut app.settings.undo_levels).range(1..=99_999),
            );
            ui.end_row();
        });
    });
}

fn display_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(&mut app.proof_colors, tr("menu.view.proof_colors"));
    chrome::section(ui, &tr("options.other_controls"));
    ui.indent("oc", |ui| {
        ui.checkbox(&mut app.settings.show_tooltips, tr("options.show_tooltips"));
        ui.checkbox(
            &mut app.settings.hide_bbox_curve_tools,
            tr("options.hide_bbox_curve_tools"),
        );
    });
    chrome::section(ui, &tr("options.full_screen_preview"));
    ui.indent("fsp", |ui| {
        ui.checkbox(
            &mut app.settings.preview_page_border,
            tr("options.show_page_border"),
        );
    });
    ui.separator();
    grid("display_wheel").show(ui, |ui| {
        row_label(ui, &tr("options.wheel_default_action"));
        let mut zoom = app.settings.wheel_zooms;
        combo(
            ui,
            "wheel",
            &mut zoom,
            &[true, false],
            |v| {
                tr(if v {
                    "options.wheel_zoom"
                } else {
                    "options.wheel_scroll"
                })
            },
            120.0,
        );
        app.settings.wheel_zooms = zoom;
        ui.end_row();
    });
}

fn edit_page(app: &mut App, ui: &mut Ui) {
    grid("edit").show(ui, |ui| {
        row_label(ui, &tr("options.constrain_angle"));
        ui.horizontal(|ui| {
            ui.add_sized(
                [80.0, 22.0],
                egui::DragValue::new(&mut app.settings.constrain_angle)
                    .range(0.1..=90.0)
                    .max_decimals(1)
                    .speed(0.5),
            );
            ui.label(tr("options.degrees"));
        });
        ui.end_row();
        row_label(ui, &tr("options.drawing_precision"));
        ui.horizontal(|ui| {
            ui.add_sized(
                [80.0, 22.0],
                egui::DragValue::new(&mut app.settings.precision).range(0..=10),
            );
            ui.label(tr("options.decimal_places"));
        });
        ui.end_row();
    });
}

fn shape_label(s: NodeShape) -> String {
    match s {
        NodeShape::Square => "\u{25A1}".into(),
        NodeShape::Circle => "\u{25CB}".into(),
        NodeShape::Diamond => "\u{25C7}".into(),
    }
}

fn nodes_page(app: &mut App, ui: &mut Ui) {
    let n = &mut app.settings.nodes;
    grid("node_size").show(ui, |ui| {
        row_label(ui, &tr("options.node_size"));
        combo(
            ui,
            "node_size",
            &mut n.size,
            &NodeSize::ALL,
            |v| tr(v.key()),
            100.0,
        );
        ui.end_row();
    });
    chrome::section(ui, &tr("options.node_shape"));
    grid("node_shapes").show(ui, |ui| {
        row_label(ui, &tr("options.node_cusp"));
        combo(ui, "cusp", &mut n.cusp, &NodeShape::ALL, shape_label, 50.0);
        ui.end_row();
        row_label(ui, &tr("options.node_smooth"));
        combo(
            ui,
            "smooth",
            &mut n.smooth,
            &NodeShape::ALL,
            shape_label,
            50.0,
        );
        ui.end_row();
        row_label(ui, &tr("options.node_symmetrical"));
        combo(
            ui,
            "symmetrical",
            &mut n.symmetrical,
            &NodeShape::ALL,
            shape_label,
            50.0,
        );
        ui.end_row();
        ui.label("");
        if chrome::button(ui, &tr("options.reset")).clicked() {
            let d = crate::settings::NodePrefs::default();
            n.cusp = d.cusp;
            n.smooth = d.smooth;
            n.symmetrical = d.symmetrical;
        }
        ui.end_row();
    });
    ui.checkbox(&mut n.show_direction, tr("options.show_curve_direction"));
    chrome::section(ui, &tr("options.colors"));
    grid("node_colors").show(ui, |ui| {
        row_label(ui, &tr("options.main_color"));
        ui.horizontal(|ui| {
            rgb_button(ui, &mut n.main_rgb);
            if ui.button(tr("options.reset")).clicked() {
                n.main_rgb = crate::settings::NodePrefs::default().main_rgb;
            }
        });
        ui.end_row();
        row_label(ui, &tr("options.secondary_color"));
        ui.horizontal(|ui| {
            rgb_button(ui, &mut n.secondary_rgb);
            if ui.button(tr("options.reset")).clicked() {
                n.secondary_rgb = crate::settings::NodePrefs::default().secondary_rgb;
            }
        });
        ui.end_row();
    });
    ui.checkbox(
        &mut n.unselected_filled,
        tr("options.unselected_nodes_filled"),
    );
}

fn clip_frame_page(app: &mut App, ui: &mut Ui) {
    let p = &mut app.settings.clip_frame;
    ui.label(tr("options.auto_center"));
    ui.indent("ac", |ui| {
        for (v, key) in [
            (AutoCenter::WhenOutside, "options.auto_center_outside"),
            (AutoCenter::Always, "options.auto_center_always"),
            (AutoCenter::Never, "options.auto_center_never"),
        ] {
            ui.radio_value(&mut p.auto_center, v, tr(key));
        }
    });
    ui.separator();
    ui.checkbox(&mut p.empty_lines, tr("options.empty_frame_lines"));
}

fn snapping_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(&mut app.snap.objects, tr("options.snap_to_objects"));
    ui.checkbox(&mut app.snap.page, tr("options.snap_to_page"));
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(tr("options.snapping_radius"));
        ui.add_sized(
            [70.0, 22.0],
            egui::DragValue::new(&mut app.settings.snap.threshold_px).range(1.0..=100.0),
        );
        ui.label(tr("options.pixels"));
    });
    ui.checkbox(&mut app.snap.guides, tr("options.snap_to_guidelines"));
    ui.checkbox(&mut app.snap.grid, tr("options.snap_to_grid"));
    ui.checkbox(
        &mut app.snap.baseline_grid,
        tr("menu.view.snap_baseline_grid"),
    );
    ui.checkbox(&mut app.snap.pixels, tr("options.snap_to_pixels"));
    ui.checkbox(
        &mut app.settings.snap.show_marks,
        tr("options.show_snap_marks"),
    );
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.add_enabled(
            app.settings.snap.show_marks,
            egui::Checkbox::new(
                &mut app.settings.snap.screen_tips,
                tr("options.snap_screen_tip"),
            ),
        );
    });
    chrome::section(ui, &tr("options.snap_modes"));
    ui.indent("snap_modes", |ui| {
        let modes = &mut app.settings.snap.modes;
        for m in crate::snap_points::SnapMode::ALL {
            ui.horizontal(|ui| {
                let mut on = modes.get(m);
                if ui.checkbox(&mut on, "").changed() {
                    modes.set(m, on);
                }
                let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                crate::canvas::draw_snap_glyph(ui.painter(), r.center(), m, Tokens::SELECTION);
                ui.label(tr(m.key()));
            });
        }
        ui.horizontal(|ui| {
            if ui.button(tr("options.select_all")).clicked() {
                *modes = crate::snap_points::SnapModes::default();
            }
            if ui.button(tr("options.deselect_all")).clicked() {
                *modes = crate::snap_points::SnapModes::none();
            }
        });
    });
}

/// A read-only path with a Browse button that picks a folder (desktop).
fn path_field(ui: &mut Ui, dir: &mut Option<std::path::PathBuf>, fallback: String) {
    let text = dir
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or(fallback);
    ui.horizontal(|ui| {
        ui.add_enabled(
            false,
            egui::TextEdit::singleline(&mut text.clone()).desired_width(320.0),
        );
        if ui
            .add_enabled(!crate::files::WEB, egui::Button::new(tr("options.browse")))
            .clicked()
        {
            if let Some(p) = crate::files::pick_folder() {
                *dir = Some(p);
            }
        }
    });
}

fn save_page(app: &mut App, ui: &mut Ui) {
    let b = &mut app.settings.backup;
    chrome::section(ui, &tr("options.backup"));
    ui.indent("bk", |ui| {
        ui.checkbox(&mut b.before_save, tr("options.backup_before_save"));
        ui.add_enabled_ui(b.before_save, |ui| {
            let mut choose = b.before_save_dir.is_some();
            ui.horizontal(|ui| {
                ui.label(tr("options.backup_to"));
                ui.vertical(|ui| {
                    ui.radio_value(&mut choose, false, tr("options.same_folder"));
                    ui.radio_value(&mut choose, true, tr("options.choose_location"));
                });
            });
            if choose && b.before_save_dir.is_none() {
                b.before_save_dir = Some(std::env::temp_dir());
            } else if !choose {
                b.before_save_dir = None;
            }
            ui.add_enabled_ui(choose, |ui| {
                path_field(ui, &mut b.before_save_dir, String::new());
            });
        });
    });
    chrome::section(ui, &tr("options.auto_backup"));
    ui.indent("ab", |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut b.auto, tr("options.backup_every"));
            ui.add_enabled(b.auto, egui::DragValue::new(&mut b.minutes).range(1..=999));
            ui.label(tr("options.minutes"));
        });
        ui.add_enabled_ui(b.auto, |ui| {
            path_field(
                ui,
                &mut b.auto_dir,
                crate::app::App::auto_backup_dir_default()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            );
        });
    });
}

fn text_page(app: &mut App, ui: &mut Ui) {
    grid("text_display").show(ui, |ui| {
        row_label(ui, &tr("options.keyboard_text_increment"));
        ui.horizontal(|ui| {
            ui.add_sized(
                [70.0, 22.0],
                egui::DragValue::new(&mut app.settings.text_increment_pt)
                    .range(0.1..=100.0)
                    .max_decimals(1),
            );
            ui.label(tr("options.points"));
        });
        ui.end_row();
        row_label(ui, &tr("options.default_font"));
        egui::ComboBox::from_id_salt("opt_font")
            .selected_text(app.text_font.clone())
            .width(200.0)
            .show_ui(ui, |ui| {
                for f in app.font_families.clone() {
                    if ui.selectable_label(app.text_font == f, &f).clicked() {
                        app.text_font = f;
                    }
                }
            });
        ui.end_row();
        row_label(ui, &tr("options.default_size"));
        ui.add(
            egui::DragValue::new(&mut app.text_size_pt)
                .range(1.0..=999.0)
                .suffix(" pt"),
        );
        ui.end_row();
    });
    ui.checkbox(&mut app.text_hyphenation, tr("menu.text.use_hyphenation"));
    ui.checkbox(
        &mut app.show_non_printing,
        tr("menu.text.show_non_printing"),
    );
}

// ----- Customization -----------------------------------------------------

fn appearance_page(app: &mut App, ui: &mut Ui) {
    grid("appearance").show(ui, |ui| {
        row_label(ui, &tr("options.desktop_color"));
        ui.horizontal(|ui| {
            rgb_button(ui, &mut app.settings.desktop_rgb);
            if ui.button(tr("options.reset")).clicked() {
                app.settings.desktop_rgb = [255, 255, 255];
            }
        });
        ui.end_row();
    });
    ui.checkbox(&mut app.show_page_border, tr("menu.view.page_border"));
    ui.checkbox(
        &mut app.settings.show_outline_flyout,
        tr("options.show_outline_flyout"),
    );
}

/// Customization > Color Palette: the docked palettes' swatches, the
/// right mouse button and the document palette.
fn color_palette_page(app: &mut App, ui: &mut Ui) {
    let p = &mut app.settings.palette;
    crate::ui::chrome::section(ui, &tr("options.palette_swatches"));
    ui.checkbox(&mut p.show_no_color, tr("options.palette_no_color"));
    crate::ui::chrome::section(ui, &tr("options.palette_right_button"));
    ui.radio_value(
        &mut p.right_click_outline,
        false,
        tr("options.palette_context_menu"),
    );
    ui.radio_value(
        &mut p.right_click_outline,
        true,
        tr("options.palette_set_outline"),
    );
    crate::ui::chrome::section(ui, &tr("palette.show_document"));
    ui.checkbox(&mut p.show_document, tr("options.palette_show_document"));
    ui.checkbox(
        &mut p.auto_update_document,
        tr("options.palette_auto_update"),
    );
}

fn commands_page(app: &mut App, ui: &mut Ui) {
    ui.label(egui::RichText::new(tr("options.shortcuts_hint")).color(Tokens::TEXT_DIM));
    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            egui::Grid::new("shortcuts").striped(true).show(ui, |ui| {
                for t in crate::tools::Tool::ALL {
                    ui.label(t.name());
                    let current = app
                        .settings
                        .shortcuts
                        .iter()
                        .find(|(k, _)| k == t.id())
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| t.shortcut().unwrap_or("").to_string());
                    let mut v = current.clone();
                    if ui
                        .add(egui::TextEdit::singleline(&mut v).desired_width(100.0))
                        .lost_focus()
                        && v != current
                    {
                        app.settings.shortcuts.retain(|(k, _)| k != t.id());
                        if !v.is_empty() {
                            app.settings.shortcuts.push((t.id().to_string(), v));
                        }
                    }
                    ui.end_row();
                }
            });
        });
    if ui.button(tr("options.reset_shortcuts")).clicked() {
        app.settings.shortcuts.clear();
    }
}

fn command_bars_page(app: &mut App, ui: &mut Ui) {
    ui.label(tr("options.command_bars_hint"));
    for (flag, key) in [
        (
            &mut app.show_standard_toolbar,
            "menu.window.toolbar_standard",
        ),
        (
            &mut app.show_property_bar,
            "menu.window.toolbar_property_bar",
        ),
        (&mut app.show_toolbox, "menu.window.toolbar_toolbox"),
        (&mut app.show_status_bar, "menu.window.toolbar_status_bar"),
        (&mut app.show_text_toolbar, "menu.window.toolbar_text"),
        (&mut app.show_zoom_toolbar, "menu.window.toolbar_zoom"),
        (
            &mut app.show_transform_toolbar,
            "menu.window.toolbar_transform",
        ),
    ] {
        ui.checkbox(flag, tr(key));
    }
}

// ----- Tools -------------------------------------------------------------

fn pick_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(
        &mut app.settings.crosshair_cursor,
        tr("options.crosshair_cursor"),
    );
    ui.checkbox(
        &mut app.settings.treat_all_filled,
        tr("options.treat_all_filled"),
    );
}

fn zoom_pan_page(app: &mut App, ui: &mut Ui) {
    ui.label(tr("options.zoom_right_button"));
    ui.indent("zr", |ui| {
        ui.radio_value(
            &mut app.settings.zoom_right_click_out,
            true,
            tr("options.zoom_out"),
        );
        ui.radio_value(
            &mut app.settings.zoom_right_click_out,
            false,
            tr("options.context_menu"),
        );
    });
    ui.separator();
    ui.label(tr("options.wheel_action"));
    ui.indent("wa", |ui| {
        ui.radio_value(
            &mut app.settings.wheel_zooms,
            true,
            tr("options.wheel_zoom"),
        );
        ui.radio_value(
            &mut app.settings.wheel_zooms,
            false,
            tr("options.wheel_scroll"),
        );
    });
}

fn rectangle_page(app: &mut App, ui: &mut Ui) {
    use tracedraw_core::{CornerKind, Corners};
    let mut c = app.rect_corners;
    let u = app.units;
    let together = app.corners_together;
    chrome::section(ui, &tr("options.rectangle_corners"));
    ui.indent("rect_corners", |ui| {
        ui.horizontal(|ui| {
            for (kind, key) in [
                (CornerKind::Round, "toolbar.corner_round"),
                (CornerKind::Scallop, "toolbar.corner_scallop"),
                (CornerKind::Chamfer, "toolbar.corner_chamfer"),
            ] {
                ui.radio_value(&mut c.kind, kind, tr(key));
            }
        });
        egui::Grid::new("rect_corners_grid")
            .num_columns(4)
            .spacing([8.0, 8.0])
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
                        row_label(ui, &tr(key));
                        let mut v = u.from_mm(c.radii[i]);
                        if ui
                            .add_sized(
                                [80.0, 22.0],
                                egui::DragValue::new(&mut v)
                                    .range(0.0..=10_000.0)
                                    .speed(0.1)
                                    .suffix(format!(" {}", u.short())),
                            )
                            .changed()
                        {
                            let v = u.to_mm(v).max(0.0);
                            if together {
                                c.radii = [v; 4];
                            } else {
                                c.radii[i] = v;
                            }
                        }
                    }
                    ui.end_row();
                }
            });
        ui.checkbox(&mut app.corners_together, tr("toolbar.corners_together"));
    });
    chrome::section(ui, &tr("options.scale_corners"));
    ui.indent("scale_corners", |ui| {
        let mut relative = !c.fixed;
        if ui
            .checkbox(&mut relative, tr("toolbar.relative_corners"))
            .changed()
        {
            c.fixed = !relative;
        }
    });
    app.rect_corners = c;
}

fn ellipse_page(app: &mut App, ui: &mut Ui) {
    let mut kind = match app.ellipse_arc {
        None => 0,
        Some(a) if a.pie => 1,
        Some(_) => 2,
    };
    let before = kind;
    ui.radio_value(&mut kind, 0, tr("options.ellipse"));
    ui.radio_value(&mut kind, 1, tr("options.pie"));
    ui.radio_value(&mut kind, 2, tr("options.arc"));
    let mut arc = app.ellipse_arc.unwrap_or(tracedraw_core::EllipseArc {
        start_deg: 0.0,
        end_deg: 270.0,
        pie: true,
    });
    if kind != before {
        app.ellipse_arc = match kind {
            0 => None,
            k => {
                arc.pie = k == 1;
                Some(arc)
            }
        };
    }
    if let Some(a) = &mut app.ellipse_arc {
        grid("arc").show(ui, |ui| {
            row_label(ui, &tr("options.starting_angle"));
            ui.add(
                egui::DragValue::new(&mut a.start_deg)
                    .range(-360.0..=360.0)
                    .suffix("°"),
            );
            ui.end_row();
            row_label(ui, &tr("options.ending_angle"));
            ui.add(
                egui::DragValue::new(&mut a.end_deg)
                    .range(-360.0..=360.0)
                    .suffix("°"),
            );
            ui.end_row();
        });
    }
}

fn polygon_page(app: &mut App, ui: &mut Ui) {
    grid("polygon").show(ui, |ui| {
        row_label(ui, &tr("options.number_of_points"));
        ui.add(egui::DragValue::new(&mut app.polygon_points).range(3..=500));
        ui.end_row();
        row_label(ui, &tr("options.sharpness"));
        // Stored as a fraction, shown in percent like the property bar.
        let mut pct = (app.star_sharpness * 100.0).round();
        if ui
            .add(egui::DragValue::new(&mut pct).range(1.0..=99.0).suffix("%"))
            .changed()
        {
            app.star_sharpness = (pct / 100.0).clamp(0.01, 0.99);
        }
        ui.end_row();
    });
}

fn spiral_page(app: &mut App, ui: &mut Ui) {
    grid("spiral").show(ui, |ui| {
        row_label(ui, &tr("options.revolutions"));
        ui.add(egui::DragValue::new(&mut app.spiral_revolutions).range(1..=100));
        ui.end_row();
    });
    ui.radio_value(
        &mut app.spiral_logarithmic,
        false,
        tr("options.symmetrical_spiral"),
    );
    ui.radio_value(
        &mut app.spiral_logarithmic,
        true,
        tr("options.logarithmic_spiral"),
    );
}

fn graph_paper_page(app: &mut App, ui: &mut Ui) {
    grid("graph").show(ui, |ui| {
        row_label(ui, &tr("options.rows"));
        ui.add(egui::DragValue::new(&mut app.graph_rows).range(1..=99));
        ui.end_row();
        row_label(ui, &tr("options.columns"));
        ui.add(egui::DragValue::new(&mut app.graph_cols).range(1..=99));
        ui.end_row();
    });
}

fn table_page(app: &mut App, ui: &mut Ui) {
    grid("table").show(ui, |ui| {
        row_label(ui, &tr("options.rows"));
        ui.add(egui::DragValue::new(&mut app.table_rows).range(1..=99));
        ui.end_row();
        row_label(ui, &tr("options.columns"));
        ui.add(egui::DragValue::new(&mut app.table_cols).range(1..=99));
        ui.end_row();
    });
}

fn eraser_page(app: &mut App, ui: &mut Ui) {
    grid("eraser").show(ui, |ui| {
        row_label(ui, &tr("options.eraser_width"));
        let u = app.units;
        let mut v = u.from_mm(app.eraser_width);
        if ui
            .add(
                egui::DragValue::new(&mut v)
                    .speed(0.1)
                    .suffix(format!(" {}", u.short())),
            )
            .changed()
        {
            app.eraser_width = u.to_mm(v).max(0.01);
        }
        ui.end_row();
    });
    ui.radio_value(&mut app.eraser_square, false, tr("options.eraser_round"));
    ui.radio_value(&mut app.eraser_square, true, tr("options.eraser_square"));
}

// ----- Global ------------------------------------------------------------

fn global_general_page(app: &mut App, ui: &mut Ui) {
    grid("global").show(ui, |ui| {
        row_label(ui, &tr("options.language"));
        let current = crate::i18n::language();
        let name = crate::i18n::LANGUAGES
            .iter()
            .find(|(c, _)| *c == current)
            .map(|(_, n)| *n)
            .unwrap_or("English");
        egui::ComboBox::from_id_salt("opt_lang")
            .selected_text(name)
            .width(200.0)
            .show_ui(ui, |ui| {
                for (code, n) in crate::i18n::LANGUAGES {
                    if ui.selectable_label(current == code, n).clicked() {
                        crate::i18n::set_language(code);
                        app.settings.language = code.to_string();
                    }
                }
            });
        ui.end_row();
    });
}

fn file_locations_page(_app: &mut App, ui: &mut Ui) {
    ui.label(tr("options.save_hint"));
    if let Some(p) = crate::settings::Settings::path() {
        ui.label(
            egui::RichText::new(p.display().to_string())
                .color(Tokens::TEXT_DIM)
                .size(12.0),
        );
    }
    if let Some(p) = crate::app::App::auto_backup_dir_default() {
        ui.add_space(6.0);
        ui.label(tr("options.auto_backup"));
        ui.label(
            egui::RichText::new(p.display().to_string())
                .color(Tokens::TEXT_DIM)
                .size(12.0),
        );
    }
}

// ----- Workspaces --------------------------------------------------------

fn workspace_page(app: &mut App, ui: &mut Ui) {
    ui.label(tr("options.workspace_choose"));
    let frame = chrome::panel();
    frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        for ws in [
            crate::app::Workspace::Default,
            crate::app::Workspace::Lite,
            crate::app::Workspace::Classic,
            crate::app::Workspace::Illustration,
            crate::app::Workspace::PageLayout,
        ] {
            if ui
                .selectable_label(
                    app.workspace == ws,
                    tr(&format!("menu.window.workspace_{}", ws.id())),
                )
                .clicked()
            {
                app.set_workspace(ws);
            }
        }
    });
}

// ----- Document Options --------------------------------------------------

fn document_general_page(app: &mut App, ui: &mut Ui) {
    if !app.has_document() {
        return;
    }
    let mut meta = app.doc().metadata.clone();
    grid("doc_general").show(ui, |ui| {
        row_label(ui, &tr("options.rendering_resolution"));
        ui.horizontal(|ui| {
            let mut dpi = app.document_dpi();
            if ui
                .add_sized(
                    [80.0, 22.0],
                    egui::DragValue::new(&mut dpi).range(36.0..=2400.0),
                )
                .changed()
            {
                meta.resolution_dpi = dpi;
            }
            ui.label("dpi");
        });
        ui.end_row();
    });
    ui.checkbox(&mut meta.fill_open_curves, tr("options.fill_open_curves"));
    let mut inflate = !meta.no_auto_inflate;
    if ui
        .checkbox(&mut inflate, tr("options.auto_inflate_bitmaps"))
        .changed()
    {
        meta.no_auto_inflate = !inflate;
    }
    if meta != app.doc().metadata {
        app.run(Command::SetMetadata { metadata: meta });
    }
    ui.separator();
    // Ctrl+D places duplicates this far from the original.
    grid("dup").show(ui, |ui| {
        row_label(ui, &tr("options.duplicate_offset"));
        let u = app.units;
        let (mut dx, mut dy) = (
            u.from_mm(app.duplicate_offset.x),
            u.from_mm(app.duplicate_offset.y),
        );
        ui.horizontal(|ui| {
            let a = ui.add(
                egui::DragValue::new(&mut dx)
                    .speed(0.1)
                    .suffix(format!(" {}", u.short())),
            );
            let b = ui.add(
                egui::DragValue::new(&mut dy)
                    .speed(0.1)
                    .suffix(format!(" {}", u.short())),
            );
            if a.changed() || b.changed() {
                app.duplicate_offset =
                    tracedraw_core::geometry::Vec2::new(u.to_mm(dx), u.to_mm(dy));
            }
        });
        ui.end_row();
    });
}

fn layout_page(app: &mut App, ui: &mut Ui) {
    let n = app.doc().pages.len();
    ui.label(trf("options.pages_count", &[("n", &n.to_string())]));
    ui.checkbox(&mut app.page_sorter, tr("menu.view.page_sorter"));
    if ui.button(tr("menu.layout.switch_orientation")).clicked() {
        let s = app.page_size();
        let page = app.page;
        app.run(Command::ResizePage {
            page,
            size: Size::new(s.height, s.width),
        });
        app.fit_pending = true;
    }
    ui.separator();
    ui.strong(tr("options.master_layers"));
    let masters: Vec<(tracedraw_core::LayerId, String, tracedraw_core::MasterScope)> = app
        .doc()
        .master
        .iter()
        .map(|l| (l.id, l.name.clone(), l.scope))
        .collect();
    for (id, name, scope) in masters {
        ui.horizontal(|ui| {
            ui.label(&name);
            for (s, k) in [
                (tracedraw_core::MasterScope::All, "options.scope_all"),
                (tracedraw_core::MasterScope::Odd, "options.scope_odd"),
                (tracedraw_core::MasterScope::Even, "options.scope_even"),
            ] {
                if ui.selectable_label(scope == s, tr(k)).clicked() {
                    app.run(Command::SetLayerScope {
                        layer: id,
                        scope: s,
                    });
                }
            }
        });
    }
    ui.horizontal(|ui| {
        for (s, k) in [
            (tracedraw_core::MasterScope::All, "options.new_master_all"),
            (tracedraw_core::MasterScope::Odd, "options.new_master_odd"),
            (tracedraw_core::MasterScope::Even, "options.new_master_even"),
        ] {
            if ui.button(tr(k)).clicked() {
                let n = app.doc().master.len() + 1;
                app.run(Command::AddMasterLayer {
                    name: format!("Master {n}"),
                    scope: s,
                });
            }
        }
    });
}

fn background_page(app: &mut App, ui: &mut Ui) {
    let page = app.page;
    let current = app.doc().page(page).ok().and_then(|p| p.background.clone());
    let mut kind = match &current {
        None => 0,
        Some(Fill::Solid(_)) => 1,
        Some(_) => 2,
    };
    ui.horizontal(|ui| {
        ui.radio_value(&mut kind, 0, tr("options.no_background"));
        ui.radio_value(&mut kind, 1, tr("options.solid"));
        ui.radio_value(&mut kind, 2, tr("options.bitmap"));
    });
    match kind {
        0 => {
            if current.is_some() {
                app.run(Command::SetPageBackground {
                    page,
                    background: None,
                });
            }
        }
        1 => {
            let mut c = match &current {
                Some(Fill::Solid(c)) => *c,
                _ => Color::WHITE,
            };
            let [r, g, b] = c.to_rgb8();
            let mut rgb = [r, g, b];
            let changed = ui.color_edit_button_srgb(&mut rgb).changed();
            if changed || !matches!(current, Some(Fill::Solid(_))) {
                c = Color::rgb8(rgb[0], rgb[1], rgb[2]);
                app.run(Command::SetPageBackground {
                    page,
                    background: Some(Fill::Solid(c)),
                });
            }
        }
        _ => {
            if ui.button(tr("options.choose_bitmap")).clicked() {
                crate::files::Dialog::new()
                    .add_filter(
                        "Images",
                        &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"],
                    )
                    .pick_file(app, move |app, p| app.set_page_background_image(page, &p));
            }
        }
    }
}

fn bleed_page(app: &mut App, ui: &mut Ui) {
    let mut md = app.doc().metadata.clone();
    grid("bleed").show(ui, |ui| {
        row_label(ui, &tr("options.bleed"));
        let mut b = app.units.from_mm(md.bleed);
        if ui
            .add_sized(
                [100.0, 22.0],
                egui::DragValue::new(&mut b)
                    .speed(0.1)
                    .max_decimals(3)
                    .suffix(format!(" {}", app.units.short())),
            )
            .changed()
        {
            md.bleed = app.units.to_mm(b).max(0.0);
            app.run(Command::SetMetadata { metadata: md });
        }
        ui.end_row();
    });
    ui.checkbox(&mut app.show_bleed, tr("options.show_bleed"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_belongs_to_one_dialog() {
        let sets = [
            OptionsSet::App,
            OptionsSet::Customization,
            OptionsSet::Tools,
            OptionsSet::Global,
            OptionsSet::Workspaces,
            OptionsSet::Document,
        ];
        let mut seen = Vec::new();
        for s in sets {
            for p in s.pages() {
                assert!(!seen.contains(p), "{p:?} twice");
                assert_eq!(p.set(), s);
                seen.push(*p);
            }
        }
        assert_eq!(OptionsPage::Rulers.set(), OptionsSet::Document);
        assert_eq!(OptionsPage::General.set(), OptionsSet::App);
    }

    #[test]
    fn cancel_puts_settings_and_document_back() {
        let mut app = App::headless();
        let snap = OptionsSnapshot::take(&app);
        app.settings.undo_levels = 3;
        app.show_grid = !app.show_grid;
        let mut meta = app.doc().metadata.clone();
        meta.fill_open_curves = true;
        app.run(Command::SetMetadata { metadata: meta });
        snap.restore(&mut app);
        assert_eq!(app.settings.undo_levels, 150);
        assert!(!app.doc().metadata.fill_open_curves);
    }

    #[test]
    fn every_page_draws() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for set in [
            OptionsSet::App,
            OptionsSet::Customization,
            OptionsSet::Tools,
            OptionsSet::Global,
            OptionsSet::Workspaces,
            OptionsSet::Document,
        ] {
            for p in set.pages() {
                app.options_page = *p;
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1600.0, 1000.0),
                    )),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    let mut close = false;
                    options_dialog(&mut app, ui.ctx(), &mut close);
                    assert!(!close, "{p:?} closed by itself");
                });
                out.textures_delta.clear();
            }
        }
        // Drawing a page never changes what it shows (fields clamp the
        // values they hold to their ranges).
        assert_eq!(app.star_sharpness, 0.5);
    }
}
