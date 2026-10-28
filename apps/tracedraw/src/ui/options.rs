//! Tools > Options (the application options and the Customization, Tools,
//! Global and Workspaces dialogs) and Layout > Document Options: one
//! window with a tab for each of them along the top (#633), a white page
//! list on the left, the page on
//! the right, the help button at the bottom left, OK and Cancel at the
//! bottom right. Cancel puts back what the dialog changed.

use crate::app::{App, Units};
use crate::i18n::{tr, trf};
use crate::settings::{AutoCenter, NodeShape, NodeSize, Startup};
use crate::theme::Tokens;
use crate::ui::chrome;
use crate::ui::tips::RichTip;
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
    Ai,
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
    AlignmentGuides,
    DynamicGuides,
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
    Curves,
    Rectangle,
    Ellipse,
    Polygon,
    Spiral,
    GraphPaper,
    Table,
    Eraser,
    // Tools > Options > Global.
    GlobalGeneral,
    Printing,
    FileFormats,
    BitmapTracing,
    Warnings,
    FileLocations,
    // Tools > Options > Workspaces.
    Workspace,
    // Tools > Options > AI.
    AiGeneral,
    AiService,
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
    /// Every set, in the order of the dialog's tabs.
    pub const ALL: [OptionsSet; 7] = [
        OptionsSet::App,
        OptionsSet::Customization,
        OptionsSet::Tools,
        OptionsSet::Global,
        OptionsSet::Workspaces,
        OptionsSet::Ai,
        OptionsSet::Document,
    ];

    /// The set's tab: the name of the menu item that opens it.
    fn tab_label(self) -> String {
        let key = match self {
            OptionsSet::App => "menu.tools.options_app",
            OptionsSet::Customization => "menu.tools.options_customization",
            OptionsSet::Tools => "menu.tools.options_tools",
            OptionsSet::Global => "menu.tools.options_global",
            OptionsSet::Workspaces => "menu.tools.options_workspaces",
            OptionsSet::Ai => "menu.tools.options_ai",
            OptionsSet::Document => "dialog.document_options",
        };
        tr(key)
            .trim_end_matches("...")
            .trim_end_matches('\u{2026}')
            .to_string()
    }

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
                AlignmentGuides,
                DynamicGuides,
                Save,
                Text,
            ],
            OptionsSet::Customization => &[Appearance, CommandBars, Commands, ColorPalette],
            OptionsSet::Tools => &[
                Pick, ZoomPan, Curves, Rectangle, Ellipse, Polygon, Spiral, GraphPaper, Table,
                Eraser,
            ],
            OptionsSet::Global => &[
                GlobalGeneral,
                Printing,
                FileFormats,
                BitmapTracing,
                Warnings,
                FileLocations,
            ],
            OptionsSet::Workspaces => &[Workspace],
            OptionsSet::Ai => &[AiGeneral, AiService],
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
            OptionsSet::Ai => tr("options.title_ai"),
            OptionsSet::Document => tr("dialog.document_options"),
        }
    }
}

impl OptionsPage {
    pub fn set(self) -> OptionsSet {
        OptionsSet::ALL
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
            AlignmentGuides => "options.alignment_guides",
            DynamicGuides => "options.dynamic_guides",
            Save => "options.save",
            Text => "options.text",
            Appearance => "options.appearance",
            Commands => "options.commands",
            CommandBars => "options.command_bars",
            ColorPalette => "options.color_palette",
            Pick => "tool.pick",
            ZoomPan => "options.zoom_pan",
            Curves => "options.curve_tools",
            Rectangle => "tool.rectangle",
            Ellipse => "tool.ellipse",
            Polygon => "tool.polygon",
            Spiral => "tool.spiral",
            GraphPaper => "tool.graph_paper",
            Table => "tool.table",
            Eraser => "tool.eraser",
            FileLocations => "options.file_locations",
            Printing => "options.printing",
            FileFormats => "options.file_formats",
            BitmapTracing => "options.bitmap_tracing",
            Warnings => "options.warnings",
            Workspace => "options.workspace",
            AiGeneral => "options.general",
            AiService => "options.ai_service",
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
    show_dockers: bool,
    docker_tab: crate::app::DockerTab,
    toolbox_last: Vec<crate::tools::Tool>,
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
    text_default_font: String,
    text_default_size: f64,
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
            show_dockers: app.show_dockers,
            docker_tab: app.docker_tab,
            toolbox_last: app.toolbox_last.clone(),
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
            text_default_font: app.text_default_font.clone(),
            text_default_size: app.text_default_size,
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
        // The workspace first: choosing one sets the bars, which the
        // snapshot then puts back as they were.
        if app.workspace != self.workspace {
            app.set_workspace(self.workspace);
        }
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
        app.text_default_font = self.text_default_font;
        app.text_default_size = self.text_default_size;
        app.text_hyphenation = self.text_hyphenation;
        app.show_non_printing = self.show_non_printing;
        app.show_dockers = self.show_dockers;
        app.docker_tab = self.docker_tab;
        app.toolbox_last = self.toolbox_last;
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
        egui::vec2(887.0, 663.0 + TABS_H),
        |ui| {
            let body = ui.max_rect();
            let tabs = egui::Rect::from_min_size(
                body.min + egui::vec2(14.0, 8.0),
                egui::vec2(859.0, TABS_H - 6.0),
            );
            set_tabs(app, ui, tabs, set);
            let top = 10.0 + TABS_H;
            let list = egui::Rect::from_min_size(
                body.min + egui::vec2(14.0, top),
                egui::vec2(225.0, 566.0),
            );
            let page = egui::Rect::from_min_size(
                body.min + egui::vec2(251.0, top),
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
                body.min + egui::vec2(14.0, top + 566.0 + 15.0),
                egui::vec2(859.0, 27.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(buttons), |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_sized([27.0, 27.0], egui::Button::new("?"))
                        .rich_tip(crate::ui::tips::keyed("dialog.help_button"))
                        .clicked()
                    {
                        app.open_help(crate::help::options_topic(app.options_page));
                    }
                    if set == OptionsSet::Document
                        && ui
                            .add_sized(
                                [150.0, 27.0],
                                egui::Button::new(tr("options.save_as_default")),
                            )
                            .rich_tip(crate::ui::tips::keyed("options.save_as_default"))
                            .clicked()
                    {
                        app.settings.document_defaults =
                            crate::settings::DocumentDefaults::of(app.doc());
                        app.status = tr("status.settings_saved");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if chrome::button(ui, &tr("dialog.cancel"))
                            .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                            .clicked()
                        {
                            cancel = true;
                        }
                        ui.add_space(5.0);
                        if chrome::button(ui, &tr("dialog.ok"))
                            .rich_tip(crate::ui::tips::keyed("dialog.ok"))
                            .clicked()
                        {
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

/// Height of the tab row over the page list.
const TABS_H: f32 = 32.0;

/// The tabs along the top: one per set of pages (Document Options while
/// a drawing is open), the current one selected. A tab opens the page of
/// its set shown last, else its first page (#633).
fn set_tabs(app: &mut App, ui: &mut Ui, rect: egui::Rect, set: OptionsSet) {
    let last = |s: OptionsSet| egui::Id::new(("options_last_page", s as u8));
    let page = app.options_page;
    ui.ctx().data_mut(|d| d.insert_temp(last(set), page));
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            for s in OptionsSet::ALL {
                if s == OptionsSet::Document && !app.has_document() {
                    continue;
                }
                let tab = ui
                    .selectable_label(s == set, s.tab_label())
                    .rich_tip_with(|| format!("{}\n{}", s.tab_label(), tr("options.set_tab_desc")));
                ui.ctx()
                    .data_mut(|d| d.insert_temp(tab_rect_id(s), tab.rect));
                if tab.clicked() && s != set {
                    app.options_page = ui
                        .ctx()
                        .data(|d| d.get_temp::<OptionsPage>(last(s)))
                        .filter(|p| p.set() == s)
                        .or_else(|| s.pages().first().copied())
                        .unwrap_or_default();
                }
            }
        });
    });
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() + 2.0,
        egui::Stroke::new(1.0, crate::theme::Tokens::border()),
    );
}

/// Where the tab of `set` was drawn last (for tests).
pub(crate) fn tab_rect_id(set: OptionsSet) -> egui::Id {
    egui::Id::new(("options_tab_rect", set as u8))
}

/// The page list: one row per page, the current one highlighted.
fn page_list(app: &mut App, ui: &mut Ui, rect: egui::Rect, set: OptionsSet) {
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, crate::theme::Tokens::field());
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, crate::theme::Tokens::border()),
        egui::StrokeKind::Inside,
    );
    for (i, p) in set.pages().iter().enumerate() {
        let row = egui::Rect::from_min_size(
            rect.min + egui::vec2(5.0, 5.0 + i as f32 * 23.0),
            egui::vec2(rect.width() - 10.0, 22.0),
        );
        let resp = ui
            .interact(
                row,
                egui::Id::new(("options_list", p.key(), i)),
                egui::Sense::click(),
            )
            .rich_tip_with(|| format!("{}\n{}", tr(p.key()), tr("options.page_desc")));
        let selected = app.options_page == *p;
        if selected {
            ui.painter()
                .rect_filled(row, 0.0, crate::theme::Tokens::selected_row());
            ui.painter().rect_stroke(
                row,
                0.0,
                egui::Stroke::new(1.0, crate::theme::Tokens::selected_row_line()),
                egui::StrokeKind::Inside,
            );
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(row, 0.0, crate::theme::Tokens::hovered_row());
        }
        ui.painter().text(
            row.left_center() + egui::vec2(5.0, 0.0),
            egui::Align2::LEFT_CENTER,
            tr(p.key()),
            egui::FontId::proportional(13.0),
            Tokens::text(),
        );
        if resp.clicked() {
            app.options_page = *p;
        }
    }
}

fn draw_page(app: &mut App, ui: &mut Ui) {
    use OptionsPage::*;
    // A command waits for its keys only while its page is shown.
    if !matches!(app.options_page, Commands) {
        app.shortcut_editor.capture = None;
    }
    match app.options_page {
        General => general_page(app, ui),
        Display => display_page(app, ui),
        Edit => edit_page(app, ui),
        NodesAndHandles => nodes_page(app, ui),
        ClipFrame => clip_frame_page(app, ui),
        Snapping => snapping_page(app, ui),
        AlignmentGuides => alignment_guides_page(app, ui),
        DynamicGuides => dynamic_guides_page(app, ui),
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
        Curves => curves_page(app, ui),
        GlobalGeneral => global_general_page(app, ui),
        Printing => crate::ui::options_global::printing_page(app, ui),
        FileFormats => crate::ui::options_global::file_formats_page(app, ui),
        BitmapTracing => crate::ui::options_global::bitmap_tracing_page(app, ui),
        Warnings => crate::ui::options_global::warnings_page(app, ui),
        FileLocations => file_locations_page(app, ui),
        Workspace => workspace_page(app, ui),
        AiGeneral => crate::ui::ai_ui::general_page(app, ui),
        AiService => crate::ui::ai_ui::service_page(app, ui),
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
pub(crate) fn row_label(ui: &mut Ui, text: &str) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(text);
    });
}

pub(crate) fn grid(id: &str) -> egui::Grid {
    egui::Grid::new(id).num_columns(2).spacing([8.0, 8.0])
}

pub(crate) fn combo<T: Copy + PartialEq>(
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
    let resp = ui.color_edit_button_srgb(rgb);
    // A frame, so a white colour shows on the white page.
    ui.painter().rect_stroke(
        resp.rect,
        0.0,
        egui::Stroke::new(1.0, Tokens::control_border()),
        egui::StrokeKind::Inside,
    );
    resp.changed()
}

// ----- TraceDraw options -------------------------------------------------

fn general_page(app: &mut App, ui: &mut Ui) {
    chrome::section(ui, &tr("options.getting_started"));
    ui.indent("gs", |ui| {
        grid("startup").show(ui, |ui| {
            row_label(ui, &trf("options.on_startup", &[("app", "TraceDraw")]));
            let mut s = app.settings.startup;
            let tip = crate::ui::tips::pair("options.startup", "options.startup_desc");
            let changed = crate::ui::tips::tip_over(ui, tip, |ui| {
                combo(ui, "startup", &mut s, &Startup::ALL, |v| tr(v.key()), 240.0)
            });
            if changed {
                app.settings.startup = s;
                app.settings.show_welcome_on_start = s == Startup::WelcomeScreen;
            }
            ui.end_row();
        });
        ui.checkbox(
            &mut app.settings.show_new_document_dialog,
            tr("options.show_new_document_dialog"),
        )
        .rich_tip(crate::ui::tips::keyed("options.show_new_document_dialog"));
        if !crate::files::WEB {
            ui.checkbox(&mut app.settings.show_splash, tr("options.show_splash"))
                .rich_tip(crate::ui::tips::keyed("options.show_splash"));
        }
        #[cfg(not(target_arch = "wasm32"))]
        hardware_acceleration(app, ui);
    });
    chrome::section(ui, &tr("options.undo_levels"));
    ui.indent("undo", |ui| {
        grid("undo_grid").show(ui, |ui| {
            row_label(ui, &tr("options.undo_regular"));
            ui.add_sized(
                [70.0, 22.0],
                crate::ui::field::NumField::new(&mut app.settings.undo_levels).range(1..=99_999),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.undo_levels",
                "options.undo_levels_desc",
            ));
            ui.end_row();
        });
    });
}

/// Use hardware acceleration and the graphics card (#445); they take
/// effect at the next start.
#[cfg(not(target_arch = "wasm32"))]
fn hardware_acceleration(app: &mut App, ui: &mut Ui) {
    let before = (
        app.settings.hardware_acceleration,
        app.settings.gpu_adapter.clone(),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.checkbox(
            &mut app.settings.hardware_acceleration,
            tr("options.hardware_acceleration"),
        )
        .rich_tip(crate::ui::tips::keyed("options.hardware_acceleration"));
        let on = app.settings.hardware_acceleration;
        let chosen = &mut app.settings.gpu_adapter;
        let shown = if chosen.is_empty() {
            tr("options.gpu_automatic")
        } else {
            chosen.clone()
        };
        ui.add_enabled_ui(on, |ui| {
            egui::ComboBox::from_id_salt("gpu_adapter")
                .width(260.0)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(chosen, String::new(), tr("options.gpu_automatic"))
                        .rich_tip(crate::ui::tips::pair(
                            "options.gpu_automatic",
                            "options.gpu_automatic_tip",
                        ));
                    for name in crate::gpu::adapters() {
                        ui.selectable_value(chosen, name.clone(), name);
                    }
                })
                .response
                .rich_tip_always(crate::ui::tips::keyed("options.gpu_adapter"));
        });
    });
    let dim = |ui: &mut Ui, text: String| {
        ui.label(
            egui::RichText::new(text)
                .color(Tokens::text_dim())
                .size(11.0),
        );
    };
    if let Some(name) = crate::gpu::in_use() {
        dim(ui, trf("options.gpu_in_use", &[("name", name)]));
    }
    dim(ui, tr("options.restart_to_apply"));
    // A new choice is tried afresh at the next start.
    if (
        app.settings.hardware_acceleration,
        app.settings.gpu_adapter.clone(),
    ) != before
    {
        app.settings.working_renderer.clear();
    }
}

fn display_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(&mut app.proof_colors, tr("menu.view.proof_colors"))
        .rich_tip(crate::ui::tips::pair(
            "menu.view.proof_colors",
            "menu.view.proof_colors_tip",
        ));
    chrome::section(ui, &tr("options.other_controls"));
    ui.indent("oc", |ui| {
        ui.checkbox(&mut app.settings.show_tooltips, tr("options.show_tooltips"))
            .rich_tip(crate::ui::tips::pair(
                "options.show_tooltips",
                "options.show_tooltips_tip",
            ));
        ui.checkbox(
            &mut app.settings.hide_bbox_curve_tools,
            tr("options.hide_bbox_curve_tools"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.hide_bbox_curve_tools",
            "options.hide_bbox_curve_tools_tip",
        ));
        ui.checkbox(&mut app.settings.node_tracking, tr("options.node_tracking"))
            .rich_tip(crate::ui::tips::pair(
                "options.node_tracking",
                "options.node_tracking_tip",
            ));
    });
    chrome::section(ui, &tr("options.full_screen_preview"));
    ui.indent("fsp", |ui| {
        ui.checkbox(
            &mut app.settings.preview_page_border,
            tr("options.show_page_border"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.show_page_border",
            "options.show_page_border_tip",
        ));
    });
    ui.separator();
    grid("display_wheel").show(ui, |ui| {
        row_label(ui, &tr("options.wheel_default_action"));
        let mut zoom = app.settings.wheel_zooms;
        let tip = crate::ui::tips::pair(
            "options.wheel_default_action",
            "options.wheel_default_action_tip",
        );
        crate::ui::tips::tip_over(ui, tip, |ui| {
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
        });
        app.settings.wheel_zooms = zoom;
        ui.end_row();
        row_label(ui, &tr("options.open_view"));
        let mut view = app.settings.open_view;
        let tip = crate::ui::tips::pair("options.open_view", "options.open_view_tip");
        crate::ui::tips::tip_over(ui, tip, |ui| {
            combo(
                ui,
                "open_view",
                &mut view,
                &crate::settings::OpenView::ALL,
                |v| tr(v.key()),
                160.0,
            );
        });
        app.settings.open_view = view;
        ui.end_row();
    });
    chrome::section(ui, &tr("options.ruler_calibration"));
    ui.label(
        egui::RichText::new(tr("options.calibrate_help"))
            .color(Tokens::text_dim())
            .size(12.0),
    );
    ui.horizontal(|ui| {
        ui.label(tr("options.screen_resolution"));
        let mut ppi = crate::settings::screen_ppi(&app.settings);
        if ui
            .add_sized(
                [80.0, 22.0],
                crate::ui::field::NumField::new(&mut ppi)
                    .range(48.0..=400.0)
                    .speed(0.1)
                    .max_decimals(1),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.screen_resolution",
                "options.screen_resolution_tip",
            ))
            .changed()
        {
            app.settings.screen_ppi = ppi;
        }
        ui.label(tr("options.points_per_inch"));
        if ui
            .button(tr("options.reset"))
            .rich_tip(crate::ui::tips::pair("options.reset", "dialog.reset_desc"))
            .clicked()
        {
            app.settings.screen_ppi = 96.0;
        }
    });
    calibration_rulers(ui, crate::settings::screen_points_per_inch(&app.settings));
}

/// Two rulers drawn at the calibrated resolution, 100 mm and 4 inches
/// long, to hold a real ruler against.
fn calibration_rulers(ui: &mut Ui, ppi: f32) {
    let per_mm = ppi / 25.4;
    let pad = 8.0;
    // Room at the right end for the unit's name.
    let width = 100.0 * per_mm + pad + 30.0;
    for metric in [true, false] {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 36.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect(
            rect,
            0.0,
            crate::theme::Tokens::field(),
            egui::Stroke::new(1.0, crate::theme::Tokens::control_border()),
            egui::StrokeKind::Inside,
        );
        let x0 = rect.left() + pad;
        let top = rect.top() + 1.0;
        let font = egui::FontId::proportional(10.0);
        let (count, step, label_every, unit) = if metric {
            (100, per_mm, 10, "mm")
        } else {
            // Eighths of an inch.
            (32, ppi / 8.0, 8, "in")
        };
        for i in 0..=count {
            let x = (x0 + i as f32 * step).round() + 0.5;
            let long = i % label_every == 0;
            let mid = if metric { i % 5 == 0 } else { i % 4 == 0 };
            let h = if long {
                14.0
            } else if mid {
                9.0
            } else {
                5.0
            };
            p.line_segment(
                [egui::pos2(x, top), egui::pos2(x, top + h)],
                egui::Stroke::new(1.0, Tokens::text()),
            );
            if long {
                let n = if metric { i } else { i / 8 };
                p.text(
                    egui::pos2(x, top + 16.0),
                    egui::Align2::CENTER_TOP,
                    n.to_string(),
                    font.clone(),
                    Tokens::text(),
                );
            }
        }
        p.text(
            egui::pos2(rect.right() - 4.0, rect.bottom() - 3.0),
            egui::Align2::RIGHT_BOTTOM,
            unit,
            font,
            Tokens::text_dim(),
        );
        ui.add_space(4.0);
    }
}

fn edit_page(app: &mut App, ui: &mut Ui) {
    grid("edit").show(ui, |ui| {
        row_label(ui, &tr("options.constrain_angle"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.constrain_angle", "options.constrain_angle_tip"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [80.0, 22.0],
                        crate::ui::field::NumField::new(&mut app.settings.constrain_angle)
                            .range(0.1..=90.0)
                            .max_decimals(1)
                            .speed(0.5),
                    );
                    ui.label(tr("options.degrees"));
                });
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.drawing_precision"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.drawing_precision", "options.drawing_precision_tip"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [80.0, 22.0],
                        crate::ui::field::NumField::new(&mut app.settings.precision).range(0..=10),
                    );
                    ui.label(tr("options.decimal_places"));
                });
            },
        );

        ui.end_row();
    });
    // Focus mode.
    crate::ui::chrome::section(ui, &tr("options.focus_inactive"));
    ui.horizontal(|ui| {
        let mut v = app.settings.focus_visibility as f32;
        if ui
            .add(
                crate::ui::Slider::new(&mut v, 0.0..=100.0)
                    .suffix(" %")
                    .integer(),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.focus_inactive",
                "options.focus_inactive_tip",
            ))
            .changed()
        {
            app.settings.focus_visibility = v.round() as u8;
        }
        if ui
            .button(tr("dialog.reset"))
            .rich_tip(crate::ui::tips::keyed("dialog.reset"))
            .clicked()
        {
            app.settings.focus_visibility = 20;
        }
    });
    ui.label(
        egui::RichText::new(tr("options.focus_inactive_hint"))
            .color(crate::theme::Tokens::text_dim()),
    );
    ui.add_space(6.0);
    ui.checkbox(
        &mut app.settings.focus_double_click,
        tr("options.focus_double_click"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.focus_double_click",
        "options.focus_double_click_tip",
    ));
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
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.node_size", "options.node_size_tip"),
            |ui| {
                combo(
                    ui,
                    "node_size",
                    &mut n.size,
                    &NodeSize::ALL,
                    |v| tr(v.key()),
                    100.0,
                );
            },
        );

        ui.end_row();
    });
    chrome::section(ui, &tr("options.node_shape"));
    grid("node_shapes").show(ui, |ui| {
        row_label(ui, &tr("options.node_cusp"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.node_cusp", "options.node_cusp_tip"),
            |ui| {
                combo(ui, "cusp", &mut n.cusp, &NodeShape::ALL, shape_label, 50.0);
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.node_smooth"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.node_smooth", "options.node_smooth_tip"),
            |ui| {
                combo(
                    ui,
                    "smooth",
                    &mut n.smooth,
                    &NodeShape::ALL,
                    shape_label,
                    50.0,
                );
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.node_symmetrical"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.node_symmetrical", "options.node_symmetrical_tip"),
            |ui| {
                combo(
                    ui,
                    "symmetrical",
                    &mut n.symmetrical,
                    &NodeShape::ALL,
                    shape_label,
                    50.0,
                );
            },
        );

        ui.end_row();
        ui.label("");
        if chrome::button(ui, &tr("options.reset"))
            .rich_tip(crate::ui::tips::pair("options.reset", "dialog.reset_desc"))
            .clicked()
        {
            let d = crate::settings::NodePrefs::default();
            n.cusp = d.cusp;
            n.smooth = d.smooth;
            n.symmetrical = d.symmetrical;
        }
        ui.end_row();
    });
    ui.checkbox(&mut n.show_direction, tr("options.show_curve_direction"))
        .rich_tip(crate::ui::tips::pair(
            "options.show_curve_direction",
            "options.show_curve_direction_tip",
        ));
    ui.checkbox(&mut n.corner_widgets, tr("options.corner_widgets"))
        .rich_tip(tr("options.corner_widgets_tip"));
    ui.horizontal(|ui| {
        ui.label(tr("options.node_pick_distance"));
        ui.add(
            crate::ui::field::NumField::new(&mut n.pick_distance)
                .range(2.0..=20.0)
                .speed(0.2)
                .max_decimals(0)
                .suffix(" px"),
        )
        .rich_tip(tr("options.node_pick_distance_tip"));
    });
    chrome::section(ui, &tr("options.colors"));
    grid("node_colors").show(ui, |ui| {
        row_label(ui, &tr("options.main_color"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.main_color", "options.main_color_tip"),
            |ui| {
                ui.horizontal(|ui| {
                    rgb_button(ui, &mut n.main_rgb);
                    if ui
                        .button(tr("options.reset"))
                        .rich_tip(crate::ui::tips::pair("options.reset", "dialog.reset_desc"))
                        .clicked()
                    {
                        n.main_rgb = crate::settings::NodePrefs::default().main_rgb;
                    }
                });
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.secondary_color"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.secondary_color", "options.secondary_color_tip"),
            |ui| {
                ui.horizontal(|ui| {
                    rgb_button(ui, &mut n.secondary_rgb);
                    if ui
                        .button(tr("options.reset"))
                        .rich_tip(crate::ui::tips::pair("options.reset", "dialog.reset_desc"))
                        .clicked()
                    {
                        n.secondary_rgb = crate::settings::NodePrefs::default().secondary_rgb;
                    }
                });
            },
        );

        ui.end_row();
    });
    ui.checkbox(
        &mut n.unselected_filled,
        tr("options.unselected_nodes_filled"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.unselected_nodes_filled",
        "options.unselected_nodes_filled_tip",
    ));
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
            ui.radio_value(&mut p.auto_center, v, tr(key))
                .rich_tip_with(|| format!("{}\n{}", tr(key), tr(&format!("{key}_tip"))));
        }
    });
    ui.separator();
    ui.checkbox(&mut p.empty_lines, tr("options.empty_frame_lines"))
        .rich_tip(crate::ui::tips::pair(
            "options.empty_frame_lines",
            "options.empty_frame_lines_tip",
        ));
}

fn snapping_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(&mut app.snap.objects, tr("options.snap_to_objects"))
        .rich_tip(crate::ui::tips::pair(
            "options.snap_to_objects",
            "options.snap_to_objects_tip",
        ));
    ui.checkbox(&mut app.snap.page, tr("options.snap_to_page"))
        .rich_tip(crate::ui::tips::pair(
            "options.snap_to_page",
            "options.snap_to_page_tip",
        ));
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(tr("options.snapping_radius"));
        ui.add_sized(
            [70.0, 22.0],
            crate::ui::field::NumField::new(&mut app.settings.snap.threshold_px).range(1.0..=100.0),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.snapping_radius",
            "options.snapping_radius_tip",
        ));
        ui.label(tr("options.pixels"));
    });
    ui.checkbox(&mut app.snap.guides, tr("options.snap_to_guidelines"))
        .rich_tip(crate::ui::tips::pair(
            "options.snap_to_guidelines",
            "options.snap_to_guidelines_tip",
        ));
    ui.checkbox(&mut app.snap.grid, tr("options.snap_to_grid"))
        .rich_tip(crate::ui::tips::pair(
            "options.snap_to_grid",
            "options.snap_to_grid_tip",
        ));
    ui.checkbox(
        &mut app.snap.baseline_grid,
        tr("menu.view.snap_baseline_grid"),
    )
    .rich_tip(crate::ui::tips::pair(
        "menu.view.snap_baseline_grid",
        "menu.view.snap_baseline_grid_tip",
    ));
    ui.checkbox(&mut app.snap.pixels, tr("options.snap_to_pixels"))
        .rich_tip(crate::ui::tips::pair(
            "options.snap_to_pixels",
            "options.snap_to_pixels_tip",
        ));
    ui.checkbox(
        &mut app.settings.snap.show_marks,
        tr("options.show_snap_marks"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.show_snap_marks",
        "options.show_snap_marks_tip",
    ));
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.add_enabled(
            app.settings.snap.show_marks,
            egui::Checkbox::new(
                &mut app.settings.snap.screen_tips,
                tr("options.snap_screen_tip"),
            ),
        )
        .rich_tip_always(crate::ui::tips::pair(
            "options.snap_screen_tip",
            "options.snap_screen_tip_desc",
        ));
    });
    chrome::section(ui, &tr("options.snap_modes"));
    ui.indent("snap_modes", |ui| {
        let modes = &mut app.settings.snap.modes;
        for m in crate::snap_points::SnapMode::ALL {
            ui.horizontal(|ui| {
                let mut on = modes.get(m);
                if ui
                    .checkbox(&mut on, "")
                    .rich_tip_with(|| format!("{}\n{}", tr(m.key()), tr("options.snap_mode_tip")))
                    .changed()
                {
                    modes.set(m, on);
                }
                let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                crate::canvas::draw_snap_glyph(ui.painter(), r.center(), m, Tokens::icon());
                ui.label(tr(m.key()));
            });
        }
        ui.horizontal(|ui| {
            if ui
                .button(tr("options.select_all"))
                .rich_tip(crate::ui::tips::pair(
                    "options.select_all",
                    "options.select_all_tip",
                ))
                .clicked()
            {
                *modes = crate::snap_points::SnapModes::default();
            }
            if ui
                .button(tr("options.deselect_all"))
                .rich_tip(crate::ui::tips::pair(
                    "options.deselect_all",
                    "options.deselect_all_tip",
                ))
                .clicked()
            {
                *modes = crate::snap_points::SnapModes::none();
            }
        });
    });
}

/// A length typed in the document's units, kept in millimetres.
fn length_field(ui: &mut Ui, units: Units, mm: &mut f64, max_mm: f64) {
    let mut v = units.world_from_mm(*mm);
    let resp = ui.add_sized(
        [80.0, 22.0],
        crate::ui::field::NumField::new(&mut v)
            .range(0.0..=units.world_from_mm(max_mm))
            .max_decimals(3)
            .speed(0.1),
    );
    if resp.changed() && v.is_finite() {
        *mm = units.world_to_mm(v).clamp(0.0, max_mm);
    }
    ui.label(units.short());
}

/// The colour and line style of a kind of guide.
fn guide_look(
    ui: &mut Ui,
    id: &str,
    color: &mut [u8; 3],
    style: &mut crate::live_guides::GuideStyle,
) {
    grid(id).show(ui, |ui| {
        row_label(ui, &tr("options.guide_color"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.guide_color", "options.guide_color_tip"),
            |ui| {
                rgb_button(ui, color);
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.guide_style"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.guide_style", "options.guide_style_tip"),
            |ui| {
                combo(
                    ui,
                    &format!("{id}_style"),
                    style,
                    &crate::live_guides::GuideStyle::ALL,
                    |v| tr(v.key()),
                    140.0,
                );
            },
        );

        ui.end_row();
    });
}

/// Options > Alignment Guides: what lines up with what while objects are
/// moved, drawn or resized.
fn alignment_guides_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(
        &mut app.snap.alignment_guides,
        tr("options.alignment_guides_on"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.alignment_guides_on",
        "options.alignment_guides_on_tip",
    ));
    let units = app.units;
    let on = app.snap.alignment_guides;
    let a = &mut app.settings.snap.align;
    ui.add_enabled_ui(on, |ui| {
        chrome::section(ui, &tr("options.align_moving"));
        ui.indent("align_moving", |ui| {
            ui.radio_value(
                &mut a.each_object,
                false,
                tr("options.align_selection_as_one"),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.align_selection_as_one",
                "options.align_selection_as_one_tip",
            ));
            ui.radio_value(&mut a.each_object, true, tr("options.align_each_object"))
                .rich_tip(crate::ui::tips::pair(
                    "options.align_each_object",
                    "options.align_each_object_tip",
                ));
        });
        chrome::section(ui, &tr("options.align_what"));
        ui.indent("align_what", |ui| {
            ui.checkbox(&mut a.edges, tr("options.align_edges"))
                .rich_tip(crate::ui::tips::pair(
                    "options.align_edges",
                    "options.align_edges_tip",
                ));
            ui.checkbox(&mut a.centers, tr("options.align_centers"))
                .rich_tip(crate::ui::tips::pair(
                    "options.align_centers",
                    "options.align_centers_tip",
                ));
            ui.checkbox(&mut a.page, tr("options.align_page"))
                .rich_tip(crate::ui::tips::pair(
                    "options.align_page",
                    "options.align_page_tip",
                ));
            ui.checkbox(&mut a.equal_spacing, tr("options.align_equal_spacing"))
                .rich_tip(crate::ui::tips::pair(
                    "options.align_equal_spacing",
                    "options.align_equal_spacing_tip",
                ));
            ui.checkbox(&mut a.equal_size, tr("options.align_equal_size"))
                .rich_tip(crate::ui::tips::pair(
                    "options.align_equal_size",
                    "options.align_equal_size_tip",
                ));
        });
        chrome::section(ui, &tr("options.align_margins"));
        ui.indent("align_margins", |ui| {
            ui.checkbox(&mut a.margins, tr("options.align_margins_on"))
                .rich_tip(crate::ui::tips::pair(
                    "options.align_margins_on",
                    "options.align_margins_on_tip",
                ));
            ui.add_enabled_ui(a.margins, |ui| {
                grid("align_margin_values").show(ui, |ui| {
                    row_label(ui, &tr("options.align_offset"));
                    crate::ui::tips::tip_over(
                        ui,
                        crate::ui::tips::pair("options.align_offset", "options.align_offset_tip"),
                        |ui| {
                            ui.horizontal(|ui| length_field(ui, units, &mut a.offset_mm, 1000.0));
                        },
                    );

                    ui.end_row();
                    row_label(ui, &tr("options.align_inset"));
                    crate::ui::tips::tip_over(
                        ui,
                        crate::ui::tips::pair("options.align_inset", "options.align_inset_tip"),
                        |ui| {
                            ui.horizontal(|ui| length_field(ui, units, &mut a.inset_mm, 1000.0));
                        },
                    );

                    ui.end_row();
                });
            });
        });
        chrome::section(ui, &tr("options.guide_look"));
        guide_look(ui, "align_look", &mut a.color, &mut a.style);
        if ui
            .button(tr("options.reset"))
            .rich_tip(crate::ui::tips::pair("options.reset", "dialog.reset_desc"))
            .clicked()
        {
            *a = crate::live_guides::AlignOptions::default();
        }
    });
}

/// The angles offered as check boxes on the Dynamic Guides page.
const DYNAMIC_PRESETS: [f64; 12] = [
    0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 105.0, 120.0, 135.0, 150.0, 165.0,
];

/// Options > Dynamic Guides: the angles of the guides from acquired snap
/// points, their tip and ticks.
fn dynamic_guides_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(
        &mut app.snap.dynamic_guides,
        tr("options.dynamic_guides_on"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.dynamic_guides_on",
        "options.dynamic_guides_on_tip",
    ));
    let units = app.units;
    let on = app.snap.dynamic_guides;
    let d = &mut app.settings.snap.dynamic;
    ui.add_enabled_ui(on, |ui| {
        chrome::section(ui, &tr("options.dynamic_angles"));
        ui.indent("dyn_angles", |ui| {
            let mut angles = d.clean_angles();
            egui::Grid::new("dyn_angle_grid")
                .num_columns(4)
                .show(ui, |ui| {
                    for (i, a) in DYNAMIC_PRESETS.iter().enumerate() {
                        let mut has = angles.iter().any(|b| (b - a).abs() < 1e-6);
                        if ui.checkbox(&mut has, format!("{a:.0}\u{00B0}")).changed() {
                            if has {
                                angles.push(*a);
                            } else {
                                angles.retain(|b| (b - a).abs() >= 1e-6);
                            }
                        }
                        if i % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
            // Other angles, each with a button that takes it away.
            let others: Vec<f64> = angles
                .iter()
                .copied()
                .filter(|a| !DYNAMIC_PRESETS.iter().any(|b| (b - a).abs() < 1e-6))
                .collect();
            if !others.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for a in others {
                        if ui
                            .button(format!("{a:.1}\u{00B0} \u{00D7}"))
                            .rich_tip(tr("options.dynamic_remove_angle"))
                            .clicked()
                        {
                            angles.retain(|b| (b - a).abs() >= 1e-6);
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                let id = egui::Id::new("dyn_new_angle");
                let mut v: f64 = ui.data_mut(|m| *m.get_temp_mut_or(id, 22.5));
                ui.add_sized(
                    [70.0, 22.0],
                    crate::ui::field::NumField::new(&mut v)
                        .range(0.0..=179.9)
                        .max_decimals(1),
                )
                .rich_tip(crate::ui::tips::pair(
                    "options.dynamic_new_angle",
                    "options.dynamic_new_angle_tip",
                ));
                ui.label(tr("options.degrees"));
                if ui
                    .button(tr("options.dynamic_add_angle"))
                    .rich_tip(crate::ui::tips::pair(
                        "options.dynamic_add_angle",
                        "options.dynamic_add_angle_tip",
                    ))
                    .clicked()
                    && v.is_finite()
                {
                    angles.push(v);
                }
                ui.data_mut(|m| m.insert_temp(id, v));
            });
            angles.sort_by(|a, b| a.total_cmp(b));
            if angles != d.angles {
                d.angles = angles;
            }
        });
        chrome::section(ui, &tr("options.dynamic_display"));
        ui.indent("dyn_display", |ui| {
            ui.checkbox(&mut d.tooltip, tr("options.dynamic_tooltip"))
                .rich_tip(crate::ui::tips::pair(
                    "options.dynamic_tooltip",
                    "options.dynamic_tooltip_tip",
                ));
            ui.checkbox(&mut d.ticks, tr("options.dynamic_ticks"))
                .rich_tip(crate::ui::tips::pair(
                    "options.dynamic_ticks",
                    "options.dynamic_ticks_tip",
                ));
            ui.add_enabled_ui(d.ticks, |ui| {
                grid("dyn_ticks").show(ui, |ui| {
                    row_label(ui, &tr("options.dynamic_tick_spacing"));
                    crate::ui::tips::tip_over(
                        ui,
                        crate::ui::tips::pair(
                            "options.dynamic_tick_spacing",
                            "options.dynamic_tick_spacing_tip",
                        ),
                        |ui| {
                            ui.horizontal(|ui| length_field(ui, units, &mut d.tick_mm, 10_000.0));
                        },
                    );

                    ui.end_row();
                });
                ui.checkbox(&mut d.snap_ticks, tr("options.dynamic_snap_ticks"))
                    .rich_tip(crate::ui::tips::pair(
                        "options.dynamic_snap_ticks",
                        "options.dynamic_snap_ticks_tip",
                    ));
            });
        });
        chrome::section(ui, &tr("options.guide_look"));
        guide_look(ui, "dyn_look", &mut d.color, &mut d.style);
        if ui
            .button(tr("options.reset"))
            .rich_tip(crate::ui::tips::pair("options.reset", "dialog.reset_desc"))
            .clicked()
        {
            *d = crate::live_guides::DynamicOptions::default();
        }
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
        )
        .rich_tip_always(crate::ui::tips::pair(
            "options.folder",
            "options.folder_tip",
        ));
        if ui
            .add_enabled(!crate::files::WEB, egui::Button::new(tr("options.browse")))
            .rich_tip(crate::ui::tips::pair(
                "options.browse",
                "options.browse_tip",
            ))
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
        ui.checkbox(&mut b.before_save, tr("options.backup_before_save"))
            .rich_tip(crate::ui::tips::pair(
                "options.backup_before_save",
                "options.backup_before_save_tip",
            ));
        ui.add_enabled_ui(b.before_save, |ui| {
            let mut choose = b.before_save_dir.is_some();
            ui.horizontal(|ui| {
                ui.label(tr("options.backup_to"));
                ui.vertical(|ui| {
                    ui.radio_value(&mut choose, false, tr("options.same_folder"))
                        .rich_tip(crate::ui::tips::pair(
                            "options.same_folder",
                            "options.same_folder_tip",
                        ));
                    ui.radio_value(&mut choose, true, tr("options.choose_location"))
                        .rich_tip(crate::ui::tips::pair(
                            "options.choose_location",
                            "options.choose_location_tip",
                        ));
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
            ui.checkbox(&mut b.auto, tr("options.backup_every"))
                .rich_tip(crate::ui::tips::pair(
                    "options.backup_every",
                    "options.backup_every_tip",
                ));
            ui.add_enabled(
                b.auto,
                crate::ui::field::NumField::new(&mut b.minutes).range(1..=999),
            )
            .rich_tip_always(crate::ui::tips::pair(
                "options.backup_every",
                "options.backup_minutes_tip",
            ));
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

/// The tabs of Options > Text, in order.
pub const TEXT_TABS: [&str; 5] = [
    "options.display",
    "options.paragraph_text",
    "docker.fonts",
    "options.spelling",
    "dialog.autocorrect",
];

/// Options > Text > Autocorrect's tab.
pub const AUTOCORRECT_TAB: usize = 4;

/// Options > Text: Display, Paragraph Text, Fonts, Spelling and
/// Autocorrect tabs.
fn text_page(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        for (i, key) in TEXT_TABS.iter().enumerate() {
            if ui
                .selectable_label(app.options_text_tab == i, tr(key))
                .rich_tip_with(|| format!("{}\n{}", tr(key), tr("options.set_tab_desc")))
                .clicked()
            {
                app.options_text_tab = i;
            }
        }
    });
    ui.separator();
    match app.options_text_tab {
        0 => text_display_tab(app, ui),
        1 => paragraph_text_tab(app, ui),
        2 => fonts_tab(app, ui),
        3 => spelling_tab(app, ui),
        _ => autocorrect_tab(app, ui),
    }
}

fn text_display_tab(app: &mut App, ui: &mut Ui) {
    grid("text_display").show(ui, |ui| {
        row_label(ui, &tr("options.keyboard_text_increment"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair(
                "options.keyboard_text_increment",
                "options.keyboard_text_increment_tip",
            ),
            |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [70.0, 22.0],
                        crate::ui::field::NumField::new(&mut app.settings.text_increment_pt)
                            .range(0.1..=100.0)
                            .max_decimals(1),
                    );
                    ui.label(tr("options.points"));
                });
            },
        );

        ui.end_row();
        // The font and size of new text, whatever text is selected (#449).
        // The unit of font sizes everywhere (#529): points, or millimetres
        // and the like for large work.
        row_label(ui, &tr("options.text_units"));
        let u = app.text_units();
        let tip = crate::ui::tips::pair("options.text_units", "options.text_units_tip");
        crate::ui::tips::tip_over(ui, tip, |ui| {
            egui::ComboBox::from_id_salt("options_text_units")
                .selected_text(u.label())
                .width(160.0)
                .show_ui(ui, |ui| {
                    for v in Units::ALL {
                        if ui.selectable_label(v == u, v.label()).clicked() {
                            app.settings.text_units = v.id().to_string();
                        }
                    }
                });
        });
        ui.end_row();
        row_label(ui, &tr("options.default_font"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.default_font", "options.default_font_tip"),
            |ui| {
                crate::ui::font_list::default_font_list(app, ui, "options_default_font", 220.0);
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.default_size"));
        let u = app.text_units();
        let mut shown = u.from_pt(app.text_default_size);
        if ui
            .add(
                crate::ui::field::NumField::new(&mut shown)
                    .range(u.from_pt(1.0)..=u.from_pt(999.0))
                    .max_decimals(if u == Units::Points { 1 } else { 3 })
                    .unit_mm(u.mm())
                    .suffix(format!(" {}", u.short())),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.default_size",
                "options.default_size_tip",
            ))
            .changed()
        {
            app.text_default_size = u.to_pt(shown);
            if !app.has_text_target() {
                app.text_size_pt = app.text_default_size;
            }
        }
        ui.end_row();
    });
    ui.checkbox(
        &mut app.show_non_printing,
        tr("menu.text.show_non_printing"),
    )
    .rich_tip(crate::ui::tips::pair(
        "menu.text.show_non_printing",
        "menu.text.show_non_printing_tip",
    ));
}

fn paragraph_text_tab(app: &mut App, ui: &mut Ui) {
    ui.checkbox(
        &mut app.settings.show_text_frames,
        tr("menu.text.show_text_frames"),
    )
    .rich_tip(crate::ui::tips::pair(
        "menu.text.show_text_frames",
        "menu.text.show_text_frames_tip",
    ));
    ui.checkbox(&mut app.text_hyphenation, tr("menu.text.use_hyphenation"))
        .rich_tip(crate::ui::tips::pair(
            "menu.text.use_hyphenation",
            "menu.text.use_hyphenation_tip",
        ));
    // The pattern files, read when the tab shows a new folder.
    type Found = (
        Option<std::path::PathBuf>,
        Vec<crate::spell::DictionaryFile>,
    );
    let cache = ui.id().with("hyphenation_found");
    let sp = &mut app.settings.spelling;
    let key = sp.dictionary_folder.clone();
    let mut found: Option<Found> = ui.data(|d| d.get_temp(cache));
    if found.as_ref().is_none_or(|(k, _)| *k != key) {
        found = Some((key.clone(), crate::spell::find_hyphenation(key.as_deref())));
    }
    let list = found.as_ref().map(|f| f.1.clone()).unwrap_or_default();
    let shown = |name: &str| {
        if name.is_empty() {
            tr("options.hyphenation_builtin")
        } else {
            name.strip_prefix("hyph_").unwrap_or(name).to_string()
        }
    };
    grid("hyphenation_patterns").show(ui, |ui| {
        row_label(ui, &tr("options.hyphenation_patterns"));
        egui::ComboBox::from_id_salt("hyphenation_patterns_combo")
            .width(220.0)
            .selected_text(shown(&sp.hyphenation_patterns))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut sp.hyphenation_patterns, String::new(), shown(""));
                for d in &list {
                    ui.selectable_value(
                        &mut sp.hyphenation_patterns,
                        d.name.clone(),
                        shown(&d.name),
                    )
                    .rich_tip(d.dic.display().to_string());
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "options.hyphenation_patterns",
                "options.hyphenation_patterns_tip",
            ));
        ui.end_row();
    });
    ui.label(egui::RichText::new(tr("options.hyphenation_note")).color(Tokens::text_dim()));
    if let Some(f) = found {
        ui.data_mut(|d| d.insert_temp(cache, f));
    }
}

fn fonts_tab(app: &mut App, ui: &mut Ui) {
    ui.checkbox(
        &mut app.settings.show_recent_fonts,
        tr("options.show_recent_fonts"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.show_recent_fonts",
        "options.show_recent_fonts_tip",
    ));
    ui.add_enabled_ui(app.settings.show_recent_fonts, |ui| {
        ui.horizontal(|ui| {
            ui.label(tr("options.recent_font_count"));
            ui.add(
                crate::ui::field::NumField::new(&mut app.settings.recent_font_count)
                    .range(1..=crate::ui::font_list::RECENT_MAX),
            )
            .rich_tip_always(crate::ui::tips::pair(
                "options.recent_font_count",
                "options.recent_font_count_tip",
            ));
        });
    });
    ui.horizontal(|ui| {
        ui.label(tr("options.font_sample_size"));
        ui.add(
            crate::ui::field::NumField::new(&mut app.settings.font_sample_px)
                .range(crate::ui::font_list::SAMPLE_SIZES)
                .max_decimals(0)
                .suffix(" pt"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.font_sample_size",
            "options.font_sample_size_tip",
        ));
    });
}

fn spelling_tab(app: &mut App, ui: &mut Ui) {
    let p = &mut app.settings.spelling;
    ui.checkbox(&mut p.ignore_digits, tr("writing.ignore_digits"))
        .rich_tip(crate::ui::tips::pair(
            "writing.ignore_digits",
            "writing.ignore_digits_tip",
        ));
    ui.checkbox(&mut p.ignore_capitals, tr("writing.ignore_capitals"))
        .rich_tip(crate::ui::tips::pair(
            "writing.ignore_capitals",
            "writing.ignore_capitals_tip",
        ));
    ui.checkbox(&mut p.add_to_autocorrect, tr("options.add_to_autocorrect"))
        .rich_tip(crate::ui::tips::pair(
            "options.add_to_autocorrect",
            "options.add_to_autocorrect_tip",
        ));
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("options.spelling_note")).color(Tokens::text_dim()));
    chrome::section(ui, &tr("options.dictionaries"));
    // The folders are read when the tab shows a new folder or on Refresh,
    // not every frame.
    type Found = (
        Option<std::path::PathBuf>,
        Vec<crate::spell::DictionaryFile>,
    );
    let cache = ui.id().with("dictionaries_found");
    let key = p.dictionary_folder.clone();
    let mut found: Option<Found> = ui.data(|d| d.get_temp(cache));
    if found.as_ref().is_none_or(|(k, _)| *k != key) {
        found = Some((key.clone(), crate::spell::find_dictionaries(key.as_deref())));
    }
    let list = found.as_ref().map(|f| f.1.clone()).unwrap_or_default();
    egui::Frame::new()
        .fill(Tokens::field())
        .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("dictionaries_list")
                .max_height(120.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if list.is_empty() {
                        ui.label(
                            egui::RichText::new(tr("options.no_dictionaries"))
                                .color(Tokens::text_dim()),
                        );
                    }
                    for d in &list {
                        let mut on = !p.disabled_dictionaries.contains(&d.name);
                        let label = if d.aff.is_some() {
                            d.name.clone()
                        } else {
                            format!("{}  ({})", d.name, tr("options.word_list"))
                        };
                        if ui
                            .checkbox(&mut on, label)
                            .rich_tip(d.dic.display().to_string())
                            .changed()
                        {
                            p.disabled_dictionaries.retain(|n| *n != d.name);
                            if !on {
                                p.disabled_dictionaries.push(d.name.clone());
                            }
                        }
                    }
                });
        });
    let own = crate::spell::own_folder();
    grid("dictionary_folders").show(ui, |ui| {
        row_label(ui, &tr("options.dictionaries_folder"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair(
                "options.dictionaries_folder",
                "options.dictionaries_folder_tip",
            ),
            |ui| {
                ui.horizontal(|ui| {
                    let mut text = own
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    ui.add_enabled(
                        false,
                        egui::TextEdit::singleline(&mut text).desired_width(320.0),
                    );
                    if ui
                        .add_enabled(
                            own.is_some() && !crate::files::WEB,
                            egui::Button::new(tr("options.open_folder")),
                        )
                        .clicked()
                    {
                        if let Some(dir) = &own {
                            let _ = std::fs::create_dir_all(dir);
                            if let Err(e) = crate::export::open_with_system(dir) {
                                app.status = e;
                            }
                        }
                    }
                });
            },
        );

        ui.end_row();
        let p = &mut app.settings.spelling;
        row_label(ui, &tr("options.more_dictionaries"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.more_dictionaries", "options.more_dictionaries_tip"),
            |ui| {
                ui.horizontal(|ui| {
                    path_field(ui, &mut p.dictionary_folder, String::new());
                    if ui
                        .add_enabled(
                            p.dictionary_folder.is_some(),
                            egui::Button::new(tr("options.clear_folder")),
                        )
                        .clicked()
                    {
                        p.dictionary_folder = None;
                    }
                });
            },
        );

        ui.end_row();
    });
    if ui
        .button(tr("options.refresh_dictionaries"))
        .rich_tip(crate::ui::tips::pair(
            "options.refresh_dictionaries",
            "options.refresh_dictionaries_tip",
        ))
        .clicked()
    {
        found = None;
        crate::spell::reload();
        let hyph = ui.id().with("hyphenation_found");
        ui.data_mut(|d| d.remove::<Found>(hyph));
    }
    ui.data_mut(|d| match found {
        Some(f) => {
            d.insert_temp(cache, f);
        }
        None => {
            d.remove::<Found>(cache);
        }
    });
}

/// A quotation mark list (one of [`crate::autocorrect::QUOTE_CHOICES`]).
fn quote_combo(ui: &mut Ui, id: &str, tip: String, value: &mut char) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .width(48.0)
        .selected_text(value.to_string())
        .show_ui(ui, |ui| {
            for q in crate::autocorrect::QUOTE_CHOICES {
                changed |= ui.selectable_value(value, q, q.to_string()).changed();
            }
        })
        .response
        .rich_tip(tip);
    changed
}

fn autocorrect_tab(app: &mut App, ui: &mut Ui) {
    let style = app.settings.autocorrect.style();
    let p = &mut app.settings.autocorrect;
    ui.checkbox(&mut p.capitalize_sentences, tr("autocorrect.capitalize"))
        .rich_tip(crate::ui::tips::pair(
            "autocorrect.capitalize",
            "autocorrect.capitalize_tip",
        ));
    ui.checkbox(
        &mut p.fix_two_initial_capitals,
        tr("autocorrect.two_capitals"),
    )
    .rich_tip(crate::ui::tips::pair(
        "autocorrect.two_capitals",
        "autocorrect.two_capitals_tip",
    ));
    ui.checkbox(&mut p.capitalize_days, tr("autocorrect.days"))
        .rich_tip(crate::ui::tips::pair(
            "autocorrect.days",
            "autocorrect.days_tip",
        ));
    ui.checkbox(&mut p.auto_hyperlinks, tr("autocorrect.hyperlinks"))
        .rich_tip(crate::ui::tips::pair(
            "autocorrect.hyperlinks",
            "autocorrect.hyperlinks_tip",
        ));
    ui.separator();
    grid("autocorrect_language").show(ui, |ui| {
        row_label(ui, &tr("autocorrect.language"));
        let shown = crate::i18n::LANGUAGES
            .iter()
            .find(|(code, _)| *code == p.quote_language)
            .map(|(_, name)| name.to_string())
            .unwrap_or_else(|| tr("autocorrect.interface_language"));
        let before = p.quote_language.clone();
        egui::ComboBox::from_id_salt("autocorrect_quote_language")
            .width(220.0)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut p.quote_language,
                    String::new(),
                    tr("autocorrect.interface_language"),
                )
                .rich_tip(crate::ui::tips::pair(
                    "autocorrect.interface_language",
                    "autocorrect.interface_language_tip",
                ));
                for (code, name) in crate::i18n::LANGUAGES {
                    ui.selectable_value(&mut p.quote_language, code.to_string(), name);
                }
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "autocorrect.language",
                "autocorrect.language_tip",
            ));
        // Another language brings its own quotation marks.
        if p.quote_language != before {
            p.single_quotes = None;
            p.double_quotes = None;
        }
        ui.end_row();
    });
    ui.checkbox(&mut p.typographic_quotes, tr("autocorrect.quotes"))
        .rich_tip(crate::ui::tips::pair(
            "autocorrect.quotes",
            "autocorrect.quotes_tip",
        ));
    ui.checkbox(
        &mut p.straight_after_numbers,
        tr("autocorrect.straight_after_numbers"),
    )
    .rich_tip(crate::ui::tips::pair(
        "autocorrect.straight_after_numbers",
        "autocorrect.straight_after_numbers_tip",
    ));
    let style = if p.quote_language.is_empty() {
        style
    } else {
        p.style()
    };
    ui.add_enabled_ui(p.typographic_quotes, |ui| {
        grid("autocorrect_quotes").show(ui, |ui| {
            for (label, single) in [
                ("autocorrect.single_quotes", true),
                ("autocorrect.double_quotes", false),
            ] {
                row_label(ui, &tr(label));
                let (mut o, mut c) = if single {
                    p.single(style)
                } else {
                    p.double(style)
                };
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label(tr("autocorrect.open"));
                    let tip = crate::ui::tips::pair("autocorrect.open", "autocorrect.open_tip");
                    changed |= quote_combo(ui, &format!("{label}_open"), tip, &mut o);
                    ui.add_space(12.0);
                    ui.label(tr("autocorrect.close"));
                    let tip = crate::ui::tips::pair("autocorrect.close", "autocorrect.close_tip");
                    changed |= quote_combo(ui, &format!("{label}_close"), tip, &mut c);
                });
                if changed {
                    if single {
                        p.single_quotes = Some([o, c]);
                    } else {
                        p.double_quotes = Some([o, c]);
                    }
                }
                ui.end_row();
            }
        });
    });
    ui.separator();
    ui.label(egui::RichText::new(tr("autocorrect.replacements")).strong());
    ui.checkbox(
        &mut p.replace_while_typing,
        tr("autocorrect.replace_while_typing"),
    )
    .rich_tip(crate::ui::tips::pair(
        "autocorrect.replace_while_typing",
        "autocorrect.replace_while_typing_tip",
    ));
    let from_id = ui.id().with("autocorrect_from");
    let to_id = ui.id().with("autocorrect_to");
    let row_id = ui.id().with("autocorrect_row");
    let mut from: String = ui.data(|d| d.get_temp(from_id)).unwrap_or_default();
    let mut to: String = ui.data(|d| d.get_temp(to_id)).unwrap_or_default();
    let mut picked: Option<usize> = ui.data(|d| d.get_temp(row_id)).flatten();
    if picked.is_some_and(|i| i >= p.replacements.len()) {
        picked = None;
    }
    let key = from.trim().to_string();
    let existing = p.replacements.iter().position(|(f, _)| *f == key);
    let usable = !key.is_empty() && !key.contains(char::is_whitespace);
    ui.horizontal(|ui| {
        ui.label(tr("autocorrect.replace_col"));
        ui.add(egui::TextEdit::singleline(&mut from).desired_width(130.0))
            .rich_tip(crate::ui::tips::pair(
                "autocorrect.replace_col",
                "autocorrect.replace_col_tip",
            ));
        ui.add_space(10.0);
        ui.label(tr("autocorrect.with_col"));
        ui.add(egui::TextEdit::singleline(&mut to).desired_width(130.0))
            .rich_tip(crate::ui::tips::pair(
                "autocorrect.with_col",
                "autocorrect.with_col_tip",
            ));
    });
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                usable && existing.is_none(),
                egui::Button::new(tr("autocorrect.add")),
            )
            .rich_tip_always(crate::ui::tips::pair(
                "autocorrect.add",
                "autocorrect.add_tip",
            ))
            .clicked()
        {
            p.replacements.push((key.clone(), to.clone()));
            from.clear();
            to.clear();
        }
        if ui
            .add_enabled(
                picked.is_some(),
                egui::Button::new(tr("autocorrect.delete")),
            )
            .rich_tip_always(crate::ui::tips::pair(
                "autocorrect.delete",
                "autocorrect.delete_tip",
            ))
            .clicked()
        {
            if let Some(i) = picked.take() {
                p.replacements.remove(i);
            }
            from.clear();
            to.clear();
        }
        if ui
            .add_enabled(
                usable && existing.is_some(),
                egui::Button::new(tr("autocorrect.replace")),
            )
            .rich_tip_always(crate::ui::tips::pair(
                "autocorrect.replace",
                "autocorrect.replace_tip",
            ))
            .clicked()
        {
            if let Some(i) = existing {
                p.replacements[i].1 = to.clone();
            }
        }
    });
    egui::Frame::new()
        .fill(Tokens::field())
        .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("autocorrect_table")
                .max_height(150.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("autocorrect_rows")
                        .num_columns(2)
                        .min_col_width(160.0)
                        .show(ui, |ui| {
                            for (i, (f, t)) in p.replacements.iter().enumerate() {
                                let on = picked == Some(i);
                                let tip = || {
                                    crate::ui::tips::pair(
                                        "autocorrect.replacement",
                                        "autocorrect.replacement_tip",
                                    )
                                };
                                let a = ui.selectable_label(on, f.as_str()).rich_tip(tip());
                                let b = ui.selectable_label(on, t.as_str()).rich_tip(tip());
                                if a.clicked() || b.clicked() {
                                    picked = Some(i);
                                    from = f.clone();
                                    to = t.clone();
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    ui.data_mut(|d| {
        d.insert_temp(from_id, from);
        d.insert_temp(to_id, to);
        d.insert_temp(row_id, picked);
    });
    ui.add_space(4.0);
    if ui
        .button(tr("autocorrect.apply_selection"))
        .rich_tip(crate::ui::tips::pair(
            "autocorrect.apply_selection",
            "autocorrect.apply_selection_tip",
        ))
        .clicked()
    {
        if app.text_shapes().is_empty() {
            app.status = tr("autocorrect.no_selection");
        } else {
            let n = crate::autocorrect::apply_to_selection(app);
            app.status = trf("autocorrect.applied", &[("n", &n.to_string())]);
        }
    }
}

// ----- Customization -----------------------------------------------------

/// The interface sizes the Appearance slider offers, in percent.
const UI_SIZE_RANGE: std::ops::RangeInclusive<f32> = 75.0..=250.0;

/// Customization > Appearance: where dialogs open, the interface size
/// and the desktop colour.
fn appearance_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(
        &mut app.settings.center_dialogs,
        tr("options.center_dialogs"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.center_dialogs",
        "options.center_dialogs_tip",
    ));
    ui.checkbox(&mut app.settings.ui_shadows, tr("options.ui_shadows"))
        .rich_tip(tr("options.ui_shadows_tip"));
    ui.checkbox(&mut app.settings.colored_icons, tr("options.colored_icons"))
        .rich_tip(tr("options.colored_icons_tip"));
    crate::ui::chrome::section(ui, &tr("options.ui_size"));
    // The size takes effect when the slider is let go or the field is
    // left, so the page does not grow under the pointer.
    let pending = ui.id().with("ui_size_pending");
    let current = crate::ui::interface_size(app.settings.ui_size) * 100.0;
    let mut pct: f32 = ui.data(|d| d.get_temp(pending)).unwrap_or(current);
    let mut reset = false;
    let busy = ui
        .horizontal(|ui| {
            let slider = ui
                .add(
                    crate::ui::Slider::new(&mut pct, UI_SIZE_RANGE)
                        .step_by(5.0)
                        .show_value(false)
                        .trailing_fill(true)
                        .width(220.0),
                )
                .rich_tip(crate::ui::tips::pair(
                    "options.interface_size",
                    "options.ui_size_tip",
                ));
            let field = ui
                .add(
                    crate::ui::field::NumField::new(&mut pct)
                        .range(UI_SIZE_RANGE)
                        .speed(1.0)
                        .max_decimals(0)
                        .suffix(" %"),
                )
                .rich_tip(crate::ui::tips::pair(
                    "options.interface_size",
                    "options.ui_size_tip",
                ));
            reset = ui
                .button(tr("options.reset_to_default"))
                .rich_tip(crate::ui::tips::pair(
                    "options.reset_to_default",
                    "options.reset_to_default_tip",
                ))
                .clicked();
            slider.dragged() || field.dragged() || field.has_focus()
        })
        .inner;
    if reset {
        app.settings.ui_size = 0.0;
        ui.data_mut(|d| d.remove::<f32>(pending));
    } else if busy {
        ui.data_mut(|d| d.insert_temp(pending, pct));
    } else {
        ui.data_mut(|d| d.remove::<f32>(pending));
        if (pct - current).abs() > 0.5 {
            app.settings.ui_size = pct.round() / 100.0;
        }
    }
    ui.label(egui::RichText::new(tr("options.ui_scale_hint")).color(Tokens::text_dim()));
    let sys = crate::system_prefs::get();
    ui.checkbox(
        &mut app.settings.follow_text_size,
        crate::i18n::trf(
            "options.follow_text_size",
            &[("n", &format!("{:.0}", sys.text_scale * 100.0))],
        ),
    )
    .rich_tip_with(|| {
        format!(
            "{}\n{}",
            tr("options.follow_text_size_title"),
            tr("options.follow_text_size_tip")
        )
    });
    crate::ui::chrome::section(ui, &tr("options.color_section"));
    grid("appearance_color").show(ui, |ui| {
        row_label(ui, &tr("options.theme"));
        let current = crate::theme::Theme::from_id(&app.settings.theme);
        let tip = crate::ui::tips::pair("options.theme", "options.theme_tip");
        crate::ui::tips::tip_over(ui, tip, |ui| {
            egui::ComboBox::from_id_salt("ui_theme")
                .selected_text(tr(current.key()))
                .width(170.0)
                .show_ui(ui, |ui| {
                    for t in crate::theme::Theme::ALL {
                        if ui.selectable_label(t == current, tr(t.key())).clicked() {
                            app.settings.theme = t.id().to_string();
                        }
                    }
                });
        });
        ui.end_row();
        row_label(ui, "");
        let on = sys.high_contrast.is_some();
        ui.checkbox(
            &mut app.settings.follow_high_contrast,
            tr(if on {
                "options.follow_high_contrast_on"
            } else {
                "options.follow_high_contrast"
            }),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.follow_high_contrast_title",
            "options.follow_high_contrast_tip",
        ));
        ui.end_row();
        row_label(ui, &tr("options.desktop_color"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.desktop_color", "options.desktop_color_tip"),
            |ui| {
                ui.horizontal(|ui| {
                    // The colour shown is the one around the page, the theme's
                    // until one is picked; any pick counts, white too (#530).
                    let [r, g, b, _] = app.desktop_color().to_array();
                    let mut rgb = [r, g, b];
                    if rgb_button(ui, &mut rgb) {
                        app.settings.desktop_rgb = rgb;
                        app.settings.desktop_custom = true;
                    }
                    if ui
                        .button(tr("options.reset"))
                        .rich_tip(crate::ui::tips::pair("options.reset", "dialog.reset_desc"))
                        .clicked()
                    {
                        app.settings.desktop_rgb = [255, 255, 255];
                        app.settings.desktop_custom = false;
                    }
                });
            },
        );

        ui.end_row();
    });
}

/// Customization > Color Palette: the document palette, the swatches and
/// what the right mouse button does on them.
fn color_palette_page(app: &mut App, ui: &mut Ui) {
    let p = &mut app.settings.palette;
    ui.checkbox(
        &mut p.auto_update_document,
        tr("options.palette_auto_update"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.palette_auto_update",
        "options.palette_auto_update_tip",
    ));
    ui.checkbox(&mut p.show_document, tr("options.palette_show_document"))
        .rich_tip(crate::ui::tips::pair(
            "options.palette_show_document",
            "options.palette_show_document_tip",
        ));
    crate::ui::chrome::section(ui, &tr("options.palette_swatches"));
    ui.checkbox(&mut p.show_no_color, tr("options.palette_no_color"))
        .rich_tip(crate::ui::tips::pair(
            "options.palette_no_color",
            "options.palette_no_color_tip",
        ));
    // Rows of the default palette (columns docked on a side, #610).
    p.rows = p.rows.clamp(1, 3);
    grid("palette_rows").show(ui, |ui| {
        row_label(ui, &tr("options.palette_rows"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.palette_rows", "options.palette_rows_tip"),
            |ui| {
                combo(
                    ui,
                    "opt_palette_rows",
                    &mut p.rows,
                    &[1, 2, 3],
                    |n| n.to_string(),
                    60.0,
                );
            },
        );

        ui.end_row();
    });
    crate::ui::chrome::section(ui, &tr("options.palette_right_button"));
    ui.radio_value(
        &mut p.right_click_outline,
        false,
        tr("options.palette_context_menu"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.palette_context_menu",
        "options.palette_context_menu_tip",
    ));
    ui.radio_value(
        &mut p.right_click_outline,
        true,
        tr("options.palette_set_outline"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.palette_set_outline",
        "options.palette_set_outline_tip",
    ));
    if p.right_click_outline {
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.label(
                egui::RichText::new(tr("options.palette_hold_hint")).color(Tokens::text_dim()),
            );
        });
    }
}

/// Options > Customization > Commands: the menu commands found, the
/// search text, the command waiting for its keys and what the last
/// shortcut set took over.
#[derive(Debug, Clone, Default)]
pub struct ShortcutEditor {
    pub entries: Vec<crate::ui::menus::invoke::Entry>,
    pub filter: String,
    pub capture: Option<String>,
    pub note: Option<String>,
    /// The list shows one menu (its title key), the toolbox
    /// ([`TOOLBOX`]) or everything (`None`).
    pub category: Option<String>,
    /// The command picked in the list (a menu key or a tool id).
    pub selected: Option<String>,
    /// The tab beside the list: General (false) or Shortcut Keys (true).
    pub keys_tab: bool,
    /// Reset all shortcuts was pressed: the question is shown.
    pub confirm_reset_all: bool,
}

/// The Commands page's category for the toolbox's tools.
pub const TOOLBOX: &str = "toolbox";
/// The commands category of the loaded VBA macros.
pub const MACROS: &str = "macros";
/// The commands category of the Standard and Zoom toolbars' own buttons
/// (#836).
pub const BUTTONS: &str = "toolbar_buttons";

/// The user's shortcut for `id` (a menu key or a tool id).
fn user_shortcut(app: &App, id: &str) -> Option<String> {
    app.settings
        .shortcuts
        .iter()
        .find(|(k, _)| k == id)
        .map(|(_, v)| v.clone())
}

/// Give the menu command `id` the shortcut `text`. Another command of the
/// user's with it loses it; commands and tools that have it built in are
/// named in the note, since the shortcut now runs this command.
fn set_menu_shortcut(app: &mut App, id: &str, text: &str) {
    use crate::shortcuts::same;
    let mut others: Vec<String> = Vec::new();
    for (k, v) in &app.settings.shortcuts {
        if k != id && same(v, text) {
            others.push(menu_or_tool_name(app, k));
        }
    }
    app.settings
        .shortcuts
        .retain(|(k, v)| k == id || !same(v, text));
    for e in &app.shortcut_editor.entries {
        if e.key != id && same(&e.shortcut, text) && user_shortcut(app, &e.key).is_none() {
            others.push(command_label(e));
        }
    }
    for t in crate::tools::Tool::ALL {
        if user_shortcut(app, t.id()).is_none() && t.shortcut().is_some_and(|s| same(s, text)) {
            others.push(t.name());
        }
    }
    // The keys it had stay (a command may have several, #822): a built-in
    // key joins the user's list first so that it keeps working.
    if user_shortcut(app, id).is_none() {
        if let Some(b) = built_in_keys(app, id) {
            app.settings.shortcuts.push((id.to_string(), b));
        }
    }
    let has = app
        .settings
        .shortcuts
        .iter()
        .any(|(k, v)| k == id && same(v, text));
    if !has {
        app.settings
            .shortcuts
            .push((id.to_string(), text.to_string()));
    }
    others.dedup();
    app.shortcut_editor.note = (!others.is_empty()).then(|| {
        trf(
            "options.shortcut_taken",
            &[("keys", text), ("others", &others.join(", "))],
        )
    });
}

/// The keys `id` has built in (a menu command's or a tool's), if any.
fn built_in_keys(app: &App, id: &str) -> Option<String> {
    let menu = app
        .shortcut_editor
        .entries
        .iter()
        .find(|e| e.key == id)
        .map(|e| e.shortcut.clone());
    let tool = crate::tools::Tool::ALL
        .iter()
        .find(|t| t.id() == id)
        .and_then(|t| t.shortcut())
        .map(str::to_string);
    menu.or(tool).filter(|k| !k.is_empty())
}

/// "File > Export...": the menus a command sits in and its name.
fn command_label(e: &crate::ui::menus::invoke::Entry) -> String {
    let mut parts: Vec<String> = e.path.iter().map(|k| tr(k)).collect();
    parts.push(match crate::macros::command_name(&e.key) {
        Some((name, _)) => name,
        None => tr(&e.key),
    });
    parts
        .iter()
        .map(|p| p.trim_end_matches("...").to_string())
        .collect::<Vec<_>>()
        .join(" > ")
}

fn menu_or_tool_name(app: &App, id: &str) -> String {
    if let Some(e) = app.shortcut_editor.entries.iter().find(|e| e.key == id) {
        return command_label(e);
    }
    crate::tools::Tool::ALL
        .iter()
        .find(|t| t.id() == id)
        .map(|t| t.name())
        .unwrap_or_else(|| id.to_string())
}

/// One row of the command list: a menu command or a toolbox tool.
#[derive(Debug, Clone)]
struct CommandItem {
    id: String,
    name: String,
    /// "File > Export" for a menu command, the toolbox for a tool.
    place: String,
    built_in: String,
    tool: Option<crate::tools::Tool>,
}

/// The rows of `category`, filtered by `filter` (lower case).
fn command_items(app: &App, category: Option<&str>, filter: &str) -> Vec<CommandItem> {
    let mut out = Vec::new();
    if category.is_none_or(|c| c != TOOLBOX && c != BUTTONS) {
        for e in &app.shortcut_editor.entries {
            let top = e.path.first().map(String::as_str);
            let is_macro = e.key.starts_with("menu.macro.");
            match category {
                Some(MACROS) if !is_macro => continue,
                Some(c) if c != MACROS && top != Some(c) => continue,
                _ => {}
            }
            let label = command_label(e);
            let name = match crate::macros::command_name(&e.key) {
                Some((name, _)) => name,
                None => tr(&e.key).trim_end_matches("...").to_string(),
            };
            out.push(CommandItem {
                id: e.key.clone(),
                name,
                place: label,
                built_in: e.shortcut.clone(),
                tool: None,
            });
        }
    }
    if category.is_none_or(|c| c == TOOLBOX) {
        for t in crate::tools::Tool::ALL {
            out.push(CommandItem {
                id: t.id().to_string(),
                name: t.name(),
                place: tr("menu.window.toolbar_toolbox"),
                built_in: t.shortcut().unwrap_or("").to_string(),
                tool: Some(t),
            });
        }
    }
    // The bars' own buttons, so one taken off a bar can be put back
    // (#836).
    if category.is_none_or(|c| c == BUTTONS) {
        for id in crate::ui::bar_items::all_buttons() {
            out.push(CommandItem {
                id: id.to_string(),
                name: crate::ui::toolbar::item_name(id).unwrap_or_else(|| id.to_string()),
                place: tr("options.toolbar_buttons"),
                built_in: String::new(),
                tool: None,
            });
        }
    }
    if !filter.is_empty() {
        out.retain(|c| {
            c.name.to_lowercase().contains(filter) || c.place.to_lowercase().contains(filter)
        });
    }
    out
}

const LIST_W: f32 = 240.0;
const LIST_H: f32 = 330.0;
const ROW_H: f32 = 24.0;

/// Options > Customization > Commands: a category and a search above the
/// list of commands (with their pictures); beside it the picked
/// command's General and Shortcut Keys tabs.
fn commands_page(app: &mut App, ui: &mut Ui) {
    if app.shortcut_editor.entries.is_empty() {
        let ctx = ui.ctx().clone();
        app.shortcut_editor.entries = crate::ui::menus::commands(app, &ctx);
    }
    capture_keys(app, ui);
    ui.label(
        egui::RichText::new(tr("options.toolbar_drag_hint"))
            .color(Tokens::text_dim())
            .size(11.0),
    );
    let category_name = |c: Option<&str>| match c {
        None => tr("options.all_commands"),
        Some(TOOLBOX) => tr("menu.window.toolbar_toolbox"),
        Some(MACROS) => tr("options.macros_category"),
        Some(BUTTONS) => tr("options.toolbar_buttons"),
        Some(k) => tr(k),
    };
    ui.horizontal(|ui| {
        let current = app.shortcut_editor.category.clone();
        egui::ComboBox::from_id_salt("command_category")
            .selected_text(category_name(current.as_deref()))
            .width(LIST_W - 8.0)
            .show_ui(ui, |ui| {
                let mut pick = current.clone();
                ui.selectable_value(&mut pick, None, category_name(None));
                for k in crate::ui::menus::menu_titles() {
                    ui.selectable_value(&mut pick, Some(k.to_string()), category_name(Some(k)));
                }
                ui.selectable_value(
                    &mut pick,
                    Some(TOOLBOX.to_string()),
                    category_name(Some(TOOLBOX)),
                );
                ui.selectable_value(
                    &mut pick,
                    Some(MACROS.to_string()),
                    category_name(Some(MACROS)),
                );
                ui.selectable_value(
                    &mut pick,
                    Some(BUTTONS.to_string()),
                    category_name(Some(BUTTONS)),
                );
                app.shortcut_editor.category = pick;
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "options.command_category",
                "options.command_category_tip",
            ));
        ui.add_space(12.0);
        ui.label(tr("options.find_command"));
        ui.add(egui::TextEdit::singleline(&mut app.shortcut_editor.filter).desired_width(180.0))
            .rich_tip(crate::ui::tips::pair(
                "options.find_command",
                "options.find_command_tip",
            ));
    });
    ui.add_space(4.0);
    let filter = app.shortcut_editor.filter.trim().to_lowercase();
    let items = command_items(app, app.shortcut_editor.category.as_deref(), &filter);
    if app
        .shortcut_editor
        .selected
        .as_ref()
        .is_none_or(|s| !items.iter().any(|c| &c.id == s))
    {
        app.shortcut_editor.selected = items.first().map(|c| c.id.clone());
    }
    ui.horizontal_top(|ui| {
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
            .fill(crate::theme::Tokens::field())
            .show(ui, |ui| {
                ui.set_width(LIST_W);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    egui::ScrollArea::vertical()
                        .id_salt("command_list")
                        .auto_shrink([false, false])
                        .max_height(LIST_H)
                        .min_scrolled_height(LIST_H)
                        .show_rows(ui, ROW_H, items.len(), |ui, rows| {
                            for i in rows {
                                if let Some(c) = items.get(i) {
                                    command_list_row(app, ui, c);
                                }
                            }
                        });
                });
            });
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.set_max_width(ui.available_width());
            let selected = app
                .shortcut_editor
                .selected
                .as_ref()
                .and_then(|s| items.iter().find(|c| &c.id == s))
                .cloned();
            // A bar's button takes the keys of the menu command it runs
            // (#1148); one the menus lack runs no command a key could take.
            let selected = selected.map(|c| keys_target(app, c));
            let button = selected
                .as_ref()
                .is_some_and(|c| c.tool.is_none() && crate::ui::bar_items::is_button(&c.id));
            if button {
                app.shortcut_editor.keys_tab = false;
            }
            ui.horizontal(|ui| {
                let keys = &mut app.shortcut_editor.keys_tab;
                ui.selectable_value(keys, false, tr("options.general"))
                    .rich_tip(crate::ui::tips::pair(
                        "options.general",
                        "options.general_tip",
                    ));
                if !button {
                    ui.selectable_value(keys, true, tr("options.shortcut_keys"))
                        .rich_tip(crate::ui::tips::pair(
                            "options.shortcut_keys",
                            "options.shortcut_keys_tip",
                        ));
                }
            });
            ui.separator();
            match selected {
                None => {
                    ui.label(
                        egui::RichText::new(tr("options.no_command")).color(Tokens::text_dim()),
                    );
                }
                Some(c) if !app.shortcut_editor.keys_tab => command_general(app, ui, &c),
                Some(c) => command_keys(app, ui, &c),
            }
        });
    });
}

/// The command whose keys `c` shows and sets: a bar button that runs a
/// menu command stands for that command (its keys are the button's),
/// keeping its own name; anything else is itself.
fn keys_target(app: &App, c: CommandItem) -> CommandItem {
    if c.tool.is_some() {
        return c;
    }
    let entry = crate::ui::bar_items::menu_command(&c.id)
        .and_then(|key| app.shortcut_editor.entries.iter().find(|e| e.key == key));
    match entry {
        Some(e) => CommandItem {
            id: e.key.clone(),
            name: c.name,
            place: command_label(e),
            built_in: e.shortcut.clone(),
            tool: None,
        },
        None => c,
    }
}

/// One row of the list: the command's picture and name.
fn command_list_row(app: &mut App, ui: &mut Ui, c: &CommandItem) {
    // A command or a tool dragged onto the Standard or Zoom toolbar
    // joins it (#563).
    let button = c.tool.is_none() && crate::ui::bar_items::is_button(&c.id);
    let item = match c.tool {
        Some(t) => Some(format!("{}{}", crate::ui::bar_items::TOOL, t.id())),
        None => (c.id.starts_with("menu.") || button).then(|| c.id.clone()),
    };
    let (row, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_H),
        if item.is_some() {
            egui::Sense::click_and_drag()
        } else {
            egui::Sense::click()
        },
    );
    if let (true, Some(item)) = (resp.drag_started(), item) {
        app.bar_item_drag = Some(crate::ui::bar_items::ItemDrag { item, from: None });
    }
    let on = app.shortcut_editor.selected.as_deref() == Some(c.id.as_str());
    if on {
        ui.painter()
            .rect_filled(row, 0.0, crate::theme::Tokens::selected_row());
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(row, 0.0, crate::theme::Tokens::hovered_row());
    }
    let icon = egui::Rect::from_center_size(
        egui::pos2(row.left() + 14.0, row.center().y),
        egui::vec2(16.0, 16.0),
    );
    match c.tool {
        Some(t) => crate::ui::icons::draw(ui.painter(), icon, t, Tokens::icon()),
        None if button => crate::ui::bar_items::picture(ui.painter(), icon, &c.id, Tokens::icon()),
        None => {
            crate::ui::menus::draw_menu_picture(ui.painter(), icon, &c.id, Tokens::icon());
        }
    }
    let galley = ui.painter().layout_no_wrap(
        c.name.clone(),
        egui::FontId::proportional(13.0),
        Tokens::text(),
    );
    ui.painter().with_clip_rect(row).galley(
        egui::pos2(row.left() + 30.0, row.center().y - galley.size().y / 2.0),
        galley,
        Tokens::text(),
    );
    let resp = resp.rich_tip(&c.place);
    if resp.clicked() {
        app.shortcut_editor.selected = Some(c.id.clone());
        app.shortcut_editor.capture = None;
        app.shortcut_editor.note = None;
    }
}

/// General: the command's name, where it is and its keys.
fn command_general(app: &App, ui: &mut Ui, c: &CommandItem) {
    grid("command_general").show(ui, |ui| {
        row_label(ui, &tr("options.command_name"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.command_name", "options.command_name_tip"),
            |ui| {
                ui.add(egui::Label::new(egui::RichText::new(&c.name).strong()).wrap());
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.available_on"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.available_on", "options.available_on_tip"),
            |ui| {
                ui.add(egui::Label::new(&c.place).wrap());
            },
        );

        ui.end_row();
        row_label(ui, &format!("{}:", tr("options.shortcut_keys")));
        let keys = current_keys(app, c);
        ui.label(if keys.is_empty() {
            "-".to_string()
        } else {
            keys.join(", ")
        });
        ui.end_row();
    });
}

/// The keys that run `c` now: the user's, else the built-in ones. The
/// user's may be empty: the command's shortcut was removed.
fn current_keys(app: &App, c: &CommandItem) -> Vec<String> {
    crate::shortcuts::keys_for(&app.settings.shortcuts, &c.id, Some(&c.built_in))
}

/// Shortcut Keys: the key waiting for a press, the current keys with
/// Delete, and Reset All.
fn command_keys(app: &mut App, ui: &mut Ui, c: &CommandItem) {
    ui.label(tr("options.new_shortcut"));
    let capturing = app.shortcut_editor.capture.as_deref() == Some(c.id.as_str());
    let text = if capturing {
        tr("options.press_keys")
    } else {
        tr("options.set_shortcut")
    };
    let button = ui
        .add_sized([200.0, 24.0], egui::Button::new(text).selected(capturing))
        .rich_tip(tr("options.set_shortcut_hint"));
    if button.clicked() {
        app.shortcut_editor.capture = (!capturing).then(|| c.id.clone());
        app.shortcut_editor.note = None;
        // The keys pressed next are the shortcut, not typing or a press
        // of this button.
        ui.ctx().memory_mut(|m| m.stop_text_input());
        button.surrender_focus();
    }
    ui.add_space(6.0);
    ui.label(tr("options.current_shortcuts"));
    let custom = user_shortcut(app, &c.id).is_some();
    let keys = current_keys(app, c);
    let mut delete_key = None;
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, Tokens::control_border()))
        .fill(crate::theme::Tokens::field())
        .inner_margin(4)
        .show(ui, |ui| {
            ui.set_width(260.0);
            ui.set_min_height(70.0);
            // Each key on its row, deleted on its own (#822).
            for k in &keys {
                ui.horizontal(|ui| {
                    if custom {
                        ui.label(k);
                    } else {
                        ui.label(format!("{k}  ({})", tr("options.built_in")));
                    }
                    if ui
                        .small_button(tr("options.delete"))
                        .rich_tip(tr("options.clear_shortcut"))
                        .clicked()
                    {
                        delete_key = Some(k.clone());
                    }
                });
            }
        });
    if let Some(k) = delete_key {
        remove_key(app, c, &k);
    }
    let has_keys = !current_keys(app, c).is_empty();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(has_keys, egui::Button::new(tr("options.delete")))
            .rich_tip(tr("options.clear_shortcut"))
            .clicked()
        {
            remove_shortcut(app, c);
        }
        // The commands of the category shown; with every command shown,
        // the same as resetting all, so it asks first.
        if ui
            .button(tr("options.reset_shortcuts"))
            .rich_tip(crate::ui::tips::pair(
                "options.reset_shortcuts",
                "options.reset_shortcuts_tip",
            ))
            .clicked()
        {
            match app.shortcut_editor.category.clone() {
                Some(category) => reset_category_shortcuts(app, &category),
                None => app.shortcut_editor.confirm_reset_all = true,
            }
        }
    });
    if ui
        .button(tr("options.reset_all_shortcuts"))
        .rich_tip(crate::ui::tips::pair(
            "options.reset_all_shortcuts",
            "options.reset_all_shortcuts_tip",
        ))
        .clicked()
    {
        app.shortcut_editor.confirm_reset_all = true;
    }
    if app.shortcut_editor.confirm_reset_all {
        ui.label(tr("options.reset_all_shortcuts_question"));
        ui.horizontal(|ui| {
            if ui
                .button(tr("dialog.reset"))
                .rich_tip(crate::ui::tips::keyed("dialog.reset"))
                .clicked()
            {
                app.settings.shortcuts.clear();
                app.shortcut_editor.note = None;
                app.shortcut_editor.capture = None;
                app.shortcut_editor.confirm_reset_all = false;
            }
            if ui
                .button(tr("dialog.cancel"))
                .rich_tip(crate::ui::tips::keyed("dialog.cancel"))
                .clicked()
            {
                app.shortcut_editor.confirm_reset_all = false;
            }
        });
    }
    // What the last shortcut set took over.
    if let Some(note) = &app.shortcut_editor.note {
        ui.label(egui::RichText::new(note).color(Tokens::ACCENT));
    }
}

/// Take `key` from the keys of `c`, keeping its others; with none left,
/// `c` is left without a shortcut ([`remove_shortcut`]).
fn remove_key(app: &mut App, c: &CommandItem, key: &str) {
    use crate::shortcuts::same;
    let keys: Vec<String> = current_keys(app, c)
        .into_iter()
        .filter(|k| !same(k, key))
        .collect();
    if keys.is_empty() {
        remove_shortcut(app, c);
        return;
    }
    app.settings.shortcuts.retain(|(k, _)| k != &c.id);
    for k in keys {
        app.settings.shortcuts.push((c.id.clone(), k));
    }
    app.shortcut_editor.note = None;
}

/// Leave `c` without a shortcut: the user's goes, and a built-in one is
/// overridden by an empty shortcut, so its keys no longer run it.
fn remove_shortcut(app: &mut App, c: &CommandItem) {
    app.settings.shortcuts.retain(|(k, _)| k != &c.id);
    if !c.built_in.is_empty() {
        app.settings.shortcuts.push((c.id.clone(), String::new()));
    }
    app.shortcut_editor.note = None;
}

/// Reset to defaults on the Commands page: the shortcuts of the commands
/// in `category` (a menu's title key, the toolbox or the macros) only.
fn reset_category_shortcuts(app: &mut App, category: &str) {
    let ids: Vec<String> = command_items(app, Some(category), "")
        .into_iter()
        .map(|c| c.id)
        .collect();
    app.settings.shortcuts.retain(|(k, _)| !ids.contains(k));
    app.shortcut_editor.note = None;
    app.shortcut_editor.capture = None;
}

/// The command waiting for its keys takes the next key pressed (Shift,
/// Ctrl and Alt alone keep it waiting); tools take the same keys as
/// menu commands.
fn capture_keys(app: &mut App, ui: &mut Ui) {
    let Some(target) = app.shortcut_editor.capture.clone() else {
        return;
    };
    let pressed = ui.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if !crate::shortcuts::is_modifier(*key) => Some((*key, *modifiers)),
            _ => None,
        })
    });
    let Some((key, mods)) = pressed else {
        return;
    };
    if key == egui::Key::Escape && !(mods.ctrl || mods.alt || mods.command) {
        app.shortcut_editor.capture = None;
        return;
    }
    if crate::shortcuts::allowed(mods, key) {
        let text = crate::shortcuts::format(mods, key);
        set_menu_shortcut(app, &target, &text);
        app.shortcut_editor.capture = None;
    }
}

/// Customization > Command Bars: the bars shown, in a list, with Reset;
/// beside it the options that hold for every bar.
fn command_bars_page(app: &mut App, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(230.0);
            ui.label(tr("options.command_bars_hint"));
            ui.label(
                egui::RichText::new(tr("options.toolbar_drag_hint"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0, Tokens::border()))
                .fill(crate::theme::Tokens::field())
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.set_width(216.0);
                    ui.set_min_height(230.0);
                    // The menu bar is always there.
                    ui.add_enabled(
                        false,
                        egui::Checkbox::new(&mut true, tr("menu.window.toolbar_menu_bar")),
                    )
                    .rich_disabled_tip(crate::ui::tips::pair(
                        "menu.window.toolbar_menu_bar",
                        "options.menu_bar_always_tip",
                    ));
                    let palette = &mut app.settings.palette.show_palette;
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
                        (palette, "bars.palette"),
                        (&mut app.show_text_toolbar, "menu.window.toolbar_text"),
                        (&mut app.show_zoom_toolbar, "menu.window.toolbar_zoom"),
                        (
                            &mut app.show_transform_toolbar,
                            "menu.window.toolbar_transform",
                        ),
                    ] {
                        ui.checkbox(flag, tr(key)).rich_tip_with(|| {
                            format!("{}\n{}", tr(key), tr("options.show_bar_tip"))
                        });
                    }
                });
            if ui
                .button(tr("options.reset"))
                .rich_tip(crate::ui::tips::pair("options.reset", "dialog.reset_desc"))
                .clicked()
            {
                app.reset_command_bars();
            }
        });
        ui.add_space(16.0);
        ui.vertical(|ui| {
            crate::ui::chrome::section(ui, &tr("options.other"));
            ui.checkbox(&mut app.settings.bars.locked, tr("bars.lock"))
                .rich_tip(crate::ui::tips::pair("bars.lock", "bars.lock_tip"));
            ui.checkbox(
                &mut app.settings.show_outline_flyout,
                tr("options.show_outline_flyout"),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.show_outline_flyout",
                "options.show_outline_flyout_tip",
            ));
        });
    });
}

// ----- Tools -------------------------------------------------------------

fn pick_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(
        &mut app.settings.crosshair_cursor,
        tr("options.crosshair_cursor"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.crosshair_cursor",
        "options.crosshair_cursor_tip",
    ));
    ui.checkbox(
        &mut app.settings.treat_all_filled,
        tr("options.treat_all_filled"),
    )
    .rich_tip(crate::ui::tips::keyed("options.treat_all_filled"));
    ui.checkbox(
        &mut app.settings.marquee_touching,
        tr("options.marquee_touching"),
    )
    .rich_tip(tr("options.marquee_touching_tip"));
}

fn zoom_pan_page(app: &mut App, ui: &mut Ui) {
    ui.label(tr("options.zoom_right_button"));
    ui.indent("zr", |ui| {
        ui.radio_value(
            &mut app.settings.zoom_right_click_out,
            true,
            tr("options.zoom_out"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.zoom_out",
            "options.zoom_out_tip",
        ));
        ui.radio_value(
            &mut app.settings.zoom_right_click_out,
            false,
            tr("options.context_menu"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.context_menu",
            "options.context_menu_tip",
        ));
    });
    ui.separator();
    ui.label(tr("options.wheel_action"));
    ui.indent("wa", |ui| {
        ui.radio_value(
            &mut app.settings.wheel_zooms,
            true,
            tr("options.wheel_zoom"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.wheel_zoom",
            "options.wheel_zoom_tip",
        ));
        ui.radio_value(
            &mut app.settings.wheel_zooms,
            false,
            tr("options.wheel_scroll"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.wheel_scroll",
            "options.wheel_scroll_tip",
        ));
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
                ui.radio_value(&mut c.kind, kind, tr(key))
                    .rich_tip(crate::ui::tips::keyed(key));
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
                        let mut v = u.world_from_mm(c.radii[i]);
                        if ui
                            .add_sized(
                                [80.0, 22.0],
                                crate::ui::field::NumField::new(&mut v)
                                    .range(0.0..=10_000.0)
                                    .speed(0.1)
                                    .suffix(format!(" {}", u.short())),
                            )
                            .rich_tip(crate::ui::tips::keyed(key))
                            .changed()
                        {
                            let v = u.world_to_mm(v).max(0.0);
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
        ui.checkbox(&mut app.corners_together, tr("toolbar.corners_together"))
            .rich_tip(crate::ui::tips::keyed("toolbar.corners_together"));
    });
    chrome::section(ui, &tr("options.scale_corners"));
    ui.indent("scale_corners", |ui| {
        let mut relative = !c.fixed;
        if ui
            .checkbox(&mut relative, tr("toolbar.relative_corners"))
            .rich_tip(crate::ui::tips::keyed("toolbar.relative_corners"))
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
    ui.radio_value(&mut kind, 0, tr("options.ellipse"))
        .rich_tip(crate::ui::tips::pair(
            "options.ellipse",
            "options.ellipse_tip",
        ));
    ui.radio_value(&mut kind, 1, tr("options.pie"))
        .rich_tip(crate::ui::tips::pair("options.pie", "options.pie_tip"));
    ui.radio_value(&mut kind, 2, tr("options.arc"))
        .rich_tip(crate::ui::tips::pair("options.arc", "options.arc_tip"));
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
            crate::ui::tips::tip_over(
                ui,
                crate::ui::tips::pair("options.starting_angle", "options.starting_angle_tip"),
                |ui| {
                    ui.add(
                        crate::ui::field::NumField::new(&mut a.start_deg)
                            .range(-360.0..=360.0)
                            .suffix("°"),
                    );
                },
            );

            ui.end_row();
            row_label(ui, &tr("options.ending_angle"));
            crate::ui::tips::tip_over(
                ui,
                crate::ui::tips::pair("options.ending_angle", "options.ending_angle_tip"),
                |ui| {
                    ui.add(
                        crate::ui::field::NumField::new(&mut a.end_deg)
                            .range(-360.0..=360.0)
                            .suffix("°"),
                    );
                },
            );

            ui.end_row();
        });
    }
}

fn polygon_page(app: &mut App, ui: &mut Ui) {
    grid("polygon").show(ui, |ui| {
        row_label(ui, &tr("options.number_of_points"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.number_of_points", "options.number_of_points_tip"),
            |ui| {
                ui.add(crate::ui::field::NumField::new(&mut app.polygon_points).range(3..=500));
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.sharpness"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.sharpness", "options.sharpness_tip"),
            |ui| {
                // Stored as a fraction, shown in percent like the property bar.
                let mut pct = (app.star_sharpness * 100.0).round();
                if ui
                    .add(
                        crate::ui::field::NumField::new(&mut pct)
                            .range(1.0..=99.0)
                            .suffix("%"),
                    )
                    .changed()
                {
                    app.star_sharpness = (pct / 100.0).clamp(0.01, 0.99);
                };
            },
        );

        ui.end_row();
    });
}

fn spiral_page(app: &mut App, ui: &mut Ui) {
    grid("spiral").show(ui, |ui| {
        row_label(ui, &tr("options.revolutions"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.revolutions", "options.revolutions_tip"),
            |ui| {
                ui.add(crate::ui::field::NumField::new(&mut app.spiral_revolutions).range(1..=100));
            },
        );

        ui.end_row();
    });
    ui.radio_value(
        &mut app.spiral_logarithmic,
        false,
        tr("options.symmetrical_spiral"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.symmetrical_spiral",
        "options.symmetrical_spiral_tip",
    ));
    ui.radio_value(
        &mut app.spiral_logarithmic,
        true,
        tr("options.logarithmic_spiral"),
    )
    .rich_tip(crate::ui::tips::pair(
        "options.logarithmic_spiral",
        "options.logarithmic_spiral_tip",
    ));
}

fn graph_paper_page(app: &mut App, ui: &mut Ui) {
    grid("graph").show(ui, |ui| {
        row_label(ui, &tr("options.rows"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.rows", "options.rows_tip"),
            |ui| {
                ui.add(crate::ui::field::NumField::new(&mut app.graph_rows).range(1..=99));
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.columns"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.columns", "options.columns_tip"),
            |ui| {
                ui.add(crate::ui::field::NumField::new(&mut app.graph_cols).range(1..=99));
            },
        );

        ui.end_row();
    });
}

fn table_page(app: &mut App, ui: &mut Ui) {
    grid("table").show(ui, |ui| {
        row_label(ui, &tr("options.rows"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.rows", "options.rows_tip"),
            |ui| {
                ui.add(crate::ui::field::NumField::new(&mut app.table_rows).range(1..=99));
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.columns"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.columns", "options.columns_tip"),
            |ui| {
                ui.add(crate::ui::field::NumField::new(&mut app.table_cols).range(1..=99));
            },
        );

        ui.end_row();
        row_label(ui, &tr("options.table_tab_key"));
        ui.vertical(|ui| {
            let p = &mut app.settings.table;
            ui.radio_value(&mut p.tab_inserts_tab, false, tr("options.table_tab_moves"))
                .rich_tip(crate::ui::tips::pair(
                    "options.table_tab_moves",
                    "options.table_tab_moves_tip",
                ));
            ui.radio_value(
                &mut p.tab_inserts_tab,
                true,
                tr("options.table_tab_inserts"),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.table_tab_inserts",
                "options.table_tab_inserts_tip",
            ));
        });
        ui.end_row();
        row_label(ui, &tr("options.table_tab_order"));
        ui.vertical(|ui| {
            let p = &mut app.settings.table;
            ui.radio_value(&mut p.tab_by_columns, false, tr("options.table_tab_rows"))
                .rich_tip(crate::ui::tips::pair(
                    "options.table_tab_rows",
                    "options.table_tab_rows_tip",
                ));
            ui.radio_value(&mut p.tab_by_columns, true, tr("options.table_tab_columns"))
                .rich_tip(crate::ui::tips::pair(
                    "options.table_tab_columns",
                    "options.table_tab_columns_tip",
                ));
        });
        ui.end_row();
    });
}

/// Options > Tools > Curve tools: the Freehand smoothing, how near an
/// end node a new line joins the selected curve (and a curve closes on
/// its first node), and the Pen's preview mode and automatic adding and
/// deleting of nodes. See `prefs::CurvePrefs`.
fn curves_page(app: &mut App, ui: &mut Ui) {
    let c = &mut app.settings.curves;
    grid("curves").show(ui, |ui| {
        row_label(ui, &tr("options.freehand_smoothing"));
        let mut v = c.smoothing.min(100);
        ui.add(crate::ui::Slider::new(&mut v, 0..=100))
            .rich_tip(crate::ui::tips::pair(
                "options.freehand_smoothing",
                "options.freehand_smoothing_tip",
            ));
        c.smoothing = v;
        ui.end_row();
        row_label(ui, &tr("options.join_within"));
        let mut px = if c.join_px.is_finite() {
            c.join_px
        } else {
            5.0
        };
        ui.add(
            crate::ui::field::NumField::new(&mut px)
                .range(1.0..=50.0)
                .speed(0.2)
                .suffix(" px"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.join_within",
            "options.join_within_tip",
        ));
        c.join_px = px;
        ui.end_row();
        row_label(ui, &tr("options.pen"));
        ui.checkbox(&mut c.pen_preview, tr("options.pen_preview"))
            .rich_tip(crate::ui::tips::pair(
                "options.pen_preview",
                "options.pen_preview_tip",
            ));
        ui.end_row();
        ui.label("");
        ui.checkbox(
            &mut c.pen_auto_add_delete,
            tr("options.pen_auto_add_delete"),
        )
        .rich_tip(crate::ui::tips::pair(
            "options.pen_auto_add_delete",
            "options.pen_auto_add_delete_tip",
        ));
        ui.end_row();
        row_label(ui, &tr("options.pen_tablet"));
        ui.checkbox(&mut c.pen_pressure, tr("options.pen_pressure"))
            .rich_tip(crate::ui::tips::pair(
                "options.pen_pressure",
                "options.pen_pressure_tip",
            ));
        ui.end_row();
        row_label(ui, &tr("options.constrain_key"));
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.constrain_key", "options.constrain_key_tip"),
            |ui| {
                ui.horizontal(|ui| {
                    for k in crate::prefs::ConstrainKey::ALL {
                        ui.radio_value(&mut c.constrain_key, k, tr(k.i18n_key()))
                            .rich_tip_with(|| {
                                format!("{}\n{}", tr(k.i18n_key()), tr("options.constrain_key_tip"))
                            });
                    }
                });
            },
        );

        ui.end_row();
    });
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("options.join_hint")).color(crate::theme::Tokens::text_dim()));
}

fn eraser_page(app: &mut App, ui: &mut Ui) {
    grid("eraser").show(ui, |ui| {
        row_label(ui, &tr("options.eraser_width"));
        let u = app.units;
        let mut v = u.from_mm(app.eraser_width);
        if ui
            .add(
                crate::ui::field::NumField::new(&mut v)
                    .speed(0.1)
                    .suffix(format!(" {}", u.short())),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.eraser_width",
                "options.eraser_width_tip",
            ))
            .changed()
        {
            app.eraser_width = u.to_mm(v).max(0.01);
        }
        ui.end_row();
    });
    ui.radio_value(&mut app.eraser_square, false, tr("options.eraser_round"))
        .rich_tip(crate::ui::tips::pair(
            "options.eraser_round",
            "options.eraser_round_tip",
        ));
    ui.radio_value(&mut app.eraser_square, true, tr("options.eraser_square"))
        .rich_tip(crate::ui::tips::pair(
            "options.eraser_square",
            "options.eraser_square_tip",
        ));
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
            })
            .response
            .rich_tip(crate::ui::tips::pair(
                "options.language",
                "options.language_tip",
            ));
        ui.end_row();
    });
    // A copy from the Microsoft Store is updated by the Store.
    #[cfg(not(target_arch = "wasm32"))]
    if !crate::updates::store_install() {
        ui.checkbox(&mut app.settings.check_updates, tr("options.check_updates"))
            .rich_tip(tr("options.check_updates_tip"));
    }
    // AI agents on this computer and the open drawing (desktop only).
    #[cfg(not(target_arch = "wasm32"))]
    {
        chrome::section(ui, &tr("options.agents"));
        ui.checkbox(&mut app.settings.mcp_window, tr("options.mcp_window"))
            .rich_tip(crate::ui::tips::pair(
                "options.mcp_window",
                "options.mcp_window_tip",
            ));
        ui.label(egui::RichText::new(tr("options.mcp_window_hint")).color(Tokens::text_dim()));
        if let Some(link) = &app.mcp_link {
            ui.label(
                egui::RichText::new(crate::i18n::trf(
                    "options.mcp_window_port",
                    &[("port", &link.info.port.to_string())],
                ))
                .color(Tokens::text_dim()),
            );
        }
    }
    ui.add_space(10.0);
    ui.label(egui::RichText::new(tr("options.reset_all_hint")).color(Tokens::text_dim()));
    if ui
        .button(tr("options.reset_all"))
        .rich_tip(crate::ui::tips::pair(
            "options.reset_all",
            "options.reset_all_tip",
        ))
        .clicked()
    {
        // The question replaces the options; nothing changes unless it is
        // answered Reset.
        if let Some(snapshot) = app.options_snapshot.take() {
            snapshot.restore(app);
        }
        app.dialog = crate::ui::dialogs::Dialog::ConfirmResetSettings;
    }
}

fn file_locations_page(app: &mut App, ui: &mut Ui) {
    // The program Edit Bitmap opens pictures in (#593).
    if !crate::files::WEB {
        ui.label(tr("options.bitmap_editor"));
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.settings.bitmap_editor)
                    .desired_width(300.0)
                    .hint_text(tr("options.bitmap_editor_system")),
            )
            .rich_tip(crate::ui::tips::pair(
                "options.bitmap_editor",
                "options.bitmap_editor_tip",
            ));
            if ui
                .button(tr("options.browse"))
                .rich_tip(crate::ui::tips::pair(
                    "options.browse",
                    "options.browse_tip",
                ))
                .clicked()
            {
                crate::files::Dialog::new().pick_file(app, |app, path| {
                    app.settings.bitmap_editor = path.display().to_string();
                });
            }
            if ui
                .add_enabled(
                    !app.settings.bitmap_editor.is_empty(),
                    egui::Button::new(tr("options.clear_folder")),
                )
                .clicked()
            {
                app.settings.bitmap_editor.clear();
            }
        });
        ui.label(
            egui::RichText::new(tr("options.bitmap_editor_hint"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        ui.add_space(10.0);
    }
    ui.label(tr("options.save_hint"));
    if let Some(p) = crate::settings::Settings::path() {
        ui.label(
            egui::RichText::new(p.display().to_string())
                .color(Tokens::text_dim())
                .size(12.0),
        );
    }
    if let Some(p) = crate::app::App::auto_backup_dir_default() {
        ui.add_space(6.0);
        ui.label(tr("options.auto_backup"));
        ui.label(
            egui::RichText::new(p.display().to_string())
                .color(Tokens::text_dim())
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
        for ws in crate::app::Workspace::ALL {
            if ui
                .selectable_label(
                    app.workspace == ws,
                    tr(&format!("menu.window.workspace_{}", ws.id())),
                )
                .rich_tip_with(|| {
                    format!(
                        "{}\n{}",
                        tr(&format!("menu.window.workspace_{}", ws.id())),
                        tr("options.workspace_pick_tip")
                    )
                })
                .clicked()
            {
                app.set_workspace(ws);
            }
        }
    });
    ui.add_space(10.0);
    ui.strong(tr("options.workspace_transfer"));
    ui.label(egui::RichText::new(tr("options.workspace_transfer_hint")).color(Tokens::text_dim()));
    let mut parts = app.settings.workspace_parts;
    for (on, key) in parts.flags_mut() {
        ui.checkbox(on, tr(key))
            .rich_tip_with(|| format!("{}\n{}", tr(key), tr("options.workspace_part_tip")));
    }
    app.settings.workspace_parts = parts;
    ui.horizontal(|ui| {
        let any = parts.any();
        if ui
            .add_enabled(any, egui::Button::new(tr("options.workspace_export")))
            .rich_tip_always(crate::ui::tips::pair(
                "options.workspace_export",
                "options.workspace_export_tip",
            ))
            .clicked()
        {
            app.export_workspace_dialog(parts);
        }
        // Imported parts stay until OK; Cancel puts the old ones back.
        if ui
            .add_enabled(any, egui::Button::new(tr("options.workspace_import")))
            .rich_tip_always(crate::ui::tips::pair(
                "options.workspace_import",
                "options.workspace_import_tip",
            ))
            .clicked()
        {
            app.import_workspace_dialog(parts, false);
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
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair(
                "options.rendering_resolution",
                "options.rendering_resolution_tip",
            ),
            |ui| {
                ui.horizontal(|ui| {
                    let mut dpi = app.document_dpi();
                    if ui
                        .add_sized(
                            [80.0, 22.0],
                            crate::ui::field::NumField::new(&mut dpi).range(36.0..=2400.0),
                        )
                        .rich_tip(crate::ui::tips::pair(
                            "options.rendering_resolution",
                            "options.rendering_resolution_tip",
                        ))
                        .changed()
                    {
                        meta.resolution_dpi = dpi;
                    }
                    ui.label("dpi");
                });
            },
        );

        ui.end_row();
    });
    ui.checkbox(&mut meta.fill_open_curves, tr("options.fill_open_curves"))
        .rich_tip(crate::ui::tips::pair(
            "options.fill_open_curves",
            "options.fill_open_curves_tip",
        ));
    let mut inflate = !meta.no_auto_inflate;
    if ui
        .checkbox(&mut inflate, tr("options.auto_inflate_bitmaps"))
        .rich_tip(crate::ui::tips::pair(
            "options.auto_inflate_bitmaps",
            "options.auto_inflate_bitmaps_tip",
        ))
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
            u.world_from_mm(app.duplicate_offset.x),
            u.world_from_mm(app.duplicate_offset.y),
        );
        ui.horizontal(|ui| {
            let a = ui
                .add(
                    crate::ui::field::NumField::new(&mut dx)
                        .speed(0.1)
                        .suffix(format!(" {}", u.short())),
                )
                .rich_tip(crate::ui::tips::pair(
                    "options.duplicate_offset_x",
                    "options.duplicate_offset_x_tip",
                ));
            let b = ui
                .add(
                    crate::ui::field::NumField::new(&mut dy)
                        .speed(0.1)
                        .suffix(format!(" {}", u.short())),
                )
                .rich_tip(crate::ui::tips::pair(
                    "options.duplicate_offset_y",
                    "options.duplicate_offset_y_tip",
                ));
            if a.changed() || b.changed() {
                app.duplicate_offset =
                    tracedraw_core::geometry::Vec2::new(u.world_to_mm(dx), u.world_to_mm(dy));
            }
        });
        ui.end_row();
    });
}

fn layout_page(app: &mut App, ui: &mut Ui) {
    let n = app.doc().pages.len();
    ui.label(trf("options.pages_count", &[("n", &n.to_string())]));
    ui.checkbox(&mut app.page_sorter, tr("menu.view.page_sorter"))
        .rich_tip(crate::ui::tips::keyed("menu.view.page_sorter"));
    if ui
        .button(tr("menu.layout.switch_orientation"))
        .rich_tip(crate::ui::tips::pair(
            "menu.layout.switch_orientation",
            "menu.layout.switch_orientation_tip",
        ))
        .clicked()
    {
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
                if ui
                    .selectable_label(scope == s, tr(k))
                    .rich_tip_with(|| format!("{}\n{}", tr(k), tr(&format!("{k}_tip"))))
                    .clicked()
                {
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
            if ui
                .button(tr(k))
                .rich_tip_with(|| format!("{}\n{}", tr(k), tr(&format!("{k}_tip"))))
                .clicked()
            {
                crate::ui::objects_docker::new_master_layer(app, s);
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
        ui.radio_value(&mut kind, 0, tr("options.no_background"))
            .rich_tip(crate::ui::tips::pair(
                "options.no_background",
                "options.no_background_tip",
            ));
        ui.radio_value(&mut kind, 1, tr("options.solid"))
            .rich_tip(crate::ui::tips::pair("options.solid", "options.solid_tip"));
        ui.radio_value(&mut kind, 2, tr("options.bitmap"))
            .rich_tip(crate::ui::tips::pair(
                "options.bitmap",
                "options.bitmap_tip",
            ));
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
            let changed = crate::ui::color_picker::color_button(ui, &mut c);
            if changed || !matches!(current, Some(Fill::Solid(_))) {
                app.run(Command::SetPageBackground {
                    page,
                    background: Some(Fill::Solid(c)),
                });
            }
        }
        _ => {
            if ui
                .button(tr("options.choose_bitmap"))
                .rich_tip(crate::ui::tips::pair(
                    "options.choose_bitmap",
                    "options.choose_bitmap_tip",
                ))
                .clicked()
            {
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
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::pair("options.bleed", "options.bleed_tip"),
            |ui| {
                let mut b = app.units.from_mm(md.bleed);
                if ui
                    .add_sized(
                        [100.0, 22.0],
                        crate::ui::field::NumField::new(&mut b)
                            .speed(0.1)
                            .max_decimals(3)
                            .suffix(format!(" {}", app.units.short())),
                    )
                    .changed()
                {
                    md.bleed = app.units.to_mm(b).max(0.0);
                    app.run(Command::SetMetadata { metadata: md });
                };
            },
        );

        ui.end_row();
    });
    ui.checkbox(&mut app.show_bleed, tr("options.show_bleed"))
        .rich_tip(crate::ui::tips::pair(
            "options.show_bleed",
            "options.show_bleed_tip",
        ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #1148: every command can take a shortcut. A bar button that runs
    /// a menu command shows that command's keys; rows the menus show only
    /// at times (Finish Editing Contents, Vertical Text) are listed too.
    #[test]
    fn bar_buttons_and_rows_shown_at_times_take_shortcuts() {
        let mut app = App::headless();
        let ctx = egui::Context::default();
        let mut entries = Vec::new();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            entries = crate::ui::menus::commands(&mut app, ui.ctx());
        });
        out.textures_delta.clear();
        app.shortcut_editor.entries = entries;
        for key in [
            "menu.object.clip_frame_finish",
            "menu.object.clip_frame_edit",
            "menu.text.vertical_text",
        ] {
            assert!(
                app.shortcut_editor.entries.iter().any(|e| e.key == key),
                "{key} is not listed"
            );
        }
        // Each button's menu command is one the menus have.
        for id in crate::ui::bar_items::all_buttons() {
            if let Some(key) = crate::ui::bar_items::menu_command(id) {
                assert!(
                    app.shortcut_editor.entries.iter().any(|e| e.key == key),
                    "{id}: {key} is not a menu command"
                );
            }
        }
        let buttons = command_items(&app, Some(BUTTONS), "");
        let zoom_in = buttons
            .iter()
            .find(|c| c.id == "zoom_in")
            .cloned()
            .expect("zoom in button");
        let target = keys_target(&app, zoom_in);
        assert_eq!(target.id, "menu.view.zoom_in");
        assert!(!crate::ui::bar_items::is_button(&target.id));
        // A key set through the button runs the menu command.
        set_menu_shortcut(&mut app, &target.id, "Alt+Shift+Z");
        assert!(current_keys(&app, &target).contains(&"Alt+Shift+Z".to_string()));
        assert!(app
            .settings
            .shortcuts
            .iter()
            .any(|(k, v)| k == "menu.view.zoom_in" && v == "Alt+Shift+Z"));
        // A button the menus lack stays without keys.
        let levels = buttons
            .iter()
            .find(|c| c.id == "zoom_levels")
            .cloned()
            .expect("zoom levels");
        assert_eq!(keys_target(&app, levels).id, "zoom_levels");
    }

    #[test]
    fn command_list_filters_by_menu_toolbox_and_text() {
        let mut app = App::headless();
        let ctx = egui::Context::default();
        let mut entries = Vec::new();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            entries = crate::ui::menus::commands(&mut app, ui.ctx());
        });
        out.textures_delta.clear();
        app.shortcut_editor.entries = entries;
        let all = command_items(&app, None, "");
        let file = command_items(&app, Some("menu.file"), "");
        let tools = command_items(&app, Some(TOOLBOX), "");
        assert!(file
            .iter()
            .any(|c| c.id == "menu.file.open" && c.built_in == "Ctrl+O"));
        assert!(file.iter().all(|c| c.tool.is_none()));
        assert_eq!(tools.len(), crate::tools::Tool::ALL.len());
        let buttons = command_items(&app, Some(BUTTONS), "");
        assert_eq!(
            all.len(),
            app.shortcut_editor.entries.len() + tools.len() + buttons.len()
        );
        let found = command_items(&app, None, "open");
        assert!(!found.is_empty() && found.len() < all.len());
        // A user's key shows beside the built-in one (#822).
        let open = file
            .iter()
            .find(|c| c.id == "menu.file.open")
            .cloned()
            .expect("open");
        set_menu_shortcut(&mut app, "menu.file.open", "Ctrl+Shift+O");
        assert_eq!(current_keys(&app, &open), ["Ctrl+O", "Ctrl+Shift+O"]);
    }

    /// The app with the menu commands collected, as the Commands page has.
    fn app_with_commands() -> App {
        let mut app = App::headless();
        let ctx = egui::Context::default();
        let mut entries = Vec::new();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            entries = crate::ui::menus::commands(&mut app, ui.ctx());
        });
        out.textures_delta.clear();
        app.shortcut_editor.entries = entries;
        app
    }

    #[test]
    fn a_built_in_shortcut_can_be_removed() {
        let mut app = app_with_commands();
        let file = command_items(&app, Some("menu.file"), "");
        let open = file
            .iter()
            .find(|c| c.id == "menu.file.open")
            .cloned()
            .expect("open");
        assert_eq!(current_keys(&app, &open), vec!["Ctrl+O".to_string()]);
        // Delete on a built-in shortcut: an empty one overrides it.
        remove_shortcut(&mut app, &open);
        assert!(current_keys(&app, &open).is_empty());
        assert!(app
            .settings
            .shortcuts
            .contains(&("menu.file.open".to_string(), String::new())));
        // A user's own key on a command with none built in just goes.
        let tool = command_items(&app, Some(TOOLBOX), "")
            .into_iter()
            .find(|c| c.built_in.is_empty())
            .expect("a tool without a built-in key");
        set_menu_shortcut(&mut app, &tool.id, "Alt+Shift+K");
        assert_eq!(current_keys(&app, &tool), vec!["Alt+Shift+K".to_string()]);
        remove_shortcut(&mut app, &tool);
        assert!(current_keys(&app, &tool).is_empty());
        assert!(!app.settings.shortcuts.iter().any(|(k, _)| k == &tool.id));
    }

    /// A button taken off a bar is listed under Toolbar buttons and goes
    /// back on when dragged there; every built-in button has a name
    /// (#836).
    #[test]
    fn a_removed_toolbar_button_can_be_put_back() {
        use crate::ui::bar_items::{apply_drop, items, ItemBar, ItemDrag};
        let mut app = app_with_commands();
        let zoom = items(&app, ItemBar::Zoom);
        let i = zoom
            .iter()
            .position(|x| x == "zoom_page")
            .expect("on the bar");
        // Dragged off the bar.
        apply_drop(
            &mut app,
            &ItemDrag {
                item: "zoom_page".into(),
                from: Some((ItemBar::Zoom, i)),
            },
            None,
        );
        assert!(!items(&app, ItemBar::Zoom).contains(&"zoom_page".to_string()));
        let buttons = command_items(&app, Some(BUTTONS), "");
        assert!(buttons.iter().all(|c| c.name != c.id), "every button named");
        let page = buttons
            .iter()
            .find(|c| c.id == "zoom_page")
            .expect("listed");
        assert_eq!(page.name, tr("toolbar.zoom_to_page"));
        assert!(!command_items(&app, None, &page.name.to_lowercase()).is_empty());
        // Dragged from the list back onto the bar.
        apply_drop(
            &mut app,
            &ItemDrag {
                item: page.id.clone(),
                from: None,
            },
            Some((ItemBar::Zoom, i)),
        );
        assert_eq!(items(&app, ItemBar::Zoom), zoom);
    }

    #[test]
    fn reset_to_defaults_touches_the_category_shown_only() {
        let mut app = app_with_commands();
        set_menu_shortcut(&mut app, "menu.file.open", "Ctrl+Shift+O");
        set_menu_shortcut(&mut app, "rectangle", "Alt+R");
        reset_category_shortcuts(&mut app, TOOLBOX);
        // The tool's keys went, the menu command's stayed.
        assert_eq!(
            app.settings.shortcuts,
            vec![
                ("menu.file.open".to_string(), "Ctrl+O".to_string()),
                ("menu.file.open".to_string(), "Ctrl+Shift+O".to_string())
            ]
        );
        reset_category_shortcuts(&mut app, "menu.file");
        assert!(app.settings.shortcuts.is_empty());
    }

    #[test]
    fn customization_pages_center_dialogs_and_reset_bars() {
        use OptionsPage::*;
        assert_eq!(
            OptionsSet::Customization.pages(),
            &[Appearance, CommandBars, Commands, ColorPalette]
        );
        let mut app = App::headless();
        assert!(app.settings.center_dialogs);
        // Dialogs follow the setting given for the frame.
        let ctx = egui::Context::default();
        assert!(crate::ui::chrome::center_dialogs(&ctx));
        crate::ui::chrome::set_center_dialogs(&ctx, false);
        assert!(!crate::ui::chrome::center_dialogs(&ctx));
        // Reset brings back the bars of a new installation.
        app.show_toolbox = false;
        app.show_zoom_toolbar = true;
        app.settings.palette.show_palette = false;
        app.settings.bars.locked = true;
        app.reset_command_bars();
        assert!(app.show_toolbox && !app.show_zoom_toolbar);
        assert!(app.settings.palette.show_palette && !app.settings.bars.locked);
    }

    #[test]
    fn every_page_belongs_to_one_dialog() {
        let sets = [
            OptionsSet::App,
            OptionsSet::Customization,
            OptionsSet::Tools,
            OptionsSet::Global,
            OptionsSet::Workspaces,
            OptionsSet::Ai,
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
    fn every_text_tab_draws() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.options_page = OptionsPage::Text;
        let before = app.settings.autocorrect.clone();
        for tab in 0..TEXT_TABS.len() {
            app.options_text_tab = tab;
            for _ in 0..2 {
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
                    assert!(!close, "tab {tab} closed the dialog");
                });
                out.textures_delta.clear();
            }
        }
        assert_eq!(
            app.settings.autocorrect, before,
            "drawing changed a setting"
        );
        assert_eq!(app.settings.recent_font_count, 5);
    }

    /// Text > Writing Tools > Autocorrect opens the Autocorrect tab.
    #[test]
    fn the_autocorrect_command_opens_its_tab() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let mut ran = false;
        let input = egui::RawInput::default();
        let mut out = ctx.run_ui(input, |ui| {
            ran = crate::ui::menus::run_command(&mut app, ui.ctx(), "menu.text.autocorrect");
        });
        out.textures_delta.clear();
        assert!(ran);
        assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::Options));
        assert_eq!(app.options_page, OptionsPage::Text);
        assert_eq!(app.options_text_tab, AUTOCORRECT_TAB);
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
            OptionsSet::Ai,
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

    /// A frame of the whole window with `events` as its input.
    fn frame(ctx: &Context, app: &mut App, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1000.0),
            )),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
        out.textures_delta.clear();
    }

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn commands_take_the_keys_pressed_as_their_shortcut() {
        use crate::ui::dialogs::Dialog;
        use egui::{Key, Modifiers};
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.dialog = Dialog::Options;
        app.options_page = OptionsPage::Commands;
        frame(&ctx, &mut app, vec![]);
        assert!(app
            .shortcut_editor
            .entries
            .iter()
            .any(|e| e.key == "menu.file.export"));
        // Export waits for its keys; Ctrl+D, Duplicate's, is pressed.
        app.shortcut_editor.capture = Some("menu.file.export".into());
        frame(
            &ctx,
            &mut app,
            vec![key(Key::ControlLeft, Modifiers::COMMAND)],
        );
        assert!(app.shortcut_editor.capture.is_some(), "Ctrl alone waits");
        frame(&ctx, &mut app, vec![key(Key::D, Modifiers::COMMAND)]);
        // The new key joins the built-in one: a command may have several
        // (#822).
        let export_keys = |app: &App| {
            crate::shortcuts::keys_for(&app.settings.shortcuts, "menu.file.export", None)
        };
        assert_eq!(export_keys(&app), ["Ctrl+E", "Ctrl+D"]);
        assert!(app.shortcut_editor.capture.is_none());
        let note = app.shortcut_editor.note.clone().unwrap_or_default();
        assert!(note.contains(&tr("menu.edit.duplicate")), "{note}");
        // Keys that edit text or close dialogs are not shortcuts alone.
        app.shortcut_editor.capture = Some("menu.file.import".into());
        frame(&ctx, &mut app, vec![key(Key::Enter, Modifiers::NONE)]);
        assert!(user_shortcut(&app, "menu.file.import").is_none());
        // Esc stops waiting and leaves the options open.
        frame(&ctx, &mut app, vec![key(Key::Escape, Modifiers::NONE)]);
        assert!(app.shortcut_editor.capture.is_none());
        assert!(matches!(app.dialog, Dialog::Options));
        // Keys of the user's move to the command given them last.
        set_menu_shortcut(&mut app, "menu.file.import", "ctrl+d");
        assert_eq!(export_keys(&app), ["Ctrl+E"]);
        assert!(app
            .shortcut_editor
            .note
            .as_deref()
            .is_some_and(|n| n.contains(&tr("menu.file.export").replace("...", ""))));
        // Another page, or no options, and nothing waits any more.
        app.shortcut_editor.capture = Some("menu.file.export".into());
        app.options_page = OptionsPage::General;
        frame(&ctx, &mut app, vec![]);
        assert!(app.shortcut_editor.capture.is_none());
        app.options_page = OptionsPage::Commands;
        app.shortcut_editor.capture = Some("menu.file.export".into());
        app.dialog = Dialog::None;
        frame(&ctx, &mut app, vec![key(Key::F7, Modifiers::COMMAND)]);
        assert!(app.shortcut_editor.capture.is_none());
        assert_eq!(export_keys(&app), ["Ctrl+E"]);
    }

    /// A tool takes a second key without losing its first; either picks
    /// it, and each is deleted on its own (#822).
    #[test]
    fn a_tool_keeps_several_shortcuts() {
        let mut app = app_with_commands();
        let rect = command_items(&app, Some(TOOLBOX), "")
            .into_iter()
            .find(|c| c.id == "rectangle")
            .expect("rectangle tool");
        let built_in = rect.built_in.clone();
        assert!(!built_in.is_empty());
        set_menu_shortcut(&mut app, "rectangle", "Alt+R");
        set_menu_shortcut(&mut app, "rectangle", "Alt+Shift+R");
        assert_eq!(
            current_keys(&app, &rect),
            [built_in.clone(), "Alt+R".into(), "Alt+Shift+R".into()]
        );
        remove_key(&mut app, &rect, "Alt+R");
        assert_eq!(
            current_keys(&app, &rect),
            [built_in.clone(), "Alt+Shift+R".into()]
        );
        remove_key(&mut app, &rect, &built_in);
        assert_eq!(current_keys(&app, &rect), ["Alt+Shift+R".to_string()]);
        remove_key(&mut app, &rect, "Alt+Shift+R");
        assert!(
            current_keys(&app, &rect).is_empty(),
            "the built-in stays deleted"
        );
    }
}
