//! The Color docker (Shift+F11). Three buttons at the top show or hide
//! its views: the color viewer (a saturation and brightness square beside
//! a hue strip), the sliders of the chosen color model, and a palette's
//! swatches. Above them the selected object's color sits beside the new
//! one, with its hexadecimal code and the gamut warning; below them Fill
//! and Outline apply the new color to the selection (to both at once
//! while the lock is on) and a "..." button drops the other actions (No
//! Fill, No Outline, Swap Colors, Add to Document Palette, Add to Custom
//! Spot Colors, Add to Color Styles Palette). When the selection changes,
//! the new color becomes the selected object's fill.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
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
    /// The selection the new color was last taken from, with the fill it
    /// had then (a fill changed, as by Convert Colors, is taken again).
    synced: Option<(Vec<ShapeId>, Option<Color>)>,
    /// Spot colours: the tint they are applied at (percent; the inks'
    /// Fill and Outline, a spot swatch of the palette view) and the name
    /// a new ink gets.
    pub tint: f32,
    pub new_ink: String,
    /// Fill and outline locked together: Fill and Outline each give the
    /// selection the new colour as both, in one step.
    pub lock: bool,
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
            lock: false,
        }
    }
}

/// The solid color of the first selected object's fill, if it has one.
fn selected_fill(app: &App) -> Option<Color> {
    app.selected_rc().first().and_then(|s| match &s.fill {
        Fill::Solid(c) => Some(*c),
        _ => None,
    })
}

/// Take the new color from the selection when the selection changed, or
/// when its fill went to another colour model (Tools > Convert Colors). A
/// CMYK or RGB fill brings the sliders to its own model, so a fill
/// converted to RGB shows RGB values, not the CMYK ones of the model the
/// sliders were in (#1254). A fill changed within its model (a colour
/// applied, an undo) leaves the new colour as it is.
fn follow_selection(app: &mut App) {
    let fill = selected_fill(app);
    let same = app.color_docker.synced.as_ref().is_some_and(|(sel, f)| {
        sel.as_slice() == app.selection.as_slice()
            && f.map(|c| c.model_name()) == fill.map(|c| c.model_name())
    });
    app.color_docker.synced = Some((app.selection.clone(), fill));
    if same {
        return;
    }
    if let Some(c) = fill {
        if matches!(c, Color::Cmyk { .. } | Color::Rgb { .. }) {
            if let Some(m) = MODELS.iter().position(|m| *m == c.model_name()) {
                app.color_model = m;
            }
        }
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
    resp.rich_tip(tip)
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

/// The colour `c` with its component `i` (in the order the sliders show
/// them) set to `v`, in the component's own scale (0 to 1 for RGB, CMYK,
/// saturation, brightness, lightness, grey and YIQ; degrees for a hue;
/// Lab's own ranges).
fn with_channel(c: Color, i: usize, v: f32) -> Color {
    let mut c = c;
    match (&mut c, i) {
        (Color::Rgb { r, .. }, 0) => *r = v,
        (Color::Rgb { g, .. }, 1) => *g = v,
        (Color::Rgb { b, .. }, 2) => *b = v,
        (Color::Cmyk { c, .. }, 0) => *c = v,
        (Color::Cmyk { m, .. }, 1) => *m = v,
        (Color::Cmyk { y, .. }, 2) => *y = v,
        (Color::Cmyk { k, .. }, 3) => *k = v,
        (Color::Hsb { h, .. }, 0) => *h = v,
        (Color::Hsb { s, .. }, 1) => *s = v,
        (Color::Hsb { b, .. }, 2) => *b = v,
        (Color::Hsl { h, .. }, 0) => *h = v,
        (Color::Hsl { s, .. }, 1) => *s = v,
        (Color::Hsl { l, .. }, 2) => *l = v,
        (Color::Lab { l, .. }, 0) => *l = v,
        (Color::Lab { a, .. }, 1) => *a = v,
        (Color::Lab { b, .. }, 2) => *b = v,
        (Color::Gray { v: g }, 0) => *g = v,
        (Color::Yiq { y, .. }, 0) => *y = v,
        (Color::Yiq { i, .. }, 1) => *i = v,
        (Color::Yiq { q, .. }, 2) => *q = v,
        _ => {}
    }
    c
}

/// The components of `c` as bits, a key that tells colours apart.
fn color_bits(c: Color) -> (u8, [u32; 4]) {
    let b = |v: f32| v.to_bits();
    match c {
        Color::Rgb { r, g, b: bb } => (0, [b(r), b(g), b(bb), 0]),
        Color::Cmyk { c, m, y, k } => (1, [b(c), b(m), b(y), b(k)]),
        Color::Hsb { h, s, b: bb } => (2, [b(h), b(s), b(bb), 0]),
        Color::Hsl { h, s, l } => (3, [b(h), b(s), b(l), 0]),
        Color::Lab { l, a, b: bb } => (4, [b(l), b(a), b(bb), 0]),
        Color::Gray { v } => (5, [b(v), 0, 0, 0]),
        Color::Yiq { y, i, q } => (6, [b(y), b(i), b(q), 0]),
        Color::Spot { ink, tint, .. } => (7, [ink, b(tint), 0, 0]),
        Color::Registration => (8, [0; 4]),
    }
}

/// Steps of a slider's gradient rail.
const RAIL_STEPS: usize = 16;

/// The screen colours along the rail of component `i` of `base`, from
/// `lo` to `hi` in the component's scale: the colour each point of the
/// rail would give, kept between frames while the other components stay
/// the same (a CMYK look costs more than a frame should spend on it every
/// time) and the colour profiles are the same.
fn rail_colors(
    ui: &Ui,
    id: egui::Id,
    base: Color,
    i: usize,
    lo: f32,
    hi: f32,
) -> std::sync::Arc<Vec<Color32>> {
    use std::sync::Arc;
    // RGB, CMYK and Lab (and a spot colour, shown by its CMYK) reach the
    // screen through the profiles: their rails are kept for the engine
    // they were worked out with. The cache holds that engine, so a new
    // one can never get its address.
    let engine = if matches!(
        base,
        Color::Rgb { .. } | Color::Cmyk { .. } | Color::Lab { .. } | Color::Spot { .. }
    ) {
        tracedraw_core::color::engine::icc()
    } else {
        None
    };
    type Key = ((u8, [u32; 4]), usize, u32, u32);
    type Cached = (
        Key,
        Option<Arc<tracedraw_core::color::IccEngine>>,
        Arc<Vec<Color32>>,
    );
    let key: Key = (color_bits(base), i, lo.to_bits(), hi.to_bits());
    let id = id.with("rail");
    if let Some((k, e, c)) = ui.data(|d| d.get_temp::<Cached>(id)) {
        let same_engine = match (&e, &engine) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if k == key && same_engine {
            return c;
        }
    }
    let colors: Vec<Color32> = (0..=RAIL_STEPS)
        .map(|k| {
            let t = k as f32 / RAIL_STEPS as f32;
            to32(with_channel(base, i, lo + (hi - lo) * t).to_rgb_f32())
        })
        .collect();
    let c = Arc::new(colors);
    ui.data_mut(|d| d.insert_temp::<Cached>(id, (key, engine, c.clone())));
    c
}

/// The gradient rail of a slider: `colors` left to right across `r`,
/// with a thin border.
fn gradient_rail(r: Rect, colors: &[Color32]) -> Shape {
    let n = colors.len().saturating_sub(1).max(1) as u32;
    let mut mesh = Mesh::default();
    for (k, c) in colors.iter().enumerate() {
        let x = r.left() + r.width() * k as f32 / n as f32;
        mesh.colored_vertex(Pos2::new(x, r.top()), *c);
        mesh.colored_vertex(Pos2::new(x, r.bottom()), *c);
    }
    for k in 0..n {
        let a = k * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    Shape::Vec(vec![
        Shape::mesh(mesh),
        Shape::rect_stroke(
            r,
            0.0,
            Stroke::new(1.0, Tokens::control_border()),
            egui::StrokeKind::Outside,
        ),
    ])
}

/// The height of a slider's gradient rail.
const RAIL_HEIGHT: f32 = 8.0;

/// A slider with its letter and its value box; `value` is component `i`
/// of `base`, scaled by `scale` to show. The rail is the gradient of
/// the colours the component gives (cyan for C, red for R, the spectrum
/// for a hue), with the other components as they are.
#[allow(clippy::too_many_arguments)]
fn slider(
    ui: &mut Ui,
    label: &str,
    name: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    scale: f32,
    base: Color,
    i: usize,
) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([14.0, 18.0], egui::Label::new(label));
        let mut x = *value * scale;
        let width = (ui.available_width() - 64.0).clamp(60.0, 220.0);
        // The rail goes under the slider: its shape is set once the
        // slider says where it is.
        let rail = ui.painter().add(Shape::Noop);
        let resp = ui
            .scope(|ui| {
                ui.spacing_mut().slider_width = width;
                ui.spacing_mut().slider_rail_height = RAIL_HEIGHT;
                ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
                ui.add(
                    egui::Slider::new(&mut x, range.clone())
                        .show_value(true)
                        .max_decimals(1),
                )
            })
            .inner
            .rich_tip(crate::ui::tips::pair(name, "color.channel_desc"));
        let colors = rail_colors(
            ui,
            resp.id,
            base,
            i,
            *range.start() / scale,
            *range.end() / scale,
        );
        let r = Rect::from_min_size(
            Pos2::new(resp.rect.left(), resp.rect.center().y - RAIL_HEIGHT / 2.0),
            Vec2::new(width, RAIL_HEIGHT),
        );
        ui.painter().set(rail, gradient_rail(r, &colors));
        let changed = resp.changed();
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
    let base = *c;
    match c {
        Color::Rgb { r, g, b } => {
            changed |= slider(ui, "R", "color.channel_red", r, 0.0..=255.0, 255.0, base, 0);
            changed |= slider(
                ui,
                "G",
                "color.channel_green",
                g,
                0.0..=255.0,
                255.0,
                base,
                1,
            );
            changed |= slider(
                ui,
                "B",
                "color.channel_blue",
                b,
                0.0..=255.0,
                255.0,
                base,
                2,
            );
        }
        Color::Cmyk { c, m, y, k } => {
            changed |= slider(
                ui,
                "C",
                "color.channel_cyan",
                c,
                0.0..=100.0,
                100.0,
                base,
                0,
            );
            changed |= slider(
                ui,
                "M",
                "color.channel_magenta",
                m,
                0.0..=100.0,
                100.0,
                base,
                1,
            );
            changed |= slider(
                ui,
                "Y",
                "color.channel_yellow",
                y,
                0.0..=100.0,
                100.0,
                base,
                2,
            );
            changed |= slider(
                ui,
                "K",
                "color.channel_black",
                k,
                0.0..=100.0,
                100.0,
                base,
                3,
            );
        }
        // Hues are degrees already.
        Color::Hsb { h, s, b } => {
            changed |= slider(ui, "H", "color.channel_hue", h, 0.0..=360.0, 1.0, base, 0);
            changed |= slider(
                ui,
                "S",
                "color.channel_saturation",
                s,
                0.0..=100.0,
                100.0,
                base,
                1,
            );
            changed |= slider(
                ui,
                "B",
                "color.channel_brightness",
                b,
                0.0..=100.0,
                100.0,
                base,
                2,
            );
        }
        Color::Hsl { h, s, l } => {
            changed |= slider(ui, "H", "color.channel_hue", h, 0.0..=360.0, 1.0, base, 0);
            changed |= slider(
                ui,
                "S",
                "color.channel_saturation",
                s,
                0.0..=100.0,
                100.0,
                base,
                1,
            );
            changed |= slider(
                ui,
                "L",
                "color.channel_lightness",
                l,
                0.0..=100.0,
                100.0,
                base,
                2,
            );
        }
        Color::Lab { l, a, b } => {
            changed |= slider(
                ui,
                "L",
                "color.channel_lightness",
                l,
                0.0..=100.0,
                1.0,
                base,
                0,
            );
            changed |= slider(
                ui,
                "a",
                "color.channel_lab_a",
                a,
                -128.0..=127.0,
                1.0,
                base,
                1,
            );
            changed |= slider(
                ui,
                "b",
                "color.channel_lab_b",
                b,
                -128.0..=127.0,
                1.0,
                base,
                2,
            );
        }
        Color::Gray { v } => {
            changed |= slider(
                ui,
                "L",
                "color.channel_gray",
                v,
                0.0..=100.0,
                100.0,
                base,
                0,
            );
        }
        Color::Yiq { y, i, q } => {
            changed |= slider(
                ui,
                "Y",
                "color.channel_luma",
                y,
                0.0..=255.0,
                255.0,
                base,
                0,
            );
            changed |= slider(
                ui,
                "I",
                "color.channel_yiq_i",
                i,
                0.0..=255.0,
                255.0,
                base,
                1,
            );
            changed |= slider(
                ui,
                "Q",
                "color.channel_yiq_q",
                q,
                0.0..=255.0,
                255.0,
                base,
                2,
            );
        }
        // A spot ink's tint is set where the inks are listed.
        Color::Registration | Color::Spot { .. } => {}
    }
    changed
}

/// A swatch of `c` in `r`; a white square crossed by a red diagonal for
/// no colour.
fn swatch(p: &Painter, r: Rect, c: Option<Color>) {
    match c {
        Some(c) => {
            let [cr, cg, cb] = c.to_rgb8();
            p.rect_filled(r, 0.0, Color32::from_rgb(cr, cg, cb));
        }
        None => {
            p.rect_filled(r, 0.0, Color32::WHITE);
            p.line_segment(
                [r.left_bottom(), r.right_top()],
                Stroke::new(1.0, Color32::from_rgb(0xE0, 0x40, 0x40)),
            );
        }
    }
    p.rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Tokens::control_border()),
        egui::StrokeKind::Outside,
    );
}

/// The fill-over-outline swatch: the new colour as the fill in front of
/// the selection's outline colour behind it.
fn fill_outline_swatch(ui: &mut Ui, fill: Color, outline: Option<Color>) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(38.0, 34.0), Sense::hover());
    let side = 24.0;
    let p = ui.painter();
    swatch(
        p,
        Rect::from_min_size(rect.min + Vec2::new(12.0, 8.0), Vec2::splat(side)),
        outline,
    );
    swatch(
        p,
        Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::splat(side)),
        Some(fill),
    );
    resp
}

/// The colour of the first selected object's outline, else the outline
/// colour new graphic objects get (none after No Outline asked for the
/// defaults). The selection is the shared one gathered once a frame, not
/// a copy of every selected object.
fn selected_outline(app: &App) -> Option<Color> {
    match app.selected_rc().first() {
        Some(s) => s.stroke.as_ref().map(|st| st.color),
        None => app
            .doc()
            .defaults
            .graphic
            .stroke
            .as_ref()
            .map(|st| st.color),
    }
}

/// The reset of the panel: views, model, tint and lock back to their
/// defaults.
pub(crate) fn reset(app: &mut App) {
    let d = ColorDockerState::default();
    app.color_docker.shown = d.shown;
    app.color_docker.tint = d.tint;
    app.color_docker.lock = d.lock;
    app.color_docker.hue = d.hue;
    let model = MODELS.iter().position(|m| *m == "CMYK").unwrap_or(1);
    if app.color_model != model {
        app.color_model = model;
        app.mixer_color = app.mixer_color.convert_to("CMYK");
    }
}

/// The gear's menu: the views, the lock and the reset of the panel.
fn options_menu(app: &mut App, ui: &mut Ui) {
    for (i, key) in [
        (0, "docker.color_viewers"),
        (1, "docker.color_sliders"),
        (2, "docker.color_palettes"),
    ] {
        let mut on = app.color_docker.shown[i];
        if ui.checkbox(&mut on, tr(key)).changed() {
            app.color_docker.shown[i] = on;
        }
    }
    ui.separator();
    let mut lock = app.color_docker.lock;
    if ui.checkbox(&mut lock, tr("docker.lock_colors")).changed() {
        app.color_docker.lock = lock;
    }
    ui.separator();
    if ui.button(tr("docker.reset_panel")).clicked() {
        reset(app);
        ui.close();
    }
}

/// The top row: the view buttons, the model list across the rest of the
/// width, the gear.
fn top_row(app: &mut App, ui: &mut Ui) {
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
        ui.add_space(6.0);
        let gear = 28.0;
        let model = MODELS.get(app.color_model).copied().unwrap_or("CMYK");
        let width = (ui.available_width() - gear - 10.0).clamp(80.0, 240.0);
        egui::ComboBox::from_id_salt("col_model")
            .selected_text(model)
            .width(width)
            .show_ui(ui, |ui| {
                for (i, m) in MODELS.iter().enumerate() {
                    if ui.selectable_label(app.color_model == i, *m).clicked() {
                        app.color_model = i;
                        app.mixer_color = app.mixer_color.convert_to(m);
                    }
                }
            })
            .response
            .rich_tip(tr("docker.model"));
        ui.add_space(4.0);
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(gear, 26.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
        }
        crate::ui::icons::draw_action(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
            crate::ui::icons::Action::Options,
            Tokens::icon(),
        );
        let resp = resp.rich_tip(crate::ui::tips::keyed("docker.panel_options"));
        egui::Popup::menu(&resp)
            .id(egui::Id::new("col_options_menu"))
            .show(|ui| options_menu(app, ui));
    });
}

/// The hex row: the new colour's code, typed values applied when the
/// field is left, and the gamut warning.
fn hex_row(ui: &mut Ui, c: &mut Color) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(tr("docker.hex"));
        let mut hex = c.to_hex();
        if ui
            .add(egui::TextEdit::singleline(&mut hex).desired_width(64.0))
            .rich_tip(crate::ui::tips::keyed("docker.hex_color"))
            .lost_focus()
        {
            if let Some(nc) = Color::from_hex(&hex) {
                if nc.to_hex() != c.to_hex() {
                    *c = nc;
                    changed = true;
                }
            }
        }
        if !c.in_cmyk_gamut(6.0) {
            gamut_warning(ui).rich_tip(tr("docker.gamut_warning"));
        }
    });
    changed
}

/// Draw the docker.
pub fn color(app: &mut App, ui: &mut Ui) {
    follow_selection(app);
    top_row(app, ui);
    ui.add_space(4.0);
    let mut c = app.mixer_color;
    let mut changed = false;
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
        ui.add_space(2.0);
    }
    changed |= hex_row(ui, &mut c);
    ui.add_space(4.0);
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
    actions(app, ui, c);
    ui.add_space(6.0);
    // One Tint control: under the palette's swatches when they show.
    let tint_shown = app.color_docker.shown[2] && !app.palettes.is_empty();
    spot_inks(app, ui, c, !tint_shown);
}

/// The text of the button that drops the other actions.
pub(crate) const MORE_ACTIONS: &str = "...";

/// A small frameless text button; `enabled` or pale.
fn small_action(ui: &mut Ui, enabled: bool, text: &str) -> Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(text).small()).frame(false),
    )
}

/// The bottom strip: the lock, the fill-over-outline swatch, the
/// eyedropper and the button that drops the other actions (No Fill, No
/// Outline, Swap Colors, and adding the new colour `c` to the document
/// palette, the spot inks or the colour styles); under the swatch, Fill
/// and Outline apply `c` to the selection (both at once while the lock
/// is on).
fn actions(app: &mut App, ui: &mut Ui, c: Color) {
    let has = !app.selection.is_empty();
    let lock = app.color_docker.lock;
    let outline = selected_outline(app);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if crate::ui::propbar::pic_button(
            ui,
            crate::ui::propbar::Pic::Lock(lock),
            &crate::ui::tips::keyed("docker.lock_colors"),
            true,
            lock,
        )
        .clicked()
        {
            app.color_docker.lock = !lock;
        }
        fill_outline_swatch(ui, c, outline).rich_tip_with(|| {
            format!(
                "{}\n{}",
                trf("docker.swatch_fill", &[("c", &c.to_hex())]),
                trf(
                    "docker.swatch_outline",
                    &[(
                        "c",
                        &outline.map_or_else(|| tr("docker.fill_none"), |o| o.to_hex())
                    )],
                ),
            )
        });
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
        ui.menu_button(MORE_ACTIONS, |ui| more_actions(app, ui, c, has))
            .response
            .rich_tip(crate::ui::tips::keyed("docker.more_actions"));
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.add_space(30.0);
        if small_action(ui, has, &tr("docker.fill"))
            .rich_tip(crate::ui::tips::pair(
                "docker.fill",
                "docker.color_fill_desc",
            ))
            .rich_disabled_tip(crate::ui::tips::pair(
                "docker.fill",
                "docker.color_fill_desc",
            ))
            .clicked()
        {
            if lock {
                app.apply_fill_and_outline(c);
            } else {
                app.apply_fill(Fill::Solid(c));
            }
        }
        if small_action(ui, has, &tr("docker.outline"))
            .rich_tip(crate::ui::tips::pair(
                "docker.outline",
                "docker.color_outline_desc",
            ))
            .rich_disabled_tip(crate::ui::tips::pair(
                "docker.outline",
                "docker.color_outline_desc",
            ))
            .clicked()
        {
            if lock {
                app.apply_fill_and_outline(c);
            } else {
                app.apply_outline_color(Some(c));
            }
        }
    });
    if let Some(ec) = app.eyedropper_color {
        if ui
            .add(egui::Button::new(trf("docker.use_sampled", &[("c", &ec.to_hex())])).small())
            .rich_tip(crate::ui::tips::keyed("docker.use_sampled_tip"))
            .clicked()
        {
            set_color(app, ec);
        }
    }
}

/// The items of the "..." menu; `has`: something is selected.
fn more_actions(app: &mut App, ui: &mut Ui, c: Color, has: bool) {
    if ui
        .add_enabled(has, egui::Button::new(tr("docker.no_fill")))
        .clicked()
    {
        app.apply_fill(Fill::None);
        ui.close();
    }
    if ui
        .add_enabled(has, egui::Button::new(tr("docker.no_outline_action")))
        .clicked()
    {
        app.apply_outline_color(None);
        ui.close();
    }
    if ui
        .add_enabled(has, egui::Button::new(tr("docker.swap_colors")))
        .clicked()
    {
        app.swap_colors();
        ui.close();
    }
    ui.separator();
    if ui.button(tr("docker.add_to_document_palette")).clicked() {
        app.add_to_document_palette(c);
        ui.close();
    }
    if ui.button(tr("docker.add_to_spot_colors")).clicked() {
        let name = spot_ink_name(app, c);
        add_ink(app, name, c);
        app.color_docker.new_ink.clear();
        ui.close();
    }
    if ui.button(tr("docker.add_to_color_styles")).clicked() {
        crate::ui::dockers2::add_color_style(app, c);
        ui.close();
    }
}

/// The name of a spot ink added from the "..." menu: the name typed in
/// Ink name, else the colour's hexadecimal code; either made unique
/// among the drawing's inks with a number.
fn spot_ink_name(app: &App, c: Color) -> String {
    let typed = app.color_docker.new_ink.trim();
    let base = if typed.is_empty() {
        c.to_hex()
    } else {
        typed.to_string()
    };
    let taken = |n: &str| app.doc().inks.iter().any(|i| i.name == n);
    if !taken(&base) {
        return base;
    }
    let numbered = (2..).map(|k| format!("{base} {k}")).find(|n| !taken(n));
    numbered.unwrap_or(base)
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
/// `with_tint`: the Tint control shows here (not when the palette view
/// already shows it).
fn spot_inks(app: &mut App, ui: &mut Ui, current: Color, with_tint: bool) {
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
                .rich_tip(crate::ui::tips::pair("docker.fill", "docker.ink_fill_desc"))
                .clicked()
            {
                app.apply_fill(Fill::Solid(ink.color(tint)));
            }
            if ui
                .add_enabled(has, egui::Button::new(tr("docker.outline")).small())
                .rich_tip(crate::ui::tips::pair(
                    "docker.outline",
                    "docker.ink_outline_desc",
                ))
                .clicked()
            {
                app.apply_outline_color(Some(ink.color(tint)));
            }
            if ui
                .small_button(REMOVE_INK.to_string())
                .rich_tip(tr("docker.remove_ink"))
                .clicked()
            {
                remove = Some(ink.id);
            }
        });
    }
    if with_tint && !inks.is_empty() {
        tint_row(app, ui);
    }
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.color_docker.new_ink)
                .hint_text(tr("docker.ink_name"))
                .desired_width(120.0),
        )
        .rich_tip(crate::ui::tips::pair(
            "docker.ink_name",
            "docker.ink_name_desc",
        ));
        let name = app.color_docker.new_ink.trim().to_string();
        let taken = inks.iter().any(|i| i.name == name);
        if ui
            .add_enabled(
                !name.is_empty() && !taken,
                egui::Button::new(tr("docker.new_ink")),
            )
            .rich_tip(tr("docker.new_ink_tip"))
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

/// A palette swatch clicked: a spot colour at the tint `t` (0 to 1) of
/// its ink, as the inks' Fill and Outline apply it; any other colour as
/// it is, so a tint chosen for an ink never lightens a plain swatch.
/// `t` 1 is the swatch itself.
pub(crate) fn at_tint(c: Color, t: f32) -> Color {
    match c {
        Color::Spot {
            ink, c, m, y, k, ..
        } if t < 1.0 => Color::Spot {
            ink,
            tint: t.max(0.0),
            c,
            m,
            y,
            k,
        },
        other => other,
    }
}

/// The palette view: the palette chosen in the list, as swatches, and
/// the tint under them; a click takes the swatch's color (a spot colour
/// at the tint).
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
            if resp.rich_tip(name).clicked() {
                out = Some(*c);
            }
        }
    });
    tint_row(app, ui);
    out.map(|c| at_tint(c, app.color_docker.tint / 100.0))
}

/// The tint: a box and a slider, 0 to 100 %.
fn tint_row(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(tr("docker.tint"));
        ui.spacing_mut().slider_width = (ui.available_width() - 70.0).clamp(60.0, 140.0);
        // A rail in the slider colour: a plain slider's groove has the
        // white of fields and vanished on the panel (#949).
        ui.add(crate::ui::Rail(
            egui::Slider::new(&mut app.color_docker.tint, 0.0..=100.0)
                .suffix(" %")
                .max_decimals(0),
        ))
        .rich_tip(crate::ui::tips::pair("docker.tint", "docker.tint_desc"));
    });
}

#[cfg(test)]
pub(crate) mod tests {
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
    pub(crate) fn source_glyphs(src: &str) -> Vec<char> {
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
        let mut out = ctx.run_ui(input, |ui| spot_inks(&mut app, ui, Color::BLACK, true));
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
            spot_inks(&mut app, ui, Color::BLACK, true)
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

    /// A fill converted to another model under the same selection
    /// (Convert Colors to RGB) is taken again, and the sliders go to its
    /// model (#1254); a fill changed within its model is not.
    #[test]
    fn the_new_color_follows_a_converted_fill_into_its_model() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0)),
        });
        app.select(vec![id]);
        follow_selection(&mut app);
        assert_eq!(MODELS[app.color_model], "CMYK");
        assert_eq!(app.mixer_color.model_name(), "CMYK");
        assert!(
            app.convert_colors(&crate::ui::convert_colors::ConvertColorsState {
                to: tracedraw_core::color_convert::ColorTarget::Rgb,
                whole: false,
                spots: false,
            })
        );
        follow_selection(&mut app);
        assert_eq!(MODELS[app.color_model], "RGB");
        assert_eq!(app.mixer_color.model_name(), "RGB");
        // Another RGB colour given to it: the new colour stays.
        let mixed = app.mixer_color;
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(Color::rgb8(0, 0, 255)),
        });
        follow_selection(&mut app);
        assert_eq!(app.mixer_color, mixed);
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

    /// A slider's rail is the gradient of its component: the cyan rail
    /// of a CMYK colour goes from no cyan to full cyan with the other inks
    /// as they are, the red rail of an RGB colour from no red to full red.
    #[test]
    fn slider_rails_run_through_their_component() {
        let c = Color::cmyk_pct(40.0, 10.0, 0.0, 0.0);
        assert_eq!(
            with_channel(c, 0, 0.0),
            Color::cmyk_pct(0.0, 10.0, 0.0, 0.0)
        );
        assert_eq!(
            with_channel(c, 0, 1.0),
            Color::cmyk_pct(100.0, 10.0, 0.0, 0.0)
        );
        assert_eq!(
            with_channel(c, 3, 0.5),
            Color::cmyk_pct(40.0, 10.0, 0.0, 50.0)
        );
        let rgb = Color::rgb8(10, 200, 30);
        assert_eq!(with_channel(rgb, 0, 1.0), Color::rgb8(255, 200, 30));
        assert_eq!(with_channel(rgb, 2, 0.0), Color::rgb8(10, 200, 0));
        // A hue rail runs through the spectrum.
        let h = Color::Hsb {
            h: 10.0,
            s: 1.0,
            b: 1.0,
        };
        assert!(
            matches!(with_channel(h, 0, 240.0), Color::Hsb { h, .. } if (h - 240.0).abs() < 1e-6)
        );
        // Drawn: the rail of a slider is a gradient mesh as wide as the
        // slider, not a plain groove.
        let ctx = crate::theme::ui_context();
        let mut m = rgb;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(320.0, 400.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            sliders(ui, &mut m);
        });
        out.textures_delta.clear();
        fn meshes(s: &egui::Shape, out: &mut Vec<egui::Rect>) {
            match s {
                egui::Shape::Mesh(m) => out.push(m.calc_bounds()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| meshes(s, out)),
                _ => {}
            }
        }
        let mut rails = Vec::new();
        out.shapes.iter().for_each(|c| meshes(&c.shape, &mut rails));
        let wide: Vec<_> = rails.iter().filter(|r| r.width() >= 60.0).collect();
        assert_eq!(wide.len(), 3, "{rails:?}");
    }

    /// A palette colour clicked: a spot colour at the tint of its ink, a
    /// process colour as it is whatever the tint (the tint set for an
    /// ink does not lighten plain swatches).
    #[test]
    fn a_palette_colour_is_taken_at_the_tint() {
        let c = Color::cmyk_pct(100.0, 0.0, 50.0, 20.0);
        assert_eq!(at_tint(c, 1.0), c);
        assert_eq!(at_tint(c, 0.5), c);
        let rgb = Color::rgb8(0, 0, 255);
        assert_eq!(at_tint(rgb, 0.3), rgb);
        let spot = Color::Spot {
            ink: 7,
            tint: 1.0,
            c: 0.0,
            m: 0.2,
            y: 0.8,
            k: 0.1,
        };
        assert!(
            matches!(at_tint(spot, 0.3), Color::Spot { ink: 7, tint, .. } if (tint - 0.3).abs() < 1e-6)
        );
    }

    /// The gear's Reset panel: views, model, tint, lock and hue back to
    /// their defaults, the new colour converted to CMYK.
    #[test]
    fn reset_panel_puts_the_defaults_back() {
        let mut app = App::headless();
        app.color_docker.shown = [false, false, true];
        app.color_docker.tint = 30.0;
        app.color_docker.lock = true;
        app.color_model = 0;
        app.mixer_color = Color::rgb8(255, 0, 0);
        reset(&mut app);
        let d = ColorDockerState::default();
        assert_eq!(app.color_docker.shown, d.shown);
        assert_eq!(app.color_docker.tint, d.tint);
        assert!(!app.color_docker.lock);
        assert_eq!(MODELS[app.color_model], "CMYK");
        assert_eq!(app.mixer_color.model_name(), "CMYK");
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

    /// The texts of one frame of the whole panel.
    fn panel_texts(app: &mut App) -> Vec<String> {
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(320.0, 1400.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| color(app, ui));
        out.textures_delta.clear();
        let mut texts = Vec::new();
        fn walk(s: &egui::Shape, texts: &mut Vec<String>) {
            match s {
                egui::Shape::Text(t) => texts.push(t.galley.text().to_string()),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts)),
                _ => {}
            }
        }
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
        texts
    }

    /// One Tint control in the panel: under the palette's swatches while
    /// the palette view shows, else under the spot inks; never both.
    #[test]
    fn the_panel_shows_one_tint_control() {
        let mut app = App::headless();
        add_ink(
            &mut app,
            "Gold".into(),
            Color::cmyk_pct(0.0, 20.0, 80.0, 10.0),
        );
        let tint = tr("docker.tint");
        for shown in [[true, true, true], [true, true, false]] {
            app.color_docker.shown = shown;
            let n = panel_texts(&mut app).iter().filter(|t| **t == tint).count();
            assert_eq!(n, 1, "{shown:?}");
        }
    }

    /// With nothing selected the outline swatch shows the outline new
    /// objects get, and none once the defaults have no outline.
    #[test]
    fn the_outline_swatch_shows_the_default_outline() {
        let mut app = App::headless();
        app.select(Vec::new());
        app.current_colors.1 = Color::rgb8(1, 2, 3);
        let default = app.default_stroke().map(|s| s.color);
        assert!(default.is_some());
        assert_eq!(selected_outline(&app), default);
        let mut defaults = app.doc().defaults.clone();
        defaults.graphic.stroke = None;
        app.run(Command::SetObjectDefaults { defaults });
        assert_eq!(selected_outline(&app), None);
    }
}
