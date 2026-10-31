//! The Properties docker's Transparency section (#1439, #1203): the merge
//! mode at the top, a row of type buttons (no transparency, uniform,
//! gradient, vector pattern, bitmap pattern, two-colour pattern, texture)
//! with the chosen type's title, a transparency picker of ready-made
//! masks with Edit Transparency, then the type's own settings (the
//! amount; the gradient's type buttons, stops and angle; the pattern's
//! or texture's foreground and background), what it applies to as three
//! buttons, and Clear Transparency. The Transparency tool's bar keeps its
//! one-row form (`toolbar::transparency_part`).

use crate::app::App;
use crate::effects_ui::TransparencySettings;
use crate::i18n::{tr, trf};
use crate::mixed::{mask_colors, TransparencyField as F};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{Color32, Rect, Sense, Ui, Vec2};
use tracedraw_core::document::{Shape, ShapeKind};
use tracedraw_core::geometry::Rect as GRect;
use tracedraw_core::style::{FountainKind, Pattern, PatternTile, Stop, Texture, TextureKind};
use tracedraw_core::{Color, Fill, ShapeId};

/// The transparency types of the docker's type buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransparencyType {
    None,
    Uniform,
    Fountain,
    VectorPattern,
    BitmapPattern,
    TwoColorPattern,
    Texture,
}

impl TransparencyType {
    /// Every type, in the order of the buttons.
    pub const ALL: [TransparencyType; 7] = [
        TransparencyType::None,
        TransparencyType::Uniform,
        TransparencyType::Fountain,
        TransparencyType::VectorPattern,
        TransparencyType::BitmapPattern,
        TransparencyType::TwoColorPattern,
        TransparencyType::Texture,
    ];

    /// The type of a transparency of `kind` (as
    /// [`crate::mixed::ShapeTransparency::kind`]) with `mask`.
    pub fn of(kind: usize, mask: &Fill) -> TransparencyType {
        match kind {
            0 => TransparencyType::None,
            1 => TransparencyType::Uniform,
            2 => TransparencyType::Fountain,
            4 => TransparencyType::Texture,
            _ => match mask {
                Fill::Pattern(Pattern::Vector { .. }) => TransparencyType::VectorPattern,
                Fill::Pattern(Pattern::Bitmap { .. }) => TransparencyType::BitmapPattern,
                _ => TransparencyType::TwoColorPattern,
            },
        }
    }

    pub fn name_key(self) -> &'static str {
        match self {
            TransparencyType::None => "transparency.none",
            TransparencyType::Uniform => "transparency.uniform",
            TransparencyType::Fountain => "transparency.fountain",
            TransparencyType::VectorPattern => "transparency.vector_pattern",
            TransparencyType::BitmapPattern => "transparency.bitmap_pattern",
            TransparencyType::TwoColorPattern => "transparency.two_color_pattern",
            TransparencyType::Texture => "transparency.texture",
        }
    }

    /// The section title shown under the type buttons.
    pub fn title_key(self) -> &'static str {
        match self {
            TransparencyType::None => "transparency.none",
            TransparencyType::Uniform => "transparency.title_uniform",
            TransparencyType::Fountain => "transparency.title_fountain",
            TransparencyType::VectorPattern => "transparency.title_vector_pattern",
            TransparencyType::BitmapPattern => "transparency.title_bitmap_pattern",
            TransparencyType::TwoColorPattern => "transparency.title_two_color_pattern",
            TransparencyType::Texture => "transparency.title_texture",
        }
    }

    /// The mask the type starts with; none for no transparency and
    /// uniform transparency.
    pub fn default_mask(self) -> Option<Fill> {
        match self {
            TransparencyType::None | TransparencyType::Uniform => None,
            TransparencyType::Fountain => Some(TransparencySettings::default_mask(1, 0.0)),
            TransparencyType::TwoColorPattern => Some(TransparencySettings::default_mask(2, 0.0)),
            TransparencyType::Texture => Some(TransparencySettings::default_mask(3, 0.0)),
            TransparencyType::VectorPattern | TransparencyType::BitmapPattern => {
                self.presets().into_iter().next()
            }
        }
    }

    /// The ready-made masks of the transparency picker (uniform
    /// transparency has amounts instead, [`UNIFORM_PRESETS`]).
    pub fn presets(self) -> Vec<Fill> {
        match self {
            TransparencyType::None | TransparencyType::Uniform => Vec::new(),
            TransparencyType::Fountain => fountain_presets(),
            TransparencyType::TwoColorPattern => PatternTile::ALL
                .iter()
                .map(|&tile| {
                    Fill::Pattern(Pattern::TwoColor {
                        tile,
                        front: Color::WHITE,
                        back: Color::BLACK,
                        size_mm: 10.0,
                    })
                })
                .collect(),
            TransparencyType::Texture => TextureKind::ALL
                .iter()
                .flat_map(|&kind| {
                    [20.0, 6.0].map(|scale| {
                        Fill::Texture(Texture {
                            kind,
                            color_a: Color::WHITE,
                            color_b: Color::BLACK,
                            scale,
                            seed: 1,
                        })
                    })
                })
                .collect(),
            TransparencyType::VectorPattern => VECTOR_PRESETS.with(|p| p.clone()),
            TransparencyType::BitmapPattern => BITMAP_PRESETS.with(|p| p.clone()),
        }
    }

    /// The `ShapeTransparency::kind - 1` the type keeps in
    /// [`TransparencySettings::kind`].
    fn settings_kind(self) -> u8 {
        match self {
            TransparencyType::None | TransparencyType::Uniform => 0,
            TransparencyType::Fountain => 1,
            TransparencyType::Texture => 3,
            _ => 2,
        }
    }
}

/// The uniform amounts (%) the picker offers: 0 to 85 % in steps of 5,
/// six to a row.
pub const UNIFORM_PRESETS: [f64; 18] = [
    0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 55.0, 60.0, 65.0, 70.0, 75.0,
    80.0, 85.0,
];

/// A uniform transparency of `amount` % on `r`: black at that opacity
/// over a checker, so the swatch shows how much shows through.
fn uniform_swatch(p: &egui::Painter, r: Rect, amount: f64) {
    let cell = 5.0;
    let (nx, ny) = (
        (r.width() / cell).ceil() as i32,
        (r.height() / cell).ceil() as i32,
    );
    for y in 0..ny {
        for x in 0..nx {
            let c = if (x + y) % 2 == 0 {
                Color32::from_gray(255)
            } else {
                Color32::from_gray(200)
            };
            let cr = Rect::from_min_size(
                r.min + Vec2::new(x as f32 * cell, y as f32 * cell),
                Vec2::splat(cell),
            )
            .intersect(r);
            p.rect_filled(cr, 0.0, c);
        }
    }
    let a = ((1.0 - amount / 100.0).clamp(0.0, 1.0) * 255.0).round() as u8;
    p.rect_filled(r, 0.0, Color32::from_black_alpha(a));
}

thread_local! {
    static VECTOR_PRESETS: Vec<Fill> = vector_presets();
    static BITMAP_PRESETS: Vec<Fill> = bitmap_presets();
}

fn gray(v: f32) -> Color {
    Color::Gray { v }
}

/// The ready-made gradient masks: white is opaque, black clear.
fn fountain_presets() -> Vec<Fill> {
    let two = |a: f32, b: f32| vec![Stop::new(0.0, gray(a)), Stop::new(1.0, gray(b))];
    let three = vec![
        Stop::new(0.0, gray(0.0)),
        Stop::new(0.5, gray(1.0)),
        Stop::new(1.0, gray(0.0)),
    ];
    let make = |kind: FountainKind, stops: Vec<Stop>, angle: f64| {
        let mut f = Fill::linear(Color::WHITE, Color::BLACK, angle);
        if let Fill::Fountain(g) = &mut f {
            g.kind = kind;
            g.stops = stops;
        }
        f
    };
    vec![
        make(FountainKind::Linear, two(1.0, 0.0), 0.0),
        make(FountainKind::Linear, two(0.0, 1.0), 0.0),
        make(FountainKind::Linear, two(1.0, 0.0), 90.0),
        make(FountainKind::Linear, two(1.0, 0.0), 45.0),
        make(FountainKind::Linear, three, 0.0),
        make(FountainKind::Radial, two(1.0, 0.0), 0.0),
        make(FountainKind::Radial, two(0.0, 1.0), 0.0),
        make(FountainKind::Conical, two(1.0, 0.0), 0.0),
        make(FountainKind::Square, two(1.0, 0.0), 0.0),
        make(FountainKind::Square, two(0.0, 1.0), 0.0),
        make(FountainKind::Shaped, two(0.0, 1.0), 0.0),
        make(FountainKind::Linear, two(1.0, 0.5), 0.0),
    ]
}

/// The ready-made vector pattern masks: black shapes on a white square,
/// one 10 mm tile each.
fn vector_presets() -> Vec<Fill> {
    let shape = |id: u64, kind: ShapeKind, v: f32| {
        let mut s = Shape::new(ShapeId(id), kind);
        s.fill = Fill::Solid(gray(v));
        s.stroke = None;
        s
    };
    let rect = |id, x0, y0, x1, y1, v| {
        shape(
            id,
            ShapeKind::Rect {
                rect: GRect::new(x0, y0, x1, y1),
                radius: 0.0,
                corners: None,
            },
            v,
        )
    };
    let ellipse = |id, x0, y0, x1, y1, v| {
        shape(
            id,
            ShapeKind::Ellipse {
                rect: GRect::new(x0, y0, x1, y1),
                arc: None,
            },
            v,
        )
    };
    let tiles = [
        // Dots.
        vec![
            rect(1, 0.0, 0.0, 10.0, 10.0, 1.0),
            ellipse(2, 3.0, 3.0, 7.0, 7.0, 0.0),
        ],
        // Rings.
        vec![
            rect(1, 0.0, 0.0, 10.0, 10.0, 1.0),
            ellipse(2, 1.0, 1.0, 9.0, 9.0, 0.0),
            ellipse(3, 3.0, 3.0, 7.0, 7.0, 1.0),
        ],
        // Squares.
        vec![
            rect(1, 0.0, 0.0, 10.0, 10.0, 1.0),
            rect(2, 0.0, 0.0, 5.0, 5.0, 0.0),
            rect(3, 5.0, 5.0, 10.0, 10.0, 0.0),
        ],
        // Bars.
        vec![
            rect(1, 0.0, 0.0, 10.0, 10.0, 1.0),
            rect(2, 0.0, 0.0, 10.0, 4.0, 0.0),
        ],
        // Soft dots: a grey ring round a black dot.
        vec![
            rect(1, 0.0, 0.0, 10.0, 10.0, 1.0),
            ellipse(2, 1.5, 1.5, 8.5, 8.5, 0.5),
            ellipse(3, 3.5, 3.5, 6.5, 6.5, 0.0),
        ],
    ];
    tiles
        .iter()
        .map(|shapes| Fill::Pattern(Pattern::vector_from_shapes(shapes)))
        .collect()
}

/// The ready-made bitmap pattern masks: small grey tiles made here.
fn bitmap_presets() -> Vec<Fill> {
    const N: u32 = 32;
    let tiles: [fn(f32, f32) -> f32; 4] = [
        // Soft dot.
        |u, v| {
            let d = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt();
            (d / 0.5).clamp(0.0, 1.0)
        },
        // Ramp stripes.
        |u, _| u,
        // Diagonal waves.
        |u, v| 0.5 + 0.5 * ((u + v) * std::f32::consts::TAU).sin(),
        // Soft checker.
        |u, v| {
            let s = (u * std::f32::consts::TAU).sin() * (v * std::f32::consts::TAU).sin();
            0.5 + 0.5 * s
        },
    ];
    tiles
        .iter()
        .filter_map(|f| {
            let img = image::RgbaImage::from_fn(N, N, |x, y| {
                let u = (x as f32 + 0.5) / N as f32;
                let v = (y as f32 + 0.5) / N as f32;
                let g = (f(u, v).clamp(0.0, 1.0) * 255.0).round() as u8;
                image::Rgba([g, g, g, 255])
            });
            let png = crate::bitmap_fx::encode(&img)?;
            Some(Fill::Pattern(Pattern::Bitmap {
                png: png.into(),
                width_px: N,
                height_px: N,
                size_mm: 10.0,
            }))
        })
        .collect()
}

/// The transparency (%) a mask grey stands for: white is opaque.
pub fn gray_transparency(c: Color) -> f64 {
    let [r, g, b] = c.to_rgb_f32();
    let v = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
    ((1.0 - v) * 100.0).clamp(0.0, 100.0)
}

/// The mask grey for a transparency of `pct` %.
pub fn transparency_gray(pct: f64) -> Color {
    gray((1.0 - pct / 100.0).clamp(0.0, 1.0) as f32)
}

/// A key that changes when `mask` does, without formatting a bitmap's
/// bytes every frame.
fn mask_key(mask: &Fill) -> egui::Id {
    match mask {
        Fill::Pattern(Pattern::Bitmap {
            png,
            width_px,
            height_px,
            size_mm,
        }) => egui::Id::new((
            "bitmap",
            png.as_slice().as_ptr() as usize,
            png.as_slice().len(),
            *width_px,
            *height_px,
            size_mm.to_bits(),
        )),
        _ => egui::Id::new(format!("{mask:?}")),
    }
}

/// `mask` rendered on a 20 mm square into a texture of about `px` pixels,
/// made once per `key` and kept.
fn mask_texture(ui: &Ui, key: egui::Id, mask: &Fill, px: f32) -> Option<egui::TextureHandle> {
    if let Some(t) = ui.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(t);
    }
    let side = 20.0;
    let dpi = px as f64 / side * 25.4;
    let pm = tracedraw_render::render_fill_image(mask, GRect::new(0.0, 0.0, side, side), dpi)?;
    let img = egui::ColorImage::from_rgba_premultiplied(
        [pm.width() as usize, pm.height() as usize],
        pm.data(),
    );
    let t = ui
        .ctx()
        .load_texture("transparency_mask", img, egui::TextureOptions::LINEAR);
    ui.data_mut(|d| d.insert_temp(key, t.clone()));
    Some(t)
}

/// A small greyscale icon made from `f` (u, v in 0..1, grey 0..1), made
/// once and kept.
fn icon_texture(ui: &Ui, name: &str, f: fn(f32, f32) -> f32) -> egui::TextureHandle {
    let key = egui::Id::new(("transparency_icon", name));
    if let Some(t) = ui.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return t;
    }
    const N: usize = 32;
    let mut px = Vec::with_capacity(N * N);
    for y in 0..N {
        for x in 0..N {
            let u = (x as f32 + 0.5) / N as f32;
            let v = (y as f32 + 0.5) / N as f32;
            px.push(Color32::from_gray(
                (f(u, v).clamp(0.0, 1.0) * 255.0).round() as u8,
            ));
        }
    }
    let img = egui::ColorImage::new([N, N], px);
    let t = ui.ctx().load_texture(
        format!("transparency_icon_{name}"),
        img,
        egui::TextureOptions::LINEAR,
    );
    ui.data_mut(|d| d.insert_temp(key, t.clone()));
    t
}

fn type_icon(t: TransparencyType) -> (&'static str, fn(f32, f32) -> f32) {
    match t {
        TransparencyType::None => ("none", |u, v| {
            if (u - (1.0 - v)).abs() < 0.09 {
                0.0
            } else {
                1.0
            }
        }),
        TransparencyType::Uniform => ("uniform", |_, _| 0.5),
        TransparencyType::Fountain => ("fountain", |u, _| 1.0 - u),
        TransparencyType::VectorPattern => ("vector", |u, v| {
            let (cu, cv) = ((u * 3.0).fract() - 0.5, (v * 3.0).fract() - 0.5);
            if cu * cu + cv * cv < 0.09 {
                0.0
            } else {
                1.0
            }
        }),
        TransparencyType::BitmapPattern => ("bitmap", |u, v| {
            let (cu, cv) = ((u * 3.0).fract() - 0.5, (v * 3.0).fract() - 0.5);
            ((cu * cu + cv * cv).sqrt() * 2.0).clamp(0.0, 1.0)
        }),
        TransparencyType::TwoColorPattern => ("two_color", |u, v| {
            if ((u * 4.0) as i32 + (v * 4.0) as i32) % 2 == 0 {
                1.0
            } else {
                0.0
            }
        }),
        TransparencyType::Texture => ("texture", |u, v| {
            let a = (u * 9.0 + (v * 7.0).sin() * 2.0).sin();
            let b = (v * 11.0 + (u * 5.0).cos() * 2.0).sin();
            0.5 + 0.25 * (a + b)
        }),
    }
}

fn fountain_icon(k: FountainKind) -> (&'static str, fn(f32, f32) -> f32) {
    match k {
        FountainKind::Linear => ("f_linear", |u, _| 1.0 - u),
        FountainKind::Radial => ("f_radial", |u, v| {
            (((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 2.0).clamp(0.0, 1.0)
        }),
        FountainKind::Conical => ("f_conical", |u, v| {
            ((v - 0.5).atan2(u - 0.5) / std::f32::consts::TAU + 0.5).fract()
        }),
        FountainKind::Square => ("f_square", |u, v| {
            ((u - 0.5).abs().max((v - 0.5).abs()) * 2.0).clamp(0.0, 1.0)
        }),
        FountainKind::Shaped => ("f_shaped", |u, v| {
            // A blob, clear at its edge and opaque deep inside.
            let d = ((u - 0.5).powi(2) * 1.6 + (v - 0.5).powi(2)).sqrt();
            if d > 0.45 {
                1.0
            } else {
                1.0 - (0.45 - d) / 0.45
            }
        }),
    }
}

/// A button showing `tex`, framed, picked when `selected`.
fn icon_button(ui: &mut Ui, tex: &egui::TextureHandle, selected: bool) -> egui::Response {
    let img = egui::Image::from_texture(egui::load::SizedTexture::new(tex.id(), Vec2::splat(16.0)));
    let resp = ui.add(egui::Button::image(img).selected(selected));
    let r = Rect::from_center_size(resp.rect.center(), Vec2::splat(16.0));
    ui.painter().rect_stroke(
        r,
        0.0,
        egui::Stroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Outside,
    );
    resp
}

/// A dim label for a row of the section.
fn dim(ui: &mut Ui, text: String) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .color(Tokens::text_dim())
                .size(11.0),
        )
        .extend(),
    );
}

/// The id of the gradient stop picked in the docker.
fn stop_id() -> egui::Id {
    egui::Id::new("transparency_docker_stop")
}

/// The Transparency section of the Properties docker.
pub fn section(app: &mut App, ui: &mut Ui) {
    let (mut set, current, mixed, has_selection) = crate::ui::toolbar::transparency_state(app);
    let kind_mixed = current.is_none();
    let ty = current.map(|k| TransparencyType::of(k, &set.mask));
    let mut field = None;

    // Merge mode, full width at the top.
    dim(ui, tr("transparency.merge_mode"));
    let merge_shown = !mixed.contains(&F::Merge);
    egui::ComboBox::from_id_salt("transparency_docker_merge")
        .selected_text(if merge_shown {
            set.merge.name().to_string()
        } else {
            String::new()
        })
        .width(ui.available_width() - 4.0)
        .show_ui(ui, |ui| {
            for m in tracedraw_core::live::MergeMode::ALL {
                if ui
                    .selectable_label(merge_shown && set.merge == m, m.name())
                    .clicked()
                {
                    set.merge = m;
                    field = Some(F::Merge);
                }
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("transparency.merge_mode"));

    // The type buttons.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for t in TransparencyType::ALL {
            let (name, f) = type_icon(t);
            let tex = icon_texture(ui, name, f);
            let resp = icon_button(ui, &tex, ty == Some(t)).rich_tip(crate::ui::tips::pair(
                t.name_key(),
                &format!("{}_desc", t.name_key()),
            ));
            if resp.clicked() && ty != Some(t) {
                match t {
                    TransparencyType::None => app.clear_transparency(),
                    TransparencyType::Uniform => {
                        set.set_kind(0);
                        field = Some(F::Kind);
                    }
                    _ => {
                        if let Some(mask) = t.default_mask() {
                            set.kind = t.settings_kind();
                            set.mask = mask;
                            field = Some(F::Mask);
                        }
                    }
                }
            }
        }
    });
    if let Some(t) = ty {
        ui.label(egui::RichText::new(tr(t.title_key())).strong());
    }

    // The picker and Edit Transparency.
    if let Some(t) = ty.filter(|t| *t != TransparencyType::None) {
        picker(app, ui, t, &mut set, &mut field);
    }

    match ty {
        Some(TransparencyType::Uniform) => {
            let amount_mixed = mixed.contains(&F::Amount);
            let mut amount = set.amount;
            if ui
                .add_enabled(
                    has_selection,
                    crate::ui::Slider::new(&mut amount, 0.0..=100.0)
                        .suffix(" %")
                        .mixed(amount_mixed)
                        .text(tr("docker.transparency_lc")),
                )
                .rich_tip(crate::ui::tips::keyed("toolbar.transparency_amount"))
                .changed()
            {
                set.amount = amount;
                field = Some(F::Amount);
            }
        }
        Some(TransparencyType::Fountain) => fountain(ui, &mut set, &mixed, &mut field),
        Some(TransparencyType::TwoColorPattern | TransparencyType::Texture) => {
            two_greys(ui, &mut set, &mut field)
        }
        _ => {}
    }

    // What it applies to, as three buttons.
    if !kind_mixed && ty != Some(TransparencyType::None) {
        dim(ui, tr("transparency.target"));
        let target_shown = !mixed.contains(&F::Target);
        ui.horizontal(|ui| {
            for (i, key) in [
                "transparency.target_fill",
                "transparency.target_outline",
                "transparency.target_all",
            ]
            .iter()
            .enumerate()
            {
                if ui
                    .add(egui::Button::selectable(
                        target_shown && set.target as usize == i,
                        tr(key),
                    ))
                    .rich_tip(crate::ui::tips::keyed("toolbar.transparency_target"))
                    .clicked()
                {
                    set.target = i as u8;
                    field = Some(F::Target);
                }
            }
        });
    }

    if let Some(f) = field {
        app.transparency_default = set.clone();
        if has_selection {
            app.patch_transparency(&set, &[f]);
        }
    }

    // Clear Transparency (#1338).
    if ui
        .add_enabled(
            app.can_clear_transparency(),
            egui::Button::new(tr("toolbar.clear_transparency")),
        )
        .rich_tip(crate::ui::tips::keyed("toolbar.clear_transparency"))
        .clicked()
    {
        app.clear_transparency();
    }
}

/// The swatch of the current transparency with its drop-down grid of
/// ready-made ones, and Edit Transparency.
fn picker(
    app: &mut App,
    ui: &mut Ui,
    t: TransparencyType,
    set: &mut TransparencySettings,
    field: &mut Option<F>,
) {
    ui.horizontal(|ui| {
        let (r, resp) = ui.allocate_exact_size(Vec2::new(44.0, 28.0), Sense::click());
        if t == TransparencyType::Uniform {
            uniform_swatch(ui.painter(), r, set.amount);
        } else {
            let key = egui::Id::new(("transparency_current", mask_key(&set.mask)));
            if let Some(tex) = mask_texture(ui, key, &set.mask, 48.0) {
                ui.painter().image(
                    tex.id(),
                    r,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        let line = if resp.hovered() {
            Tokens::ACCENT
        } else {
            Tokens::border()
        };
        ui.painter().rect_stroke(
            r,
            2.0,
            egui::Stroke::new(1.0, line),
            egui::StrokeKind::Inside,
        );
        // The arrow of the drop-down.
        let a = egui::pos2(r.right() - 7.0, r.bottom() - 6.0);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                a + Vec2::new(-4.0, -3.0),
                a + Vec2::new(4.0, -3.0),
                a + Vec2::new(0.0, 2.0),
            ],
            Tokens::ACCENT,
            egui::Stroke::NONE,
        ));
        let resp = resp.rich_tip(crate::ui::tips::keyed("transparency.picker"));
        egui::Popup::menu(&resp)
            .id(egui::Id::new("transparency_picker_menu"))
            .show(|ui| {
                ui.set_max_width(6.0 * 38.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(3.0);
                    if t == TransparencyType::Uniform {
                        for a in UNIFORM_PRESETS {
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::new(34.0, 24.0), Sense::click());
                            uniform_swatch(ui.painter(), r, a);
                            frame(ui, r, resp.hovered());
                            if resp.rich_tip(format!("{a} %")).clicked() {
                                set.amount = a;
                                *field = Some(F::Amount);
                                ui.close();
                            }
                        }
                    } else {
                        for (i, mask) in t.presets().into_iter().enumerate() {
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::new(34.0, 24.0), Sense::click());
                            let key = egui::Id::new(("transparency_preset", t.name_key(), i));
                            if let Some(tex) = mask_texture(ui, key, &mask, 40.0) {
                                ui.painter().image(
                                    tex.id(),
                                    r,
                                    Rect::from_min_max(
                                        egui::pos2(0.0, 0.15),
                                        egui::pos2(1.0, 0.85),
                                    ),
                                    Color32::WHITE,
                                );
                            }
                            frame(ui, r, resp.hovered());
                            let tip = trf("transparency.preset_n", &[("n", &(i + 1).to_string())]);
                            if resp.rich_tip(tip).clicked() {
                                set.kind = t.settings_kind();
                                set.mask = mask;
                                *field = Some(F::Mask);
                                ui.close();
                            }
                        }
                    }
                });
            });
        if ui
            .button(tr("transparency.edit"))
            .rich_tip(crate::ui::tips::keyed("transparency.edit"))
            .clicked()
        {
            app.set_tool(crate::tools::Tool::Transparency);
        }
    });
}

fn frame(ui: &Ui, r: Rect, hovered: bool) {
    let line = if hovered {
        Tokens::ACCENT
    } else {
        Tokens::border()
    };
    ui.painter().rect_stroke(
        r,
        2.0,
        egui::Stroke::new(1.0, line),
        egui::StrokeKind::Inside,
    );
}

/// A gradient mask: its type buttons, the bar with its stops, the picked
/// stop's opacity and position with Add and Remove, and the angle.
fn fountain(ui: &mut Ui, set: &mut TransparencySettings, mixed: &[F], field: &mut Option<F>) {
    let Fill::Fountain(f) = &mut set.mask else {
        return;
    };
    let kind_shown = !mixed.contains(&F::MaskKind);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for k in FountainKind::ALL {
            let (name, g) = fountain_icon(k);
            let tex = icon_texture(ui, name, g);
            if icon_button(ui, &tex, kind_shown && f.kind == k)
                .rich_tip(crate::ui::tips::pair(
                    crate::ui::dockers::fountain_kind_key(k),
                    "toolbar.transparency_fountain_kind_desc",
                ))
                .clicked()
                && !(kind_shown && f.kind == k)
            {
                f.kind = k;
                *field = Some(F::MaskKind);
            }
        }
    });

    // The bar: the stops as drawn, a marker under each; a click picks
    // the nearest stop, a drag moves it between its neighbours.
    let n = f.stops.len();
    let mut sel: usize = ui.data(|d| d.get_temp(stop_id())).unwrap_or(0);
    sel = sel.min(n.saturating_sub(1));
    let width = ui.available_width().min(260.0);
    let (r, resp) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::click_and_drag());
    let bar = Rect::from_min_size(r.min, Vec2::new(r.width(), 14.0));
    crate::ui::dockers::gradient_swatch(ui.painter(), bar, &f.sorted_stops());
    ui.painter().rect_stroke(
        bar,
        2.0,
        egui::Stroke::new(1.0, Tokens::border()),
        egui::StrokeKind::Inside,
    );
    let x_of = |pos: f64| bar.left() + pos.clamp(0.0, 1.0) as f32 * bar.width();
    if let Some(p) = resp.interact_pointer_pos() {
        if resp.drag_started() || resp.clicked() {
            if let Some((i, _)) = f
                .stops
                .iter()
                .enumerate()
                .map(|(i, s)| (i, (x_of(s.pos) - p.x).abs()))
                .min_by(|a, b| a.1.total_cmp(&b.1))
            {
                sel = i;
            }
        } else if resp.dragged() && n > 0 {
            let lo = if sel > 0 { f.stops[sel - 1].pos } else { 0.0 };
            let hi = f.stops.get(sel + 1).map(|s| s.pos).unwrap_or(1.0);
            let pos = (((p.x - bar.left()) / bar.width().max(1.0)) as f64).clamp(lo, hi);
            if (pos - f.stops[sel].pos).abs() > 1e-9 {
                f.stops[sel].pos = pos;
                *field = Some(F::MaskStops);
            }
        }
    }
    for (i, s) in f.stops.iter().enumerate() {
        let x = x_of(s.pos);
        let top = bar.bottom() + 1.0;
        let fill = if i == sel {
            Tokens::ACCENT
        } else {
            Tokens::text_dim()
        };
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(x, top),
                egui::pos2(x + 5.0, top + 9.0),
                egui::pos2(x - 5.0, top + 9.0),
            ],
            fill,
            egui::Stroke::NONE,
        ));
    }
    resp.rich_tip(crate::ui::tips::keyed("toolbar.node_position"));

    if let Some(stop) = f.stops.get_mut(sel) {
        // Shown as the stop's opacity: 100 % where the mask is white.
        let mut o = (100.0 - gray_transparency(stop.color)).round();
        if ui
            .add(
                crate::ui::Slider::new(&mut o, 0.0..=100.0)
                    .suffix(" %")
                    .text(tr("transparency.stop_opacity")),
            )
            .rich_tip(crate::ui::tips::keyed("transparency.stop_opacity"))
            .changed()
        {
            stop.color = transparency_gray(100.0 - o);
            *field = Some(F::MaskStops);
        }
    }
    ui.horizontal(|ui| {
        dim(ui, tr("toolbar.node_position"));
        let lo = if sel > 0 { f.stops[sel - 1].pos } else { 0.0 };
        let hi = f.stops.get(sel + 1).map(|s| s.pos).unwrap_or(1.0);
        if let Some(stop) = f.stops.get_mut(sel) {
            let mut pct = (stop.pos * 100.0).round();
            if ui
                .add(
                    crate::ui::field::NumField::new(&mut pct)
                        .range(lo * 100.0..=hi * 100.0)
                        .suffix(" %")
                        .speed(1.0),
                )
                .rich_tip(crate::ui::tips::keyed("toolbar.node_position"))
                .changed()
            {
                stop.pos = (pct / 100.0).clamp(lo, hi);
                *field = Some(F::MaskStops);
            }
        }
        if ui
            .small_button("+")
            .rich_tip(tr("docker.add_stop"))
            .clicked()
        {
            add_stop(&mut f.stops, sel);
            sel += 1;
            *field = Some(F::MaskStops);
        }
        if n > 2
            && ui
                .small_button("x")
                .rich_tip(tr("docker.remove_stop"))
                .clicked()
        {
            f.stops.remove(sel);
            sel = sel.saturating_sub(1);
            *field = Some(F::MaskStops);
        }
    });
    ui.data_mut(|d| d.insert_temp(stop_id(), sel));

    if !matches!(f.kind, FountainKind::Radial | FountainKind::Shaped) {
        let mut angle = f.angle;
        if ui
            .add(
                crate::ui::Slider::new(&mut angle, -180.0..=180.0)
                    .text(tr("docker.angle_lc"))
                    .suffix("°")
                    .mixed(mixed.contains(&F::MaskAngle)),
            )
            .rich_tip(crate::ui::tips::pair(
                "toolbar.angle",
                "toolbar.fill_angle_desc",
            ))
            .changed()
        {
            f.angle = angle;
            *field = Some(F::MaskAngle);
        }
    }
}

/// Add a stop after `i` (before it when `i` is the last), half way to its
/// neighbour, with the gradient's own grey there.
pub fn add_stop(stops: &mut Vec<Stop>, i: usize) {
    if stops.is_empty() {
        stops.push(Stop::new(0.0, Color::WHITE));
        return;
    }
    let i = i.min(stops.len() - 1);
    let (a, b) = if i + 1 < stops.len() {
        (i, i + 1)
    } else if i > 0 {
        (i - 1, i)
    } else {
        stops.push(Stop::new(1.0, stops[0].color));
        return;
    };
    let pos = (stops[a].pos + stops[b].pos) / 2.0;
    let mut sorted = stops.clone();
    sorted.sort_by(|x, y| x.pos.total_cmp(&y.pos));
    let color = tracedraw_core::style::sample_stops(&sorted, pos);
    stops.insert(b, Stop::new(pos, color));
}

/// A two-colour pattern's or a texture's foreground and background
/// transparency, with a swap button.
fn two_greys(ui: &mut Ui, set: &mut TransparencySettings, field: &mut Option<F>) {
    let Some((fg, bg)) = mask_colors(&set.mask) else {
        return;
    };
    let mut new = (fg, bg);
    let mut fg_t = gray_transparency(fg).round();
    if ui
        .add(
            crate::ui::Slider::new(&mut fg_t, 0.0..=100.0)
                .suffix(" %")
                .text(tr("transparency.foreground")),
        )
        .rich_tip(crate::ui::tips::keyed("transparency.foreground"))
        .changed()
    {
        new.0 = transparency_gray(fg_t);
    }
    let mut bg_t = gray_transparency(bg).round();
    if ui
        .add(
            crate::ui::Slider::new(&mut bg_t, 0.0..=100.0)
                .suffix(" %")
                .text(tr("transparency.background")),
        )
        .rich_tip(crate::ui::tips::keyed("transparency.background"))
        .changed()
    {
        new.1 = transparency_gray(bg_t);
    }
    if ui
        .button(tr("transparency.swap"))
        .rich_tip(crate::ui::tips::keyed("transparency.swap"))
        .clicked()
    {
        new = (new.1, new.0);
    }
    if new != (fg, bg) {
        set_mask_colors(&mut set.mask, new.0, new.1);
        *field = Some(F::MaskColors);
    }
}

/// Set the foreground and background greys of a two-colour pattern or a
/// texture mask; other masks are left as they are.
pub fn set_mask_colors(mask: &mut Fill, fg: Color, bg: Color) {
    match mask {
        Fill::Pattern(Pattern::TwoColor { front, back, .. }) => {
            *front = fg;
            *back = bg;
        }
        Fill::Texture(t) => {
            t.color_a = fg;
            t.color_b = bg;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixed::mask_kind;

    #[test]
    fn every_type_has_a_mask_of_its_own_kind() {
        for t in TransparencyType::ALL {
            match t.default_mask() {
                Some(m) => {
                    let kind = mask_kind(&m);
                    assert_eq!(TransparencyType::of(kind, &m), t, "{t:?}");
                    assert_eq!(kind - 1, t.settings_kind() as usize, "{t:?}");
                }
                None => assert!(matches!(
                    t,
                    TransparencyType::None | TransparencyType::Uniform
                )),
            }
        }
        assert_eq!(TransparencyType::of(0, &Fill::None), TransparencyType::None);
        assert_eq!(
            TransparencyType::of(1, &Fill::None),
            TransparencyType::Uniform
        );
    }

    #[test]
    fn presets_are_of_their_type_and_render() {
        for t in TransparencyType::ALL {
            let presets = t.presets();
            if matches!(t, TransparencyType::None | TransparencyType::Uniform) {
                assert!(presets.is_empty());
                continue;
            }
            assert!(presets.len() >= 4, "{t:?}");
            for (i, m) in presets.iter().enumerate() {
                assert_eq!(TransparencyType::of(mask_kind(m), m), t, "{t:?} {i}");
                let pm =
                    tracedraw_render::render_fill_image(m, GRect::new(0.0, 0.0, 20.0, 20.0), 50.0)
                        .expect("rendered");
                // Every preset has opaque and clear parts, or a grey.
                let greys: std::collections::BTreeSet<u8> =
                    pm.data().chunks(4).map(|p| p[0]).collect();
                assert!(greys.len() > 1, "{t:?} {i}: one grey");
            }
        }
    }

    #[test]
    fn greys_and_transparency_round_trip() {
        for pct in [0.0, 25.0, 50.0, 100.0] {
            let back = gray_transparency(transparency_gray(pct));
            assert!((back - pct).abs() < 0.01, "{pct}: {back}");
        }
        assert!(gray_transparency(Color::WHITE).abs() < 1e-9);
        assert!((gray_transparency(Color::BLACK) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn a_stop_is_added_half_way_with_the_gradient_grey() {
        let mut stops = vec![Stop::new(0.0, Color::WHITE), Stop::new(1.0, Color::BLACK)];
        add_stop(&mut stops, 0);
        assert_eq!(stops.len(), 3);
        assert!((stops[1].pos - 0.5).abs() < 1e-9);
        assert!((gray_transparency(stops[1].color) - 50.0).abs() < 1.0);
        // After the last: between it and the one before.
        add_stop(&mut stops, 2);
        assert_eq!(stops.len(), 4);
        assert!((stops[2].pos - 0.75).abs() < 1e-9);
        assert!(stops.windows(2).all(|w| w[0].pos <= w[1].pos));
    }

    /// The section shows the chosen type's title, and a preset or a
    /// type button sends the whole mask (#1439).
    #[test]
    fn the_section_titles_the_type_and_masks_apply() {
        use crate::app::{App, DockerTab};
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.show_welcome = false;
        app.settings.properties_closed.clear();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: GRect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.show_dockers = true;
        app.docker_tab = DockerTab::Properties;
        for t in TransparencyType::ALL {
            match t {
                TransparencyType::None => app.clear_transparency(),
                TransparencyType::Uniform => {
                    let s = TransparencySettings {
                        amount: 40.0,
                        ..Default::default()
                    };
                    app.patch_transparency(&s, &[F::Amount]);
                }
                _ => {
                    let s = TransparencySettings {
                        kind: t.settings_kind(),
                        mask: t.presets().pop().expect("a preset"),
                        ..Default::default()
                    };
                    app.patch_transparency(&s, &[F::Mask]);
                }
            }
            let ts = app.selection_transparency();
            let mask = ts[0].mask.clone().unwrap_or(Fill::None);
            assert_eq!(TransparencyType::of(ts[0].kind, &mask), t);
            crate::i18n::with_thread_language("en", || {
                for _ in 0..3 {
                    crate::ui::dockers::tests::draw(&ctx, &mut app);
                }
                let texts = crate::ui::dockers::tests::painted_texts(&ctx, &mut app);
                assert!(
                    texts.iter().any(|x| x == &tr(t.title_key())),
                    "{t:?}: {texts:?}"
                );
            });
        }
    }

    #[test]
    fn swapping_greys_sets_both() {
        let mut m = TransparencyType::Texture.default_mask().expect("mask");
        set_mask_colors(&mut m, Color::BLACK, Color::WHITE);
        assert_eq!(mask_colors(&m), Some((Color::BLACK, Color::WHITE)));
        let mut f = TransparencyType::Fountain.default_mask().expect("mask");
        let before = f.clone();
        set_mask_colors(&mut f, Color::BLACK, Color::WHITE);
        assert_eq!(f, before);
    }
}
