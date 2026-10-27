//! The Color docker (Shift+F11). Three buttons at the top show or hide
//! its views: the color viewer (a saturation and brightness square beside
//! a hue strip), the sliders of the chosen color model, and a palette's
//! swatches. Above them the selected object's color sits beside the new
//! one, with its hexadecimal code and the gamut warning; below them Fill
//! and Outline apply the new color to the selection. When the selection
//! changes, the new color becomes the selected object's fill.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::{Color32, Mesh, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use tracedraw_core::{Color, Command, Fill, ShapeId};

/// The models the sliders offer, by their names in `Color::convert_to`.
pub const MODELS: [&str; 7] = ["RGB", "CMYK", "HSB", "HSL", "Lab", "Grayscale", "YIQ"];

/// The views the top buttons show or hide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Viewer,
    Sliders,
    Palette,
}

/// What the docker keeps between frames.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorDockerState {
    /// Color viewer, sliders and palette shown.
    pub shown: [bool; 3],
    /// The hue the viewer shows, kept while the color has none (greys).
    pub hue: f32,
    /// The palette the palette view shows (index into `App::palettes`).
    pub palette: usize,
    /// The selection the new color was last taken from.
    synced: Option<Vec<ShapeId>>,
    /// Spot inks: the tint they are applied at (percent) and the name a
    /// new ink gets.
    pub tint: f32,
    pub new_ink: String,
}

impl Default for ColorDockerState {
    fn default() -> Self {
        ColorDockerState {
            shown: [true, true, false],
            hue: 0.0,
            palette: 0,
            synced: None,
            tint: 100.0,
            new_ink: String::new(),
        }
    }
}

/// The solid color of the first selected object's fill, if it has one.
fn selected_fill(app: &App) -> Option<Color> {
    app.selected_shapes().first().and_then(|s| match &s.fill {
        Fill::Solid(c) => Some(*c),
        _ => None,
    })
}

/// Take the new color from the selection when the selection changed.
fn follow_selection(app: &mut App) {
    if app.color_docker.synced.as_deref() == Some(app.selection.as_slice()) {
        return;
    }
    app.color_docker.synced = Some(app.selection.clone());
    if let Some(c) = selected_fill(app) {
        set_color(app, c);
    }
}

/// Make `c` the new color, in the sliders' model; the viewer keeps its
/// hue for a grey.
fn set_color(app: &mut App, c: Color) {
    let model = MODELS.get(app.color_model).copied().unwrap_or("CMYK");
    app.mixer_color = c.convert_to(model);
    let (h, s, _) = crate::ui::color_picker::viewer_point(app.mixer_color);
    if s > 1e-3 {
        app.color_docker.hue = h;
    }
}

/// A 28 x 26 picture button, shaded while `on`.
fn view_button(ui: &mut Ui, view: View, tip: &str, on: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 26.0), Sense::click());
    if on {
        ui.painter().rect_filled(rect, 2.0, Tokens::tool_active());
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
    }
    let r = Rect::from_center_size(rect.center(), Vec2::splat(16.0));
    let p = ui.painter();
    match view {
        View::Viewer => {
            hsb_square(p, r, 0.0, 4);
            p.rect_stroke(
                r,
                0.0,
                Stroke::new(1.0, Tokens::icon()),
                egui::StrokeKind::Inside,
            );
        }
        View::Sliders => {
            for (i, at) in [0.3f32, 0.75, 0.5].iter().enumerate() {
                let y = r.top() + 3.0 + i as f32 * 5.0;
                p.line_segment(
                    [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
                    Stroke::new(1.0, Tokens::icon()),
                );
                p.rect_filled(
                    Rect::from_center_size(
                        Pos2::new(r.left() + r.width() * at, y),
                        Vec2::new(3.0, 4.5),
                    ),
                    0.5,
                    Tokens::icon(),
                );
            }
        }
        View::Palette => {
            let colors = [
                Color32::from_rgb(0xED, 0x1C, 0x24),
                Color32::from_rgb(0xFF, 0xF2, 0x00),
                Color32::from_rgb(0x00, 0xA6, 0x51),
                Color32::from_rgb(0x00, 0xAE, 0xEF),
                Color32::from_rgb(0x2E, 0x31, 0x92),
                Color32::from_rgb(0xEC, 0x00, 0x8C),
                Color32::WHITE,
                Color32::from_gray(0x80),
                Color32::BLACK,
            ];
            for (i, c) in colors.iter().enumerate() {
                let cell = Rect::from_min_size(
                    r.min + Vec2::new((i % 3) as f32 * 5.5, (i / 3) as f32 * 5.5),
                    Vec2::splat(5.0),
                );
                p.rect_filled(cell, 0.0, *c);
                p.rect_stroke(
                    cell,
                    0.0,
                    Stroke::new(0.5, Tokens::icon()),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }
    resp.on_hover_text(tip)
}

fn to32(c: [f32; 3]) -> Color32 {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(q(c[0]), q(c[1]), q(c[2]))
}

/// The saturation (left to right) and brightness (bottom to top) square
/// of `hue`, as an `n` x `n` grid of shaded cells.
fn hsb_square(p: &Painter, r: Rect, hue: f32, n: usize) {
    color_square(p, r, &square_colors(hue, "HSB", n), n);
}

/// The screen colours of the viewer's square for `hue` in `model`, as an
/// `(n + 1)` x `(n + 1)` grid of vertex colours, row by row from the top:
/// in CMYK the look of the inks each point gives (#907), else the
/// saturation and brightness of the hue.
fn square_colors(hue: f32, model: &str, n: usize) -> Vec<Color32> {
    let n = n.max(1);
    let cmyk = model == "CMYK";
    let mut out = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            let s = i as f32 / n as f32;
            let v = 1.0 - j as f32 / n as f32;
            out.push(to32(if cmyk {
                crate::ui::color_picker::from_viewer(hue, s, v, model).to_rgb_f32()
            } else {
                tracedraw_core::color::hsb_to_rgb(hue, s, v)
            }));
        }
    }
    out
}

/// The colours of the square `id` for `hue` in `model`, kept between
/// frames while the hue, the model and the colour engine stay the same:
/// CMYK looks cost more than a frame should spend on them every time.
fn cached_square_colors(
    ui: &Ui,
    id: egui::Id,
    hue: f32,
    model: &str,
    n: usize,
) -> std::sync::Arc<Vec<Color32>> {
    let cmyk = model == "CMYK";
    let engine = if cmyk {
        tracedraw_core::color::engine::icc().map_or(0usize, |e| std::sync::Arc::as_ptr(&e) as usize)
    } else {
        0
    };
    let key = (hue.to_bits(), cmyk, n, engine);
    type Cached = (u32, bool, usize, usize);
    let id = id.with("colors");
    if let Some((k, c)) = ui.data(|d| d.get_temp::<(Cached, std::sync::Arc<Vec<Color32>>)>(id)) {
        if k == key {
            return c;
        }
    }
    let c = std::sync::Arc::new(square_colors(hue, model, n));
    ui.data_mut(|d| d.insert_temp(id, (key, c.clone())));
    c
}

/// The viewer's square from its vertex colours (`n` x `n` cells).
fn color_square(p: &Painter, r: Rect, colors: &[Color32], n: usize) {
    let mut mesh = Mesh::default();
    let n = n.max(1);
    for j in 0..=n {
        for i in 0..=n {
            let pos = r.min
                + Vec2::new(
                    r.width() * i as f32 / n as f32,
                    r.height() * j as f32 / n as f32,
                );
            let c = colors
                .get(j * (n + 1) + i)
                .copied()
                .unwrap_or(Color32::GRAY);
            mesh.colored_vertex(pos, c);
        }
    }
    let w = (n + 1) as u32;
    for j in 0..n as u32 {
        for i in 0..n as u32 {
            let a = j * w + i;
            mesh.add_triangle(a, a + 1, a + w);
            mesh.add_triangle(a + 1, a + w + 1, a + w);
        }
    }
    p.add(Shape::mesh(mesh));
}

/// The hue strip, red at the top through the spectrum to red again: each
/// hue's pure screen colour, or in CMYK the look of its solid inks.
fn hue_strip(p: &Painter, r: Rect, model: &str) {
    let mut mesh = Mesh::default();
    let n = 36u32;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let c = to32(if model == "CMYK" {
            crate::ui::color_picker::from_viewer(t * 360.0, 1.0, 1.0, model).to_rgb_f32()
        } else {
            tracedraw_core::color::hsb_to_rgb(t * 360.0, 1.0, 1.0)
        });
        let y = r.top() + r.height() * t;
        mesh.colored_vertex(Pos2::new(r.left(), y), c);
        mesh.colored_vertex(Pos2::new(r.right(), y), c);
    }
    for k in 0..n {
        let a = k * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    p.add(Shape::mesh(mesh));
}

/// Where the viewer's ring shows `color` (saturation, brightness): at
/// the point last `picked` while the colour is still the one it gave,
/// else where the colour's screen look sits (#761).
fn shown_point(picked: Option<(f32, f32, Color)>, color: Color) -> (f32, f32) {
    match picked {
        Some((s, b, c)) if c == color => (s, b),
        _ => {
            let (_, s, b) = crate::ui::color_picker::viewer_point(color);
            (s, b)
        }
    }
}

/// The color viewer: drag in the square for saturation and brightness,
/// in the strip for the hue. Some((h, s, b)) when changed.
pub(crate) fn viewer(ui: &mut Ui, hue: f32, color: Color) -> Option<(f32, f32, f32)> {
    let side = (ui.available_width() - 34.0).clamp(80.0, 220.0);
    let mut out = None;
    ui.horizontal(|ui| {
        let (square, sq) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
        // The point last picked, while the colour is still the one it gave:
        // in a model such as CMYK many points give the same colour (all
        // the darkest ones give K 100, shown lighter than they are), and
        // the ring stays where it was put instead of jumping to where that
        // colour's screen look sits (#761).
        let picked_id = sq.id.with("picked");
        let (s, b) = shown_point(ui.data(|d| d.get_temp(picked_id)), color);
        ui.add_space(8.0);
        let (strip, st) = ui.allocate_exact_size(Vec2::new(18.0, side), Sense::click_and_drag());
        let model = crate::ui::color_picker::edit_model(color);
        let colors = cached_square_colors(ui, sq.id, hue, model, 16);
        let p = ui.painter();
        color_square(p, square, &colors, 16);
        p.rect_stroke(
            square,
            0.0,
            Stroke::new(1.0, Tokens::control_border()),
            egui::StrokeKind::Outside,
        );
        hue_strip(p, strip, model);
        p.rect_stroke(
            strip,
            0.0,
            Stroke::new(1.0, Tokens::control_border()),
            egui::StrokeKind::Outside,
        );
        // Markers: a ring in the square, a bar across the strip.
        let at = square.min + Vec2::new(square.width() * s, square.height() * (1.0 - b));
        let ring = if b > 0.55 && s < 0.45 {
            Color32::BLACK
        } else {
            Color32::WHITE
        };
        p.circle_stroke(at, 4.5, Stroke::new(1.5, ring));
        let y = strip.top() + strip.height() * (hue.rem_euclid(360.0) / 360.0);
        p.rect_stroke(
            Rect::from_min_max(
                Pos2::new(strip.left() - 2.0, y - 2.0),
                Pos2::new(strip.right() + 2.0, y + 2.0),
            ),
            1.0,
            Stroke::new(1.5, Tokens::icon()),
            egui::StrokeKind::Middle,
        );
        if sq.dragged() || sq.clicked() {
            if let Some(pos) = sq.interact_pointer_pos() {
                let ns = ((pos.x - square.left()) / square.width()).clamp(0.0, 1.0);
                let nb = 1.0 - ((pos.y - square.top()) / square.height()).clamp(0.0, 1.0);
                out = Some((hue, ns, nb));
            }
        }
        if let Some((h, ns, nb)) = out {
            // The colour this point gives, as the caller makes it.
            let c = crate::ui::color_picker::from_viewer(h, ns, nb, model);
            ui.data_mut(|d| d.insert_temp(picked_id, (ns, nb, c)));
        }
        if st.dragged() || st.clicked() {
            if let Some(pos) = st.interact_pointer_pos() {
                let t = ((pos.y - strip.top()) / strip.height()).clamp(0.0, 1.0);
                // Saturation 0 has no hue: show it at full saturation.
                let (ns, nb) = if s < 1e-3 { (1.0, b.max(0.5)) } else { (s, b) };
                out = Some((t * 360.0, ns, nb));
            }
        }
    });
    out
}

/// A slider with its letter; `value` scaled by `scale` to show.
fn slider(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    scale: f32,
) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([14.0, 18.0], egui::Label::new(label));
        let mut x = *value * scale;
        let changed = ui
            .add(crate::ui::Rail(
                egui::Slider::new(&mut x, range)
                    .show_value(true)
                    .max_decimals(1),
            ))
            .changed();
        if changed {
            *value = x / scale;
        }
        changed
    })
    .inner
}

/// The sliders of the color's model. True when one moved.
pub(crate) fn sliders(ui: &mut Ui, c: &mut Color) -> bool {
    let mut changed = false;
    match c {
        Color::Rgb { r, g, b } => {
            changed |= slider(ui, "R", r, 0.0..=255.0, 255.0);
            changed |= slider(ui, "G", g, 0.0..=255.0, 255.0);
            changed |= slider(ui, "B", b, 0.0..=255.0, 255.0);
        }
        Color::Cmyk { c, m, y, k } => {
            changed |= slider(ui, "C", c, 0.0..=100.0, 100.0);
            changed |= slider(ui, "M", m, 0.0..=100.0, 100.0);
            changed |= slider(ui, "Y", y, 0.0..=100.0, 100.0);
            changed |= slider(ui, "K", k, 0.0..=100.0, 100.0);
        }
        // Hues are degrees already.
        Color::Hsb { h, s, b } => {
            changed |= slider(ui, "H", h, 0.0..=360.0, 1.0);
            changed |= slider(ui, "S", s, 0.0..=100.0, 100.0);
            changed |= slider(ui, "B", b, 0.0..=100.0, 100.0);
        }
        Color::Hsl { h, s, l } => {
            changed |= slider(ui, "H", h, 0.0..=360.0, 1.0);
            changed |= slider(ui, "S", s, 0.0..=100.0, 100.0);
            changed |= slider(ui, "L", l, 0.0..=100.0, 100.0);
        }
        Color::Lab { l, a, b } => {
            changed |= slider(ui, "L", l, 0.0..=100.0, 1.0);
            changed |= slider(ui, "a", a, -128.0..=127.0, 1.0);
            changed |= slider(ui, "b", b, -128.0..=127.0, 1.0);
        }
        Color::Gray { v } => {
            changed |= slider(ui, "L", v, 0.0..=100.0, 100.0);
        }
        Color::Yiq { y, i, q } => {
            changed |= slider(ui, "Y", y, 0.0..=255.0, 255.0);
            changed |= slider(ui, "I", i, 0.0..=255.0, 255.0);
            changed |= slider(ui, "Q", q, 0.0..=255.0, 255.0);
        }
        // A spot ink's tint is set where the inks are listed.
        Color::Registration | Color::Spot { .. } => {}
    }
    changed
}

/// The old color (the selection's) beside the new one.
fn swatches(ui: &mut Ui, old: Option<Color>, new: Color) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 30.0), Sense::hover());
    let half = rect.width() / 2.0;
    let left = Rect::from_min_size(rect.min, Vec2::new(half, rect.height()));
    let right = Rect::from_min_size(
        rect.min + Vec2::new(half, 0.0),
        Vec2::new(half, rect.height()),
    );
    let p = ui.painter();
    match old {
        Some(c) => {
            let [r, g, b] = c.to_rgb8();
            p.rect_filled(left, 0.0, Color32::from_rgb(r, g, b));
        }
        None => {
            p.rect_filled(left, 0.0, Color32::WHITE);
            p.line_segment(
                [left.left_bottom(), left.right_top()],
                Stroke::new(1.0, Color32::from_rgb(0xE0, 0x40, 0x40)),
            );
        }
    }
    let [r, g, b] = new.to_rgb8();
    p.rect_filled(right, 0.0, Color32::from_rgb(r, g, b));
    p.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, Tokens::control_border()),
        egui::StrokeKind::Outside,
    );
}

/// Draw the docker.
pub fn color(app: &mut App, ui: &mut Ui) {
    follow_selection(app);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, view, key) in [
            (0, View::Viewer, "docker.color_viewers"),
            (1, View::Sliders, "docker.color_sliders"),
            (2, View::Palette, "docker.color_palettes"),
        ] {
            let on = app.color_docker.shown[i];
            if view_button(ui, view, &tr(key), on).clicked() {
                app.color_docker.shown[i] = !on;
            }
        }
        ui.add_space(8.0);
        let model = MODELS.get(app.color_model).copied().unwrap_or("CMYK");
        egui::ComboBox::from_id_salt("col_model")
            .selected_text(model)
            .width(100.0)
            .show_ui(ui, |ui| {
                for (i, m) in MODELS.iter().enumerate() {
                    if ui.selectable_label(app.color_model == i, *m).clicked() {
                        app.color_model = i;
                        app.mixer_color = app.mixer_color.convert_to(m);
                    }
                }
            })
            .response
            .on_hover_text(tr("docker.model"));
    });
    ui.add_space(4.0);
    let old = selected_fill(app);
    let mut c = app.mixer_color;
    let mut changed = false;
    ui.horizontal(|ui| {
        swatches(ui, old, c);
        let mut hex = c.to_hex();
        if ui
            .add(egui::TextEdit::singleline(&mut hex).desired_width(70.0))
            .lost_focus()
        {
            if let Some(nc) = Color::from_hex(&hex) {
                if nc.to_hex() != c.to_hex() {
                    c = nc;
                    changed = true;
                }
            }
        }
        if !c.in_cmyk_gamut(6.0) {
            gamut_warning(ui).on_hover_text(tr("docker.gamut_warning"));
        }
        if crate::ui::propbar::pic_button(
            ui,
            crate::ui::propbar::Pic::Pen,
            &tr("docker.eyedropper"),
            true,
            app.tool == crate::tools::Tool::ColorEyedropper,
        )
        .clicked()
        {
            app.set_tool(crate::tools::Tool::ColorEyedropper);
        }
    });
    if let Some(ec) = app.eyedropper_color {
        if ui
            .button(trf("docker.use_sampled", &[("c", &ec.to_hex())]))
            .clicked()
        {
            c = ec;
            changed = true;
        }
    }
    ui.add_space(4.0);
    if app.color_docker.shown[0] {
        if let Some((h, s, b)) = viewer(ui, app.color_docker.hue, c) {
            app.color_docker.hue = h;
            c = crate::ui::color_picker::from_viewer(
                h,
                s,
                b,
                crate::ui::color_picker::edit_model(c),
            );
            changed = true;
        }
        ui.add_space(6.0);
    }
    if app.color_docker.shown[1] {
        let mut m = c;
        if sliders(ui, &mut m) {
            c = m;
            changed = true;
        }
        ui.add_space(6.0);
    }
    if app.color_docker.shown[2] {
        if let Some(pc) = palette_view(app, ui) {
            c = pc;
            changed = true;
        }
        ui.add_space(6.0);
    }
    if changed {
        set_color(app, c);
        c = app.mixer_color;
    }
    ui.horizontal(|ui| {
        let has = !app.selection.is_empty();
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.fill")))
            .clicked()
        {
            app.apply_fill(Fill::Solid(c));
        }
        if ui
            .add_enabled(has, egui::Button::new(tr("docker.outline")))
            .clicked()
        {
            app.apply_outline_color(Some(c));
        }
        if ui.button(tr("docker.add_to_palette")).clicked() {
            let mut colors = app.doc().palette.clone();
            if !colors.contains(&c) {
                colors.push(c);
            }
            app.run(Command::SetDocumentPalette { colors });
        }
    });
    ui.add_space(6.0);
    spot_inks(app, ui, c);
}

/// The out-of-gamut sign: an orange triangle with an exclamation mark,
/// drawn rather than taken from a font (no bundled font has the warning
/// sign character, so it drew as an empty box, #949).
fn gamut_warning(ui: &mut Ui) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
    let orange = Color32::from_rgb(200, 120, 0);
    let p = ui.painter();
    let top = Pos2::new(r.center().x, r.top() + 1.5);
    let left = Pos2::new(r.left() + 1.0, r.bottom() - 1.5);
    let right = Pos2::new(r.right() - 1.0, r.bottom() - 1.5);
    p.add(Shape::convex_polygon(
        vec![top, right, left],
        orange,
        Stroke::NONE,
    ));
    let x = r.center().x;
    let white = Stroke::new(1.6, Color32::WHITE);
    p.line_segment(
        [Pos2::new(x, r.top() + 6.0), Pos2::new(x, r.bottom() - 6.0)],
        white,
    );
    p.circle_filled(Pos2::new(x, r.bottom() - 3.8), 0.9, Color32::WHITE);
    resp
}

/// The remove button of a spot ink: the multiplication sign, which the
/// bundled interface font has (a heavier cross drew as an empty box on
/// systems without a font for it, #949).
const REMOVE_INK: char = '\u{00D7}';

/// The drawing's spot inks: each applies as fill or outline at the tint;
/// a new one takes the new color's process values and the name typed.
fn spot_inks(app: &mut App, ui: &mut Ui, current: Color) {
    crate::ui::chrome::section(ui, &tr("docker.spot_inks"));
    let has = !app.selection.is_empty();
    let tint = app.color_docker.tint.clamp(0.0, 100.0) / 100.0;
    let inks = app.doc().inks.clone();
    let mut remove = None;
    for ink in &inks {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), Sense::hover());
            let [cr, cg, cb] = ink.color(tint).to_rgb8();
            ui.painter()
                .rect_filled(r, 0.0, Color32::from_rgb(cr, cg, cb));
            ui.painter().rect_stroke(
                r,
                0.0,
                Stroke::new(0.5, Tokens::icon()),
                egui::StrokeKind::Inside,
            );
            ui.label(&ink.name);
            if ui
                .add_enabled(has, egui::Button::new(tr("docker.fill")).small())
                .clicked()
            {
                app.apply_fill(Fill::Solid(ink.color(tint)));
            }
            if ui
                .add_enabled(has, egui::Button::new(tr("docker.outline")).small())
                .clicked()
            {
                app.apply_outline_color(Some(ink.color(tint)));
            }
            if ui
                .small_button(REMOVE_INK.to_string())
                .on_hover_text(tr("docker.remove_ink"))
                .clicked()
            {
                remove = Some(ink.id);
            }
        });
    }
    if !inks.is_empty() {
        ui.horizontal(|ui| {
            ui.label(tr("docker.tint"));
            ui.spacing_mut().slider_width = 140.0;
            // A rail in the slider colour: a plain slider's groove has the
            // white of fields and vanished on the panel (#949).
            ui.add(crate::ui::Rail(
                egui::Slider::new(&mut app.color_docker.tint, 0.0..=100.0)
                    .suffix(" %")
                    .max_decimals(0),
            ));
        });
    }
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.color_docker.new_ink)
                .hint_text(tr("docker.ink_name"))
                .desired_width(120.0),
        );
        let name = app.color_docker.new_ink.trim().to_string();
        let taken = inks.iter().any(|i| i.name == name);
        if ui
            .add_enabled(
                !name.is_empty() && !taken,
                egui::Button::new(tr("docker.new_ink")),
            )
            .on_hover_text(tr("docker.new_ink_tip"))
            .clicked()
        {
            add_ink(app, name, current);
            app.color_docker.new_ink.clear();
        }
    });
    if let Some(id) = remove {
        let inks: Vec<_> = inks.into_iter().filter(|i| i.id != id).collect();
        app.run(Command::SetInks { inks });
    }
}

/// Add a spot ink named `name` standing for `c` in process inks.
pub fn add_ink(app: &mut App, name: String, c: Color) -> u32 {
    let id = app.doc().next_ink_id();
    let cmyk = match c.convert_to("CMYK") {
        Color::Cmyk { c, m, y, k } => [c, m, y, k],
        _ => [0.0, 0.0, 0.0, 1.0],
    };
    let mut inks = app.doc().inks.clone();
    inks.push(tracedraw_core::document::Ink { id, name, cmyk });
    app.run(Command::SetInks { inks });
    id
}

/// The palette view: the palette chosen in the list, as swatches; a click
/// takes the swatch's color.
fn palette_view(app: &mut App, ui: &mut Ui) -> Option<Color> {
    if app.palettes.is_empty() {
        return None;
    }
    let mut chosen = app.color_docker.palette.min(app.palettes.len() - 1);
    egui::ComboBox::from_id_salt("col_palette")
        .selected_text(tr(app.palettes[chosen].name_key()))
        .width(ui.available_width().min(240.0))
        .show_ui(ui, |ui| {
            for (i, pal) in app.palettes.iter().enumerate() {
                if ui
                    .selectable_label(chosen == i, tr(pal.name_key()))
                    .clicked()
                {
                    chosen = i;
                }
            }
        });
    app.color_docker.palette = chosen;
    let mut out = None;
    let cell = 16.0;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(2.0);
        for (name, c) in app.palettes[chosen].colors.iter().take(512) {
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(cell), Sense::click());
            let [cr, cg, cb] = c.to_rgb8();
            ui.painter()
                .rect_filled(r, 0.0, Color32::from_rgb(cr, cg, cb));
            ui.painter().rect_stroke(
                r,
                0.0,
                Stroke::new(if resp.hovered() { 1.5 } else { 0.5 }, Tokens::icon()),
                egui::StrokeKind::Inside,
            );
            if resp.on_hover_text(name).clicked() {
                out = Some(*c);
            }
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::ShapeKind;

    /// The other places that drew the same remove cross as a spot ink.
    const CROSS_SOURCES: [(&str, &str); 11] = [
        ("dockers.rs", include_str!("dockers.rs")),
        ("dockers2.rs", include_str!("dockers2.rs")),
        ("effect_dialog.rs", include_str!("effect_dialog.rs")),
        ("effects_docker.rs", include_str!("effects_docker.rs")),
        ("export_docker.rs", include_str!("export_docker.rs")),
        ("focus_bar.rs", include_str!("focus_bar.rs")),
        ("font_sampler.rs", include_str!("font_sampler.rs")),
        ("lab_dialog.rs", include_str!("lab_dialog.rs")),
        ("options.rs", include_str!("options.rs")),
        (
            "print_merge_docker.rs",
            include_str!("print_merge_docker.rs"),
        ),
        ("toolbar.rs", include_str!("toolbar.rs")),
    ];

    /// The characters `src` writes as `\u{...}` escapes or as non-ASCII
    /// characters outside comments: its text icons.
    fn source_glyphs(src: &str) -> Vec<char> {
        let mut out = Vec::new();
        for line in src.lines() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            out.extend(code.chars().filter(|c| !c.is_ascii()));
            for (at, _) in code.match_indices("\\u{") {
                let rest = &code[at + 3..];
                let Some(end) = rest.find('}') else { continue };
                if let Some(c) = u32::from_str_radix(&rest[..end], 16)
                    .ok()
                    .and_then(char::from_u32)
                {
                    out.push(c);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Every text icon of the panel is in the fonts the app bundles, so
    /// it never draws as an empty box where no system font has it: the
    /// remove button of a spot ink and the out-of-gamut sign did (#949).
    /// The other remove buttons use the same cross, which the bundled
    /// fonts have, instead of the heavy cross U+2715, which they lack.
    #[test]
    fn every_text_icon_is_in_the_bundled_fonts() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts_bundled(&ctx);
        let mut out = ctx.run_ui(Default::default(), |_| {});
        out.textures_delta.clear();
        let font = egui::FontId::proportional(12.0);
        let has = |c: char| ctx.fonts_mut(|f| f.has_glyph(&font, c));
        let glyphs = source_glyphs(include_str!("color_docker.rs"));
        assert!(glyphs.contains(&REMOVE_INK), "{glyphs:?}");
        let missing: Vec<String> = glyphs
            .into_iter()
            .filter(|c| !has(*c))
            .map(|c| format!("U+{:04X}", c as u32))
            .collect();
        assert!(missing.is_empty(), "not in the bundled fonts: {missing:?}");
        let heavy = char::from_u32(0x2715).expect("a char");
        assert!(!has(heavy));
        for (name, src) in CROSS_SOURCES {
            let glyphs = source_glyphs(src);
            assert!(glyphs.contains(&REMOVE_INK), "{name}");
            assert!(!glyphs.contains(&heavy), "{name}");
        }
    }

    /// The spot ink tint slider shows its groove: a rail as wide as the
    /// slider, in the slider rail colour rather than the white of fields
    /// that the panel is drawn on (#949).
    #[test]
    fn the_tint_slider_shows_its_rail() {
        let mut app = App::headless();
        add_ink(
            &mut app,
            "Gold".into(),
            Color::cmyk_pct(0.0, 20.0, 80.0, 10.0),
        );
        let ctx = crate::theme::ui_context();
        ctx.set_visuals(crate::theme::visuals());
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(320.0, 400.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| spot_inks(&mut app, ui, Color::BLACK));
        out.textures_delta.clear();
        let rail = Tokens::slider_rail();
        assert_ne!(rail, Tokens::field());
        assert_ne!(rail, crate::theme::visuals().panel_fill);
        let widest = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Rect(r) if r.fill == rail => Some(r.rect.width()),
                _ => None,
            })
            .fold(0.0f32, f32::max);
        assert!(widest >= 100.0, "no visible tint rail ({widest})");
    }

    /// The ring stays at the point picked while the colour is the one it
    /// gave (#761); another colour shows where it sits.
    #[test]
    fn the_ring_stays_where_a_dark_cmyk_colour_was_picked() {
        let picked = crate::ui::color_picker::from_viewer(0.0, 0.05, 0.02, "CMYK");
        assert_eq!(
            shown_point(Some((0.05, 0.02, picked)), picked),
            (0.05, 0.02)
        );
        let other = Color::cmyk_pct(0.0, 100.0, 100.0, 0.0);
        let (s, b) = shown_point(Some((0.05, 0.02, picked)), other);
        assert!(b > 0.5 && s > 0.5, "{s} {b}");
    }

    /// In CMYK the ring reaches every corner of the square, and the
    /// square's corners show the ink extremes, not screen colours the
    /// press cannot print (#907).
    #[test]
    fn a_cmyk_colour_reaches_the_corners_of_the_viewer() {
        for (c, at) in [
            (Color::cmyk_pct(0.0, 0.0, 0.0, 0.0), (0.0, 1.0)),
            (Color::cmyk_pct(0.0, 100.0, 100.0, 0.0), (1.0, 1.0)),
            (Color::cmyk_pct(0.0, 0.0, 0.0, 100.0), (0.0, 0.0)),
            (Color::cmyk_pct(0.0, 100.0, 100.0, 100.0), (1.0, 0.0)),
        ] {
            let (s, b) = shown_point(None, c);
            assert!(
                (s - at.0).abs() < 1e-4 && (b - at.1).abs() < 1e-4,
                "{c:?}: {s} {b}"
            );
        }
        let n = 4;
        let colors = square_colors(0.0, "CMYK", n);
        let corner = |i: usize, j: usize| colors[j * (n + 1) + i];
        let look = |c: Color| to32(c.to_rgb_f32());
        assert_eq!(corner(0, 0), look(Color::cmyk_pct(0.0, 0.0, 0.0, 0.0)));
        assert_eq!(corner(n, 0), look(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0)));
        assert_eq!(corner(0, n), look(Color::cmyk_pct(0.0, 0.0, 0.0, 100.0)));
        assert_eq!(
            corner(n, n),
            look(Color::cmyk_pct(0.0, 100.0, 100.0, 100.0))
        );
        // RGB keeps the screen's own square, black at the bottom.
        let rgb = square_colors(0.0, "RGB", n);
        assert_eq!(rgb[n * (n + 1)], Color32::BLACK);
        assert_eq!(rgb[n], Color32::from_rgb(255, 0, 0));
    }

    /// The docker's viewer makes a colour of the chosen model, and in
    /// CMYK its bottom right corner is the hue's inks over full black.
    #[test]
    fn the_docker_viewer_picks_in_the_model() {
        let c = crate::ui::color_picker::from_viewer(240.0, 1.0, 0.0, "CMYK");
        assert_eq!(c, Color::cmyk_pct(100.0, 100.0, 0.0, 100.0));
        let r = crate::ui::color_picker::from_viewer(0.0, 1.0, 0.0, "RGB");
        assert_eq!(
            r,
            Color::Rgb {
                r: 0.0,
                g: 0.0,
                b: 0.0
            }
        );
    }

    #[test]
    fn spot_inks_are_added_applied_and_removed() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        let ink = add_ink(
            &mut app,
            "Gold".into(),
            Color::cmyk_pct(0.0, 20.0, 80.0, 10.0),
        );
        assert_eq!(app.doc().inks.len(), 1);
        app.select(vec![id]);
        let gold = app.doc().ink(ink).expect("ink").color(0.5);
        app.apply_fill(Fill::Solid(gold));
        let Fill::Solid(Color::Spot { ink: i, tint, .. }) =
            app.doc().find_shape(id).expect("shape").fill
        else {
            panic!("not a spot fill");
        };
        assert_eq!((i, tint), (ink, 0.5));
        // The docker draws with inks listed.
        let ctx = crate::theme::ui_context();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            spot_inks(&mut app, ui, Color::BLACK)
        });
        out.textures_delta.clear();
        app.run(Command::SetInks { inks: Vec::new() });
        assert!(app.doc().inks.is_empty());
    }

    #[test]
    fn the_new_color_follows_the_selection() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        let red = Color::rgb8(255, 0, 0);
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(red),
        });
        app.select(vec![id]);
        // RGB, so the red does not go through the CMYK gamut.
        app.color_model = 0;
        follow_selection(&mut app);
        assert_eq!(app.mixer_color.to_hex(), "#ff0000");
        // In the sliders' model, and the viewer took its hue.
        assert_eq!(app.mixer_color.model_name(), MODELS[app.color_model]);
        assert!(app.color_docker.hue.abs() < 1.0);
        // Changing it by hand is kept until the selection changes.
        app.mixer_color = Color::rgb8(0, 0, 255);
        follow_selection(&mut app);
        assert_eq!(app.mixer_color.to_hex(), "#0000ff");
    }

    #[test]
    fn hue_sliders_count_degrees() {
        let mut c = Color::Hsb {
            h: 120.0,
            s: 1.0,
            b: 1.0,
        };
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(320.0, 400.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            sliders(ui, &mut c);
        });
        out.textures_delta.clear();
        // Drawing does not change it: 120 degrees stays 120.
        assert!(matches!(c, Color::Hsb { h, .. } if (h - 120.0).abs() < 1e-4));
    }

    #[test]
    fn the_docker_draws_every_view_and_model() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.color_docker.shown = [true, true, true];
        for (model, name) in MODELS.iter().enumerate() {
            app.color_model = model;
            app.mixer_color = app.mixer_color.convert_to(name);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 900.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| color(&mut app, ui));
            out.textures_delta.clear();
        }
    }
}
