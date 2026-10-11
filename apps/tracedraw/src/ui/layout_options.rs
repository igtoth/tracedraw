//! Rulers, grids and guidelines: the Rulers, Grid and Guidelines pages of
//! Document Options and the Guidelines docker. Every document change goes
//! through a command (SetMetadata for the settings, the guideline commands
//! for guidelines).

use crate::app::{App, Units};
use crate::guides::GuidePreset;
use crate::i18n::tr;
use crate::theme::Tokens;
use egui::Ui;
use tracedraw_core::{
    document::{GridDisplay, Guide, GuideLine, GuideStyle, Metadata},
    geometry::Point,
    Color, Command,
};

/// What the Guidelines docker's form edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GuideKind {
    #[default]
    Horizontal,
    Vertical,
    Angled,
}

impl GuideKind {
    pub const ALL: [GuideKind; 3] = [
        GuideKind::Horizontal,
        GuideKind::Vertical,
        GuideKind::Angled,
    ];

    fn key(self) -> &'static str {
        match self {
            GuideKind::Horizontal => "options.horizontal",
            GuideKind::Vertical => "options.vertical",
            GuideKind::Angled => "options.angled",
        }
    }

    pub fn of(line: &GuideLine) -> GuideKind {
        match line {
            GuideLine::Horizontal { .. } => GuideKind::Horizontal,
            GuideLine::Vertical { .. } => GuideKind::Vertical,
            GuideLine::Angled { .. } => GuideKind::Angled,
        }
    }
}

/// The Guidelines docker's entry fields, counted from the ruler origin
/// (mm) and in degrees.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GuideForm {
    pub kind: GuideKind,
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

impl GuideForm {
    /// The guideline the form describes, in page coordinates.
    pub fn line(&self, app: &App) -> GuideLine {
        let p = app.from_ruler(Point::new(self.x, self.y));
        match self.kind {
            GuideKind::Horizontal => GuideLine::Horizontal { y: p.y },
            GuideKind::Vertical => GuideLine::Vertical { x: p.x },
            GuideKind::Angled => GuideLine::Angled {
                x: p.x,
                y: p.y,
                angle: self.angle,
            },
        }
    }

    /// Fill the form from a guideline.
    pub fn load(&mut self, app: &App, line: &GuideLine) {
        self.kind = GuideKind::of(line);
        let (o, _) = line.point_and_direction();
        let r = app.to_ruler(o);
        match line {
            GuideLine::Horizontal { .. } => self.y = r.y,
            GuideLine::Vertical { .. } => self.x = r.x,
            GuideLine::Angled { angle, .. } => {
                self.x = r.x;
                self.y = r.y;
                self.angle = *angle;
            }
        }
    }
}

/// The presets of Document Options > Guidelines > Presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedPreset {
    OneInchMargins,
    BleedArea,
    PageBorders,
    PrintableArea,
    ThreeColumnNewsletter,
    BasicGrid,
    UpperLeftGrid,
}

impl NamedPreset {
    pub const ALL: [NamedPreset; 7] = [
        NamedPreset::OneInchMargins,
        NamedPreset::BleedArea,
        NamedPreset::PageBorders,
        NamedPreset::PrintableArea,
        NamedPreset::ThreeColumnNewsletter,
        NamedPreset::BasicGrid,
        NamedPreset::UpperLeftGrid,
    ];

    fn key(self) -> &'static str {
        match self {
            NamedPreset::OneInchMargins => "guides.preset_one_inch_margins",
            NamedPreset::BleedArea => "guides.preset_bleed_area",
            NamedPreset::PageBorders => "guides.preset_page_borders",
            NamedPreset::PrintableArea => "guides.preset_printable_area",
            NamedPreset::ThreeColumnNewsletter => "guides.preset_three_columns",
            NamedPreset::BasicGrid => "guides.preset_basic_grid",
            NamedPreset::UpperLeftGrid => "guides.preset_upper_left_grid",
        }
    }

    /// The guideline sets this preset adds, for a page of `size` with
    /// `bleed` around it (mm).
    pub fn presets(self, size: tracedraw_core::geometry::Size, bleed: f64) -> Vec<GuidePreset> {
        let inch = 25.4;
        match self {
            NamedPreset::OneInchMargins => vec![GuidePreset::Margins {
                top: inch,
                bottom: inch,
                left: inch,
                right: inch,
            }],
            NamedPreset::BleedArea => vec![GuidePreset::Margins {
                top: -bleed,
                bottom: -bleed,
                left: -bleed,
                right: -bleed,
            }],
            NamedPreset::PageBorders => vec![GuidePreset::Margins {
                top: 0.0,
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
            }],
            NamedPreset::PrintableArea => vec![GuidePreset::Margins {
                top: 5.0,
                bottom: 5.0,
                left: 5.0,
                right: 5.0,
            }],
            NamedPreset::ThreeColumnNewsletter => vec![
                GuidePreset::Margins {
                    top: inch,
                    bottom: inch,
                    left: inch,
                    right: inch,
                },
                GuidePreset::Columns {
                    count: 3,
                    gutter: inch / 4.0,
                },
            ],
            NamedPreset::BasicGrid => vec![GuidePreset::Grid {
                rows: (size.height / inch).floor().max(1.0) as u32,
                columns: (size.width / inch).floor().max(1.0) as u32,
            }],
            NamedPreset::UpperLeftGrid => vec![GuidePreset::Grid {
                rows: 4,
                columns: 4,
            }],
        }
    }
}

/// Lines of a named preset for a page; columns are inset by the margins
/// of the newsletter preset, the upper-left grid covers the page's
/// upper-left quarter.
pub fn preset_lines(
    preset: NamedPreset,
    size: tracedraw_core::geometry::Size,
    bleed: f64,
) -> Vec<GuideLine> {
    let inch = 25.4;
    match preset {
        NamedPreset::ThreeColumnNewsletter => {
            let mut out = GuidePreset::Margins {
                top: inch,
                bottom: inch,
                left: inch,
                right: inch,
            }
            .lines(size);
            let inner = tracedraw_core::geometry::Size::new(
                (size.width - 2.0 * inch).max(0.0),
                size.height,
            );
            for l in (GuidePreset::Columns {
                count: 3,
                gutter: inch / 4.0,
            })
            .lines(inner)
            {
                if let GuideLine::Vertical { x } = l {
                    out.push(GuideLine::Vertical { x: x + inch });
                }
            }
            out
        }
        NamedPreset::UpperLeftGrid => {
            let quarter = tracedraw_core::geometry::Size::new(size.width / 2.0, size.height / 2.0);
            GuidePreset::Grid {
                rows: 4,
                columns: 4,
            }
            .lines(quarter)
            .into_iter()
            .map(|l| match l {
                GuideLine::Horizontal { y } => GuideLine::Horizontal {
                    y: y + size.height / 2.0,
                },
                other => other,
            })
            .collect()
        }
        other => other
            .presets(size, bleed)
            .into_iter()
            .flat_map(|p| p.lines(size))
            .collect(),
    }
}

/// State of the Guidelines page's preset section.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetForm {
    pub user_defined: bool,
    pub named: Vec<NamedPreset>,
    pub margins: bool,
    pub margin_top: f64,
    pub margin_bottom: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    pub columns: bool,
    pub column_count: u32,
    pub column_gutter: f64,
    pub grid: bool,
    pub grid_rows: u32,
    pub grid_columns: u32,
}

impl Default for PresetForm {
    fn default() -> Self {
        PresetForm {
            user_defined: false,
            named: Vec::new(),
            margins: true,
            margin_top: 12.7,
            margin_bottom: 12.7,
            margin_left: 12.7,
            margin_right: 12.7,
            columns: false,
            column_count: 2,
            column_gutter: 6.35,
            grid: false,
            grid_rows: 4,
            grid_columns: 4,
        }
    }
}

fn set_metadata(app: &mut App, metadata: Metadata) {
    if metadata != app.doc().metadata {
        app.run(Command::SetMetadata { metadata });
    }
}

/// A row label right-aligned in the first grid column.
fn row_label(ui: &mut Ui, key: &str) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(tr(key));
    });
}

/// A distance field in the ruler unit; returns the new value in mm when
/// it changed.
fn distance_field(ui: &mut Ui, units: Units, mm: f64, speed: f64) -> Option<f64> {
    let mut v = units.from_mm(mm);
    let r = ui.add_sized(
        [110.0, 20.0],
        egui::DragValue::new(&mut v)
            .speed(speed)
            .max_decimals(4)
            .suffix(format!(" {}", units.short())),
    );
    r.changed().then(|| units.to_mm(v))
}

fn units_combo(ui: &mut Ui, id: &str, units: &mut Units) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(units.label())
        .width(160.0)
        .show_ui(ui, |ui| {
            for u in Units::ALL {
                if ui.selectable_label(*units == u, u.label()).clicked() {
                    *units = u;
                    changed = true;
                }
            }
        });
    changed
}

/// A colour button; returns the new colour when it changed.
fn color_button(ui: &mut Ui, c: Color) -> Option<Color> {
    let [r, g, b] = c.to_rgb8();
    let mut rgb = [r, g, b];
    ui.color_edit_button_srgb(&mut rgb)
        .changed()
        .then(|| Color::rgb8(rgb[0], rgb[1], rgb[2]))
}

fn section(ui: &mut Ui, key: &str) {
    ui.add_space(6.0);
    ui.label(tr(key));
    ui.separator();
}

/// Document Options > Rulers.
pub fn rulers_page(app: &mut App, ui: &mut Ui) {
    let mut meta = app.doc().metadata.clone();
    let units = app.units;
    section(ui, "options.units_section");
    egui::Grid::new("rulers_units")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            row_label(ui, "options.units");
            let mut u = app.units;
            if units_combo(ui, "ruler_units", &mut u) {
                app.units = u;
            }
            ui.end_row();
        });
    section(ui, "options.origin");
    egui::Grid::new("rulers_origin")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            row_label(ui, "options.horizontal_origin");
            if let Some(v) = distance_field(ui, units, meta.rulers.origin_x, 0.1) {
                meta.rulers.origin_x = v;
            }
            ui.end_row();
            row_label(ui, "options.vertical_origin");
            if let Some(v) = distance_field(ui, units, meta.rulers.origin_y, 0.1) {
                meta.rulers.origin_y = v;
            }
            ui.end_row();
        });
    section(ui, "options.tick_divisions");
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let mut n = meta.rulers.tick_divisions;
        if ui
            .add_sized([70.0, 20.0], egui::DragValue::new(&mut n).range(1..=100))
            .changed()
        {
            meta.rulers.tick_divisions = n;
        }
        ui.label(tr("options.per_tick"));
    });
    section(ui, "options.nudge_section");
    egui::Grid::new("rulers_nudge")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            row_label(ui, "options.nudge_label");
            if let Some(v) = distance_field(ui, units, app.nudge_mm, 0.01) {
                if v > 0.0 {
                    app.nudge_mm = v;
                }
            }
            ui.end_row();
            row_label(ui, "options.super_nudge");
            if let Some(v) = distance_field(ui, units, app.settings.super_nudge_mm, 0.01) {
                if v > 0.0 {
                    app.settings.super_nudge_mm = v;
                }
            }
            ui.end_row();
            row_label(ui, "options.micro_nudge");
            if let Some(v) = distance_field(ui, units, app.settings.micro_nudge_mm, 0.001) {
                if v > 0.0 {
                    app.settings.micro_nudge_mm = v;
                }
            }
            ui.end_row();
        });
    ui.add_space(8.0);
    ui.checkbox(&mut app.show_rulers, tr("options.show_rulers"));
    if ui.button(tr("options.reset_origin")).clicked() {
        meta.rulers.origin_x = 0.0;
        meta.rulers.origin_y = 0.0;
    }
    set_metadata(app, meta);
}

/// Document Options > Grid.
pub fn grid_page(app: &mut App, ui: &mut Ui) {
    let mut meta = app.doc().metadata.clone();
    let units = app.units;
    section(ui, "options.document_grid");
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.show_grid, tr("options.show_grid"));
        ui.add_space(24.0);
        ui.checkbox(&mut app.snap.grid, tr("options.snap_to_grid"));
    });
    egui::Grid::new("grid_spacing")
        .num_columns(3)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            for (key, value, salt) in [
                ("options.horizontal", &mut meta.grid.spacing_x, "gx"),
                ("options.vertical", &mut meta.grid.spacing_y, "gy"),
            ] {
                row_label(ui, key);
                if app.grid_frequency {
                    // Lines per unit.
                    let mut f = units.mm() / value.max(1e-9);
                    if ui
                        .add_sized(
                            [110.0, 20.0],
                            egui::DragValue::new(&mut f)
                                .speed(0.05)
                                .max_decimals(4)
                                .range(0.0001..=10_000.0),
                        )
                        .changed()
                        && f > 0.0
                    {
                        *value = units.mm() / f;
                    }
                } else if let Some(v) = distance_field(ui, units, *value, 0.1) {
                    if v > 0.0 {
                        *value = v;
                    }
                }
                let spacing = crate::i18n::trf("options.grid_spacing_of", &[("u", &units.label())]);
                let frequency =
                    crate::i18n::trf("options.grid_frequency_of", &[("u", &units.label())]);
                egui::ComboBox::from_id_salt(salt)
                    .selected_text(if app.grid_frequency {
                        frequency.clone()
                    } else {
                        spacing.clone()
                    })
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.grid_frequency, false, spacing);
                        ui.selectable_value(&mut app.grid_frequency, true, frequency);
                    });
                ui.end_row();
            }
            row_label(ui, "options.show_grid_as");
            ui.horizontal(|ui| {
                ui.radio_value(
                    &mut meta.grid.display,
                    GridDisplay::Lines,
                    tr("options.grid_lines"),
                );
                ui.radio_value(
                    &mut meta.grid.display,
                    GridDisplay::Dots,
                    tr("options.grid_dots"),
                );
            });
            ui.end_row();
        });

    section(ui, "options.baseline_grid");
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.show_baseline_grid, tr("options.show_grid"));
        ui.add_space(24.0);
        ui.checkbox(&mut app.snap.baseline_grid, tr("options.snap_to_grid"));
    });
    egui::Grid::new("baseline_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            row_label(ui, "options.baseline_spacing");
            // Spacing in points, as text leading is.
            let mut pt = meta.grid.baseline_spacing * 72.0 / 25.4;
            if ui
                .add_sized(
                    [110.0, 20.0],
                    egui::DragValue::new(&mut pt)
                        .speed(0.1)
                        .max_decimals(2)
                        .range(0.1..=1000.0)
                        .suffix(" pt"),
                )
                .changed()
            {
                meta.grid.baseline_spacing = pt * 25.4 / 72.0;
            }
            ui.end_row();
            row_label(ui, "options.baseline_start");
            if let Some(v) = distance_field(ui, units, meta.grid.baseline_start, 0.1) {
                meta.grid.baseline_start = v.max(0.0);
            }
            ui.end_row();
            row_label(ui, "options.line_color");
            if let Some(c) = color_button(ui, meta.grid.baseline_color) {
                meta.grid.baseline_color = c;
            }
            ui.end_row();
        });

    section(ui, "options.pixel_grid");
    ui.spacing_mut().slider_width = 150.0;
    egui::Grid::new("pixel_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            row_label(ui, "options.color");
            if let Some(c) = color_button(ui, meta.grid.pixel_color) {
                meta.grid.pixel_color = c;
            }
            ui.end_row();
            row_label(ui, "options.opacity");
            let mut o = meta.grid.pixel_opacity * 100.0;
            if ui
                .add(crate::ui::Rail(
                    egui::Slider::new(&mut o, 0.0..=100.0).suffix(" %"),
                ))
                .changed()
            {
                meta.grid.pixel_opacity = (o / 100.0).clamp(0.0, 1.0);
            }
            ui.end_row();
        });
    ui.checkbox(&mut app.show_pixel_grid, tr("options.pixel_grid_800"));
    ui.checkbox(&mut app.snap.pixels, tr("options.snap_to_pixels"));
    set_metadata(app, meta);
}

/// Document Options > Guidelines: visibility, snapping, default colours,
/// presets, and every guideline of the page.
pub fn guidelines_page(app: &mut App, ui: &mut Ui) {
    let mut meta = app.doc().metadata.clone();
    ui.horizontal(|ui| {
        ui.checkbox(&mut app.show_guides, tr("options.show_guidelines"));
        ui.add_space(24.0);
        ui.checkbox(&mut app.snap.guides, tr("options.snap_to_guidelines"));
    });
    egui::Grid::new("guide_colors")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            row_label(ui, "options.default_guide_color");
            if let Some(c) = color_button(ui, meta.guides.color) {
                meta.guides.color = c;
            }
            ui.end_row();
            row_label(ui, "options.default_preset_guide_color");
            if let Some(c) = color_button(ui, meta.guides.preset_color) {
                meta.guides.preset_color = c;
            }
            ui.end_row();
        });
    set_metadata(app, meta);

    section(ui, "options.guide_presets");
    let mut form = std::mem::take(&mut app.guide_presets);
    ui.horizontal(|ui| {
        ui.radio_value(&mut form.user_defined, false, tr("guides.presets"));
        ui.radio_value(&mut form.user_defined, true, tr("guides.user_defined"));
    });
    let units = app.units;
    if form.user_defined {
        ui.checkbox(&mut form.margins, tr("guides.margins"));
        ui.add_enabled_ui(form.margins, |ui| {
            egui::Grid::new("preset_margins")
                .num_columns(4)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    row_label(ui, "guides.top");
                    if let Some(v) = distance_field(ui, units, form.margin_top, 0.1) {
                        form.margin_top = v;
                    }
                    row_label(ui, "guides.left");
                    if let Some(v) = distance_field(ui, units, form.margin_left, 0.1) {
                        form.margin_left = v;
                    }
                    ui.end_row();
                    row_label(ui, "guides.bottom");
                    if let Some(v) = distance_field(ui, units, form.margin_bottom, 0.1) {
                        form.margin_bottom = v;
                    }
                    row_label(ui, "guides.right");
                    if let Some(v) = distance_field(ui, units, form.margin_right, 0.1) {
                        form.margin_right = v;
                    }
                    ui.end_row();
                });
        });
        ui.checkbox(&mut form.columns, tr("guides.columns"));
        ui.add_enabled_ui(form.columns, |ui| {
            egui::Grid::new("preset_columns")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    row_label(ui, "guides.number_of_columns");
                    ui.add_sized(
                        [70.0, 20.0],
                        egui::DragValue::new(&mut form.column_count).range(1..=100),
                    );
                    ui.end_row();
                    row_label(ui, "guides.distance_between");
                    if let Some(v) = distance_field(ui, units, form.column_gutter, 0.1) {
                        form.column_gutter = v.max(0.0);
                    }
                    ui.end_row();
                });
        });
        ui.checkbox(&mut form.grid, tr("guides.grid"));
        ui.add_enabled_ui(form.grid, |ui| {
            egui::Grid::new("preset_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    row_label(ui, "guides.rows");
                    ui.add_sized(
                        [70.0, 20.0],
                        egui::DragValue::new(&mut form.grid_rows).range(1..=200),
                    );
                    ui.end_row();
                    row_label(ui, "guides.columns_count");
                    ui.add_sized(
                        [70.0, 20.0],
                        egui::DragValue::new(&mut form.grid_columns).range(1..=200),
                    );
                    ui.end_row();
                });
        });
    } else {
        for p in NamedPreset::ALL {
            let mut on = form.named.contains(&p);
            if ui.checkbox(&mut on, tr(p.key())).changed() {
                if on {
                    form.named.push(p);
                } else {
                    form.named.retain(|q| *q != p);
                }
            }
        }
    }
    if ui.button(tr("guides.apply_presets")).clicked() {
        apply_presets(app, &form);
    }
    app.guide_presets = form;

    section(ui, "options.guidelines_list");
    guide_list(app, ui, None, 220.0);
}

/// Add the guidelines a preset form describes, in the preset colour.
pub fn apply_presets(app: &mut App, form: &PresetForm) {
    let size = app.page_size();
    let bleed = app.doc().metadata.bleed.max(0.0);
    let color = app.doc().metadata.guides.preset_color;
    let mut lines: Vec<GuideLine> = Vec::new();
    if form.user_defined {
        if form.margins {
            lines.extend(
                GuidePreset::Margins {
                    top: form.margin_top,
                    bottom: form.margin_bottom,
                    left: form.margin_left,
                    right: form.margin_right,
                }
                .lines(size),
            );
        }
        if form.columns {
            // Columns between the margins when margins are on.
            let (l, r) = if form.margins {
                (form.margin_left, form.margin_right)
            } else {
                (0.0, 0.0)
            };
            let inner =
                tracedraw_core::geometry::Size::new((size.width - l - r).max(0.0), size.height);
            for line in (GuidePreset::Columns {
                count: form.column_count,
                gutter: form.column_gutter,
            })
            .lines(inner)
            {
                if let GuideLine::Vertical { x } = line {
                    lines.push(GuideLine::Vertical { x: x + l });
                }
            }
        }
        if form.grid {
            lines.extend(
                GuidePreset::Grid {
                    rows: form.grid_rows,
                    columns: form.grid_columns,
                }
                .lines(size),
            );
        }
    } else {
        for p in &form.named {
            lines.extend(preset_lines(*p, size, bleed));
        }
    }
    let mut guides = app.page_guides();
    for line in lines {
        if !guides.iter().any(|g| g.line == line) {
            let mut g = Guide::from(line);
            g.color = Some(color);
            guides.push(g);
        }
    }
    app.set_guides(guides, "Add Guideline");
}

/// Text for a guideline in the lists: its position from the ruler origin
/// (and angle).
fn guide_text(app: &App, g: &Guide) -> String {
    let u = app.units;
    let (o, _) = g.line.point_and_direction();
    let r = app.to_ruler(o);
    let fmt = |v: f64| format!("{:.3} {}", u.from_mm(v), u.short());
    match g.line {
        GuideLine::Horizontal { .. } => fmt(r.y),
        GuideLine::Vertical { .. } => fmt(r.x),
        GuideLine::Angled { angle, .. } => format!("{}, {}, {:.1}°", fmt(r.x), fmt(r.y), angle),
    }
}

/// A selectable list of the page's guidelines (only those of `kind` when
/// given). Clicking selects the guideline on the page too.
fn guide_list(app: &mut App, ui: &mut Ui, kind: Option<GuideKind>, height: f32) {
    let guides = app.page_guides();
    let frame = egui::Frame::new()
        .fill(egui::Color32::WHITE)
        .stroke(egui::Stroke::new(1.0, Tokens::CONTROL_BORDER))
        .inner_margin(egui::Margin::same(2));
    frame.show(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(("guide_list", kind.map(|k| k as u8)))
            .max_height(height)
            .min_scrolled_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for (i, g) in guides.iter().enumerate() {
                    if kind.is_some_and(|k| k != GuideKind::of(&g.line)) {
                        continue;
                    }
                    let selected = app.selected_guides.contains(&i);
                    let mut text = guide_text(app, g);
                    if kind.is_none() {
                        text = format!("{}  {}", tr(GuideKind::of(&g.line).key()), text);
                    }
                    if g.locked {
                        text.push_str(&format!("  ({})", tr("guides.locked")));
                    }
                    let r = ui.selectable_label(selected, text);
                    if r.clicked() {
                        let ctrl = ui.input(|i| i.modifiers.command);
                        if ctrl {
                            if selected {
                                app.selected_guides.retain(|j| *j != i);
                            } else {
                                app.selected_guides.push(i);
                            }
                        } else {
                            app.selected_guides = vec![i];
                        }
                        app.guide_rotate = None;
                        let mut form = app.guide_form;
                        form.load(app, &g.line);
                        app.guide_form = form;
                    }
                }
            });
    });
}

/// Window > Panels > Guidelines: show and snap toggles, the guideline type, position fields,
/// Add and Modify, the list, delete and lock, colour and style.
pub fn guidelines_docker(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.toggle_value(&mut app.show_guides, tr("guides.show"))
            .on_hover_text(tr("guides.show_tip"));
        ui.toggle_value(&mut app.snap.guides, tr("guides.snap"))
            .on_hover_text(tr("guides.snap_tip"));
    });
    ui.add_space(4.0);
    let mut form = app.guide_form;
    ui.horizontal(|ui| {
        ui.label(tr("guides.type"));
        egui::ComboBox::from_id_salt("guide_type")
            .selected_text(tr(form.kind.key()))
            .width(140.0)
            .show_ui(ui, |ui| {
                for k in GuideKind::ALL {
                    ui.selectable_value(&mut form.kind, k, tr(k.key()));
                }
            });
    });
    let units = app.units;
    egui::Grid::new("guide_form")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            if form.kind != GuideKind::Horizontal {
                row_label(ui, "guides.x");
                if let Some(v) = distance_field(ui, units, form.x, 0.1) {
                    form.x = v;
                }
                ui.end_row();
            }
            if form.kind != GuideKind::Vertical {
                row_label(ui, "guides.y");
                if let Some(v) = distance_field(ui, units, form.y, 0.1) {
                    form.y = v;
                }
                ui.end_row();
            }
            if form.kind == GuideKind::Angled {
                row_label(ui, "guides.angle");
                ui.add_sized(
                    [110.0, 20.0],
                    egui::DragValue::new(&mut form.angle)
                        .speed(1.0)
                        .max_decimals(2)
                        .suffix("°"),
                );
                ui.end_row();
            }
        });
    app.guide_form = form;
    ui.horizontal(|ui| {
        if ui.button(tr("guides.add")).clicked() {
            let line = form.line(app);
            app.add_guide(Guide::from(line));
            app.selected_guides = vec![app.page_guides().len().saturating_sub(1)];
        }
        let single = match app.selected_guides.as_slice() {
            [i] => Some(*i),
            _ => None,
        };
        if ui
            .add_enabled(single.is_some(), egui::Button::new(tr("guides.modify")))
            .clicked()
        {
            if let Some(i) = single {
                if let Some(g) = app.page_guides().get(i).copied() {
                    if g.locked {
                        app.status = tr("status.guideline_locked");
                    } else {
                        let line = form.line(app);
                        app.replace_guide(i, g.with_line(line), "Move Guideline");
                    }
                }
            }
        }
    });
    ui.add_space(4.0);
    guide_list(app, ui, Some(form.kind), 160.0);
    ui.add_space(4.0);
    let any = !app.selected_guides.is_empty();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(any, egui::Button::new(tr("guides.delete")))
            .clicked()
            && !app.delete_selected_guides()
        {
            app.status = tr("status.guideline_locked");
        }
        let guides = app.page_guides();
        let all_locked = any
            && app
                .selected_guides
                .iter()
                .all(|i| guides.get(*i).is_some_and(|g| g.locked));
        let label = if all_locked {
            tr("guides.unlock")
        } else {
            tr("guides.lock")
        };
        if ui.add_enabled(any, egui::Button::new(label)).clicked() {
            let sel = app.selected_guides.clone();
            app.set_guides_locked(&sel, !all_locked);
        }
    });
    // Colour and style of the selected guidelines (or the default colour
    // when none is selected).
    let guides = app.page_guides();
    let first = app
        .selected_guides
        .first()
        .and_then(|i| guides.get(*i))
        .copied();
    egui::Grid::new("guide_look")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            row_label(ui, "guides.color");
            let current = first
                .map(|g| app.guide_color(&g))
                .unwrap_or(app.doc().metadata.guides.color);
            if let Some(c) = color_button(ui, current) {
                if any {
                    let mut gs = guides.clone();
                    for i in &app.selected_guides {
                        if let Some(g) = gs.get_mut(*i) {
                            g.color = Some(c);
                        }
                    }
                    app.set_guides(gs, "Guideline Color");
                } else {
                    let mut meta = app.doc().metadata.clone();
                    meta.guides.color = c;
                    set_metadata(app, meta);
                }
            }
            ui.end_row();
            row_label(ui, "guides.style");
            let style = first.map(|g| g.style).unwrap_or_default();
            ui.add_enabled_ui(any, |ui| {
                egui::ComboBox::from_id_salt("guide_style")
                    .selected_text(tr(style_key(style)))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for s in GuideStyle::ALL {
                            if ui.selectable_label(s == style, tr(style_key(s))).clicked() {
                                let mut gs = guides.clone();
                                for i in &app.selected_guides {
                                    if let Some(g) = gs.get_mut(*i) {
                                        g.style = s;
                                    }
                                }
                                app.set_guides(gs, "Guideline Style");
                            }
                        }
                    });
            });
            ui.end_row();
        });
}

fn style_key(s: GuideStyle) -> &'static str {
    match s {
        GuideStyle::Solid => "guides.style_solid",
        GuideStyle::Dashed => "guides.style_dashed",
        GuideStyle::Dotted => "guides.style_dotted",
        GuideStyle::DashDot => "guides.style_dash_dot",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Size;

    #[test]
    fn the_form_counts_from_the_ruler_origin() {
        let mut app = App::headless();
        app.set_ruler_origin(Point::new(10.0, 20.0));
        let form = GuideForm {
            kind: GuideKind::Horizontal,
            x: 0.0,
            y: 5.0,
            angle: 0.0,
        };
        assert_eq!(form.line(&app), GuideLine::Horizontal { y: 25.0 });
        let mut back = GuideForm::default();
        back.load(&app, &GuideLine::Vertical { x: 13.0 });
        assert_eq!(back.kind, GuideKind::Vertical);
        assert!((back.x - 3.0).abs() < 1e-9);
    }

    #[test]
    fn named_presets_add_their_lines() {
        let size = Size::new(210.0, 297.0);
        let margins = preset_lines(NamedPreset::OneInchMargins, size, 0.0);
        assert_eq!(margins.len(), 4);
        assert!(margins.contains(&GuideLine::Vertical { x: 210.0 - 25.4 }));
        let bleed = preset_lines(NamedPreset::BleedArea, size, 3.0);
        assert!(bleed.contains(&GuideLine::Vertical { x: -3.0 }));
        assert!(bleed.contains(&GuideLine::Horizontal { y: 300.0 }));
        let news = preset_lines(NamedPreset::ThreeColumnNewsletter, size, 0.0);
        // Four margins and four column edges between them.
        assert_eq!(news.len(), 8);
        let upper = preset_lines(NamedPreset::UpperLeftGrid, size, 0.0);
        assert!(upper.iter().all(|l| match l {
            GuideLine::Vertical { x } => *x <= 105.0 + 1e-9,
            GuideLine::Horizontal { y } => *y >= 148.5 - 1e-9,
            _ => false,
        }));
    }

    #[test]
    fn user_defined_columns_sit_between_the_margins() {
        let mut app = App::headless();
        let size = app.page_size();
        let form = PresetForm {
            user_defined: true,
            margins: true,
            margin_left: 10.0,
            margin_right: 10.0,
            columns: true,
            column_count: 2,
            column_gutter: 10.0,
            ..PresetForm::default()
        };
        apply_presets(&mut app, &form);
        let xs: Vec<f64> = app
            .page_guides()
            .iter()
            .filter_map(|g| match g.line {
                GuideLine::Vertical { x } => Some(x),
                _ => None,
            })
            .collect();
        let mid = size.width / 2.0;
        assert!(xs.iter().any(|x| (x - (mid - 5.0)).abs() < 1e-9), "{xs:?}");
        assert!(xs.iter().any(|x| (x - (mid + 5.0)).abs() < 1e-9), "{xs:?}");
        // Applying twice adds nothing new.
        let n = app.page_guides().len();
        apply_presets(&mut app, &form);
        assert_eq!(app.page_guides().len(), n);
    }
}
