//! Preflight: a check of the drawing before it is printed or sent to a
//! print shop (File > Preflight, and a summary in the Print dialog). Each
//! check can be turned off and some have a threshold (preferences,
//! `preflight`). The checks read the document only; the dialog lists what
//! they find and selects the object an issue is about.

use serde::{Deserialize, Serialize};
use tracedraw_core::{
    document::{PrimaryColorMode, ShapeKind},
    geometry::{Affine, Point, Rect},
    Color, Document, Fill, Shape, ShapeId,
};

/// What a check looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Check {
    /// Text objects with no text.
    EmptyText,
    /// Paragraph text that does not fit its frame (the last frame of a
    /// chain).
    TextOverflow,
    /// Text in a font that is not installed.
    MissingFont,
    /// Text smaller than the minimum size, as printed.
    SmallText,
    /// Small black text made of several inks: it shows colour fringes when
    /// the plates do not register exactly.
    RichBlackText,
    /// RGB colours (also HSB, HSL, YIQ) in a CMYK drawing.
    RgbColor,
    /// Objects entirely outside the page and its bleed.
    OffPage,
    /// Bitmaps below the minimum resolution, as placed.
    LowResBitmap,
    /// Bitmaps above the maximum resolution (they make big files for no
    /// gain).
    HighResBitmap,
    /// Outlines thinner than the minimum width (hairlines too).
    ThinOutline,
    /// White fills or outlines set to overprint: they vanish when printed.
    WhiteOverprint,
    /// Pages with no objects.
    EmptyPage,
    /// Objects on hidden or non-printing layers: they will not print.
    HiddenObjects,
    /// Text in a font whose licence forbids embedding: PDF files get its
    /// text as curves.
    RestrictedFont,
}

/// How serious an issue is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Likely to spoil the print.
    Warning,
    /// Worth knowing.
    Info,
}

impl Check {
    pub const ALL: [Check; 14] = [
        Check::EmptyText,
        Check::TextOverflow,
        Check::MissingFont,
        Check::SmallText,
        Check::RichBlackText,
        Check::RgbColor,
        Check::OffPage,
        Check::LowResBitmap,
        Check::HighResBitmap,
        Check::ThinOutline,
        Check::WhiteOverprint,
        Check::EmptyPage,
        Check::HiddenObjects,
        Check::RestrictedFont,
    ];

    /// Stable id, used in the preferences and the i18n keys.
    pub fn id(self) -> &'static str {
        match self {
            Check::EmptyText => "empty_text",
            Check::TextOverflow => "text_overflow",
            Check::MissingFont => "missing_font",
            Check::SmallText => "small_text",
            Check::RichBlackText => "rich_black_text",
            Check::RgbColor => "rgb_color",
            Check::OffPage => "off_page",
            Check::LowResBitmap => "low_res_bitmap",
            Check::HighResBitmap => "high_res_bitmap",
            Check::ThinOutline => "thin_outline",
            Check::WhiteOverprint => "white_overprint",
            Check::EmptyPage => "empty_page",
            Check::HiddenObjects => "hidden_objects",
            Check::RestrictedFont => "restricted_font",
        }
    }

    /// The check's name (i18n key).
    pub fn key(self) -> String {
        format!("preflight.check.{}", self.id())
    }

    pub fn severity(self) -> Severity {
        match self {
            Check::EmptyPage
            | Check::HiddenObjects
            | Check::HighResBitmap
            | Check::EmptyText
            | Check::RestrictedFont => Severity::Info,
            _ => Severity::Warning,
        }
    }
}

/// The preflight preferences: the checks turned off and the thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PreflightPrefs {
    /// Ids of the checks that do not run.
    pub off: Vec<String>,
    /// Bitmaps below this many dots per inch, as placed, are reported.
    pub min_dpi: f64,
    /// Bitmaps above this many dots per inch are reported.
    pub max_dpi: f64,
    /// Text smaller than this many points, as printed, is reported.
    pub min_text_pt: f64,
    /// Black text below this size is checked for extra inks.
    pub rich_black_below_pt: f64,
    /// Outlines thinner than this, in mm, are reported.
    pub min_outline_mm: f64,
}

impl Default for PreflightPrefs {
    fn default() -> Self {
        PreflightPrefs {
            off: Vec::new(),
            min_dpi: 200.0,
            max_dpi: 1200.0,
            min_text_pt: 5.0,
            rich_black_below_pt: 12.0,
            min_outline_mm: 0.1,
        }
    }
}

impl PreflightPrefs {
    pub fn runs(&self, c: Check) -> bool {
        !self.off.iter().any(|id| id == c.id())
    }

    pub fn set_runs(&mut self, c: Check, on: bool) {
        self.off.retain(|id| id != c.id());
        if !on {
            self.off.push(c.id().to_string());
        }
    }
}

/// One thing the preflight found.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub check: Check,
    /// The page (0-based); `None` for master layers.
    pub page: Option<usize>,
    /// The object to select: the top-level object the issue is in.
    pub shape: Option<ShapeId>,
    /// What it is about: a font, a resolution, a size, a layer.
    pub detail: String,
}

/// Run the checks over `doc`. `missing_fonts` are the font families used
/// in the drawing that are not installed.
pub fn run(doc: &Document, missing_fonts: &[String], prefs: &PreflightPrefs) -> Vec<Issue> {
    let mut out = Vec::new();
    let bleed = doc.metadata.bleed.max(0.0);
    let cmyk = doc.metadata.primary_color_mode == PrimaryColorMode::Cmyk;
    for (pi, page) in doc.pages.iter().enumerate() {
        let area = Rect::new(
            -bleed,
            -bleed,
            page.size.width + bleed,
            page.size.height + bleed,
        );
        let mut objects = 0usize;
        for layer in &page.layers {
            let hidden = !layer.visible || !layer.printable;
            if hidden && !layer.shapes.is_empty() && prefs.runs(Check::HiddenObjects) {
                out.push(Issue {
                    check: Check::HiddenObjects,
                    page: Some(pi),
                    shape: None,
                    detail: format!("{} ({})", layer.name, layer.shapes.len()),
                });
            }
            objects += layer.shapes.len();
            if hidden {
                continue;
            }
            for s in &layer.shapes {
                let ctx = Ctx {
                    page: Some(pi),
                    top: s.id,
                    doc,
                    prefs,
                    missing_fonts,
                    cmyk,
                };
                if prefs.runs(Check::OffPage) && off_page(s, area) {
                    out.push(Issue {
                        check: Check::OffPage,
                        page: Some(pi),
                        shape: Some(s.id),
                        detail: String::new(),
                    });
                }
                visit(&ctx, s, Affine::IDENTITY, &mut out);
            }
        }
        if objects == 0 && prefs.runs(Check::EmptyPage) {
            out.push(Issue {
                check: Check::EmptyPage,
                page: Some(pi),
                shape: None,
                detail: page.name.clone(),
            });
        }
    }
    // Master layers show on many pages: they are checked once.
    for layer in &doc.master {
        if !layer.visible || !layer.printable {
            continue;
        }
        for s in &layer.shapes {
            let ctx = Ctx {
                page: None,
                top: s.id,
                doc,
                prefs,
                missing_fonts,
                cmyk,
            };
            visit(&ctx, s, Affine::IDENTITY, &mut out);
        }
    }
    out
}

struct Ctx<'a> {
    page: Option<usize>,
    top: ShapeId,
    doc: &'a Document,
    prefs: &'a PreflightPrefs,
    missing_fonts: &'a [String],
    cmyk: bool,
}

impl Ctx<'_> {
    fn issue(&self, out: &mut Vec<Issue>, check: Check, detail: String) {
        if !self.prefs.runs(check) {
            return;
        }
        // One issue of a kind per object and detail.
        let dup = out.iter().any(|i| {
            i.check == check
                && i.shape == Some(self.top)
                && i.page == self.page
                && i.detail == detail
        });
        if !dup {
            out.push(Issue {
                check,
                page: self.page,
                shape: Some(self.top),
                detail,
            });
        }
    }
}

/// True when the object's bounds miss the page and its bleed entirely.
fn off_page(s: &Shape, area: Rect) -> bool {
    let b = s.bounds();
    if ![b.x0, b.y0, b.x1, b.y1].iter().all(|v| v.is_finite()) {
        return false;
    }
    b.x1 < area.x0 || b.x0 > area.x1 || b.y1 < area.y0 || b.y0 > area.y1
}

/// How much an affine map scales lengths (the square root of its area
/// factor).
fn scale_of(t: Affine) -> f64 {
    let [a, b, c, d, _, _] = t.as_coeffs();
    (a * d - b * c).abs().sqrt()
}

/// The colours a fill uses.
fn fill_colors(f: &Fill) -> Vec<Color> {
    match f {
        Fill::None | Fill::Texture(_) => Vec::new(),
        Fill::Solid(c) => vec![*c],
        Fill::Fountain(g) => g.stops.iter().map(|s| s.color).collect(),
        Fill::Mesh(m) => m.nodes.iter().map(|n| n.color).collect(),
        Fill::Procedural(p) => std::iter::once(p.foreground).chain(p.background).collect(),
        Fill::Pattern(p) => match p {
            tracedraw_core::Pattern::TwoColor { front, back, .. } => vec![*front, *back],
            _ => Vec::new(),
        },
    }
}

fn is_rgb_like(c: Color) -> bool {
    matches!(
        c,
        Color::Rgb { .. } | Color::Hsb { .. } | Color::Hsl { .. } | Color::Yiq { .. }
    )
}

fn is_white(c: Color) -> bool {
    !matches!(c, Color::Registration) && c.to_rgb8().iter().all(|v| *v >= 254)
}

fn is_rich_black(c: Color) -> bool {
    matches!(c, Color::Cmyk { c, m, y, k } if k >= 0.99 && c + m + y > 0.01)
}

/// Check one object and, for groups and frames, what is inside it.
fn visit(ctx: &Ctx, s: &Shape, parent: Affine, out: &mut Vec<Issue>) {
    let t = parent * s.transform;
    let mut colors = fill_colors(&s.fill);
    if let Some(st) = &s.stroke {
        colors.push(st.color);
        if st.width < ctx.prefs.min_outline_mm {
            ctx.issue(out, Check::ThinOutline, format!("{:.3} mm", st.width));
        }
        if s.overprint_outline && is_white(st.color) {
            ctx.issue(out, Check::WhiteOverprint, String::new());
        }
    }
    if s.overprint_fill && fill_colors(&s.fill).into_iter().any(is_white) {
        ctx.issue(out, Check::WhiteOverprint, String::new());
    }
    match &s.kind {
        ShapeKind::Group { children } => {
            for c in children {
                visit(ctx, c, t, out);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            visit(ctx, frame, t, out);
            for c in contents {
                visit(ctx, c, t, out);
            }
        }
        ShapeKind::Text {
            spans,
            frame,
            on_path,
            ..
        } => {
            text_checks(ctx, s, t, spans, frame.is_some() && on_path.is_none(), out);
            for span in spans {
                if let Some(f) = &span.fill {
                    colors.extend(fill_colors(f));
                }
            }
        }
        ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            ..
        } => bitmap_checks(ctx, t, *rect, *width_px, *height_px, out),
        _ => {}
    }
    if ctx.cmyk && colors.iter().any(|c| is_rgb_like(*c)) {
        ctx.issue(out, Check::RgbColor, String::new());
    }
}

/// The font that draws `family` in this style forbids embedding (its
/// licence bits), so PDF files get its text as curves. Remembered per
/// face until fonts are added.
pub fn font_restricted(family: &str, bold: bool, italic: bool) -> bool {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type Seen = (u64, HashMap<(String, bool, bool), bool>);
    static SEEN: Mutex<Option<Seen>> = Mutex::new(None);
    let fonts = tracedraw_text::fonts();
    let generation = fonts.generation();
    let key = (family.to_string(), bold, italic);
    if let Ok(seen) = SEEN.lock() {
        if let Some((g, map)) = seen.as_ref() {
            if *g == generation {
                if let Some(v) = map.get(&key) {
                    return *v;
                }
            }
        }
    }
    let restricted = fonts
        .embedding_bits(family, bold, italic)
        .is_some_and(|bits| {
            tracedraw_io::pdf::licence_of(bits) == tracedraw_io::pdf::FontLicence::Restricted
        });
    if let Ok(mut seen) = SEEN.lock() {
        let fresh = !matches!(seen.as_ref(), Some((g, _)) if *g == generation);
        if fresh {
            *seen = Some((generation, HashMap::new()));
        }
        if let Some((_, map)) = seen.as_mut() {
            map.insert(key, restricted);
        }
    }
    restricted
}

fn text_checks(
    ctx: &Ctx,
    s: &Shape,
    t: Affine,
    spans: &[tracedraw_core::TextSpan],
    paragraph: bool,
    out: &mut Vec<Issue>,
) {
    let text: String = spans.iter().map(|sp| sp.text.as_str()).collect();
    if text.trim().is_empty() {
        ctx.issue(out, Check::EmptyText, String::new());
        return;
    }
    let scale = scale_of(t);
    for sp in spans.iter().filter(|sp| !sp.text.trim().is_empty()) {
        let size = sp.size_pt * scale;
        if size < ctx.prefs.min_text_pt {
            ctx.issue(out, Check::SmallText, format!("{size:.1} pt"));
        }
        if ctx.missing_fonts.iter().any(|f| f == &sp.font_family) {
            ctx.issue(out, Check::MissingFont, sp.font_family.clone());
        } else if ctx.prefs.runs(Check::RestrictedFont)
            && font_restricted(&sp.font_family, sp.bold, sp.italic)
        {
            ctx.issue(out, Check::RestrictedFont, sp.font_family.clone());
        }
        let fill = sp.fill.as_ref().unwrap_or(&s.fill);
        if size < ctx.prefs.rich_black_below_pt && fill_colors(fill).into_iter().any(is_rich_black)
        {
            ctx.issue(out, Check::RichBlackText, format!("{size:.1} pt"));
        }
    }
    // Overflow counts at the end of a chain of linked frames only.
    if paragraph && ctx.prefs.runs(Check::TextOverflow) && next_frame(ctx.doc, s).is_none() {
        let overflows = crate::text_editing::layout_of_text_shape(s, Affine::IDENTITY)
            .map(|(l, _)| l.overflow)
            .unwrap_or(false);
        if overflows {
            ctx.issue(out, Check::TextOverflow, String::new());
        }
    }
}

/// The frame a paragraph frame flows into, when it is linked to one that
/// is still there.
fn next_frame(doc: &Document, s: &Shape) -> Option<ShapeId> {
    let next = s
        .data
        .iter()
        .find(|(k, _)| k == "text.next")
        .and_then(|(_, v)| v.parse::<u64>().ok())
        .map(ShapeId)?;
    doc.find_shape(next).map(|_| next)
}

fn bitmap_checks(ctx: &Ctx, t: Affine, rect: Rect, w: u32, h: u32, out: &mut Vec<Issue>) {
    // The placed size of the bitmap's edges, in inches.
    let p = |x: f64, y: f64| t * Point::new(x, y);
    let across = (p(rect.x1, rect.y0) - p(rect.x0, rect.y0)).hypot() / 25.4;
    let down = (p(rect.x0, rect.y1) - p(rect.x0, rect.y0)).hypot() / 25.4;
    if !(across > 1e-9 && down > 1e-9) {
        return;
    }
    let dpi = (f64::from(w) / across).min(f64::from(h) / down);
    if dpi < ctx.prefs.min_dpi {
        ctx.issue(out, Check::LowResBitmap, format!("{dpi:.0} dpi"));
    } else if dpi > ctx.prefs.max_dpi {
        ctx.issue(out, Check::HighResBitmap, format!("{dpi:.0} dpi"));
    }
}

/// How many issues of each severity.
pub fn counts(issues: &[Issue]) -> (usize, usize) {
    let warnings = issues
        .iter()
        .filter(|i| i.check.severity() == Severity::Warning)
        .count();
    (warnings, issues.len() - warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{
        document::{Layer, TextSpan},
        geometry::Size,
        Stroke,
    };

    fn doc() -> Document {
        Document::new("t", Size::new(100.0, 100.0))
    }

    fn add(doc: &mut Document, kind: ShapeKind) -> ShapeId {
        let id = doc.ids_mut().shape();
        let mut s = Shape::new(id, kind);
        s.stroke = None;
        s.fill = Fill::Solid(Color::Cmyk {
            c: 0.0,
            m: 0.0,
            y: 0.0,
            k: 1.0,
        });
        doc.pages[0].layers[0].shapes.push(s);
        id
    }

    fn rect(x: f64, y: f64, w: f64, h: f64) -> ShapeKind {
        ShapeKind::Rect {
            rect: Rect::new(x, y, x + w, y + h),
            radius: 0.0,
            corners: None,
        }
    }

    fn text(t: &str, size: f64) -> ShapeKind {
        ShapeKind::Text {
            spans: vec![TextSpan::new(t, "Inter", size)],
            origin: Point::new(10.0, 10.0),
            frame: None,
            align: Default::default(),
            para: Default::default(),
            on_path: None,
        }
    }

    fn checks(issues: &[Issue]) -> Vec<Check> {
        issues.iter().map(|i| i.check).collect()
    }

    #[test]
    fn a_clean_drawing_has_no_issues() {
        let mut d = doc();
        add(&mut d, rect(10.0, 10.0, 20.0, 20.0));
        add(&mut d, text("Hello", 12.0));
        assert_eq!(run(&d, &[], &PreflightPrefs::default()), Vec::new());
    }

    #[test]
    fn text_problems_are_found() {
        let mut d = doc();
        let empty = add(&mut d, text("  ", 12.0));
        let small = add(&mut d, text("tiny", 4.0));
        let font = add(&mut d, text("font", 12.0));
        let issues = run(&d, &["Inter".to_string()], &PreflightPrefs::default());
        let of = |id| {
            issues
                .iter()
                .filter(|i| i.shape == Some(id))
                .map(|i| i.check)
                .collect::<Vec<_>>()
        };
        assert_eq!(of(empty), vec![Check::EmptyText]);
        assert!(of(small).contains(&Check::SmallText));
        assert!(of(font).contains(&Check::MissingFont));
        // Small black text in four inks is rich black.
        let mut d = doc();
        let id = add(&mut d, text("rich", 8.0));
        if let Ok(s) = d.shape_mut(id) {
            s.fill = Fill::Solid(Color::Cmyk {
                c: 0.6,
                m: 0.4,
                y: 0.4,
                k: 1.0,
            });
        }
        let issues = run(&d, &[], &PreflightPrefs::default());
        assert_eq!(checks(&issues), vec![Check::RichBlackText]);
        assert_eq!(issues[0].detail, "8.0 pt");
        // Text scaled down below the minimum counts as small.
        if let Ok(s) = d.shape_mut(id) {
            s.fill = Fill::Solid(Color::Cmyk {
                c: 0.0,
                m: 0.0,
                y: 0.0,
                k: 1.0,
            });
            s.transform = Affine::scale(0.5);
        }
        assert_eq!(
            checks(&run(&d, &[], &PreflightPrefs::default())),
            vec![Check::SmallText]
        );
    }

    #[test]
    fn colours_outlines_and_overprints_are_checked() {
        let mut d = doc();
        let rgb = add(&mut d, rect(10.0, 10.0, 10.0, 10.0));
        let thin = add(&mut d, rect(30.0, 10.0, 10.0, 10.0));
        let white = add(&mut d, rect(50.0, 10.0, 10.0, 10.0));
        if let Ok(s) = d.shape_mut(rgb) {
            s.fill = Fill::Solid(Color::Rgb {
                r: 1.0,
                g: 0.0,
                b: 0.0,
            });
        }
        if let Ok(s) = d.shape_mut(thin) {
            s.stroke = Some(Stroke::hairline(Color::BLACK));
        }
        if let Ok(s) = d.shape_mut(white) {
            s.fill = Fill::Solid(Color::Cmyk {
                c: 0.0,
                m: 0.0,
                y: 0.0,
                k: 0.0,
            });
            s.overprint_fill = true;
        }
        let issues = run(&d, &[], &PreflightPrefs::default());
        let find = |id| issues.iter().find(|i| i.shape == Some(id)).map(|i| i.check);
        assert_eq!(find(rgb), Some(Check::RgbColor));
        assert_eq!(find(thin), Some(Check::ThinOutline));
        assert_eq!(find(white), Some(Check::WhiteOverprint));
        // RGB is fine in an RGB drawing.
        d.metadata.primary_color_mode = PrimaryColorMode::Rgb;
        let issues = run(&d, &[], &PreflightPrefs::default());
        assert!(!checks(&issues).contains(&Check::RgbColor));
        // A check that is off does not run.
        let mut prefs = PreflightPrefs::default();
        prefs.set_runs(Check::ThinOutline, false);
        assert!(!prefs.runs(Check::ThinOutline));
        assert!(!checks(&run(&d, &[], &prefs)).contains(&Check::ThinOutline));
        prefs.set_runs(Check::ThinOutline, true);
        assert!(prefs.off.is_empty());
    }

    #[test]
    fn pages_layers_and_placement_are_checked() {
        let mut d = doc();
        // Off the page, beyond the bleed.
        d.metadata.bleed = 3.0;
        let away = add(&mut d, rect(150.0, 10.0, 10.0, 10.0));
        // In the bleed: fine.
        add(&mut d, rect(-2.5, 10.0, 2.0, 10.0));
        let mut hidden = Layer::new(d.ids_mut().layer(), "Notes");
        hidden.printable = false;
        let id = d.ids_mut().shape();
        hidden
            .shapes
            .push(Shape::new(id, rect(10.0, 10.0, 5.0, 5.0)));
        d.pages[0].layers.push(hidden);
        // A second, empty page.
        let page = tracedraw_core::document::Page {
            id: d.ids_mut().page(),
            name: "Back".into(),
            size: Size::new(100.0, 100.0),
            layers: Vec::new(),
            guides: Vec::new(),
            background: None,
        };
        d.pages.push(page);
        let issues = run(&d, &[], &PreflightPrefs::default());
        assert!(issues
            .iter()
            .any(|i| i.check == Check::OffPage && i.shape == Some(away)));
        assert_eq!(
            issues.iter().filter(|i| i.check == Check::OffPage).count(),
            1
        );
        let layer = issues.iter().find(|i| i.check == Check::HiddenObjects);
        assert_eq!(layer.map(|i| i.detail.as_str()), Some("Notes (1)"));
        let empty = issues.iter().find(|i| i.check == Check::EmptyPage);
        assert_eq!(empty.and_then(|i| i.page), Some(1));
        let (warnings, info) = counts(&issues);
        assert_eq!((warnings, info), (1, 2));
    }

    #[test]
    fn bitmap_resolution_is_measured_as_placed() {
        let mut d = doc();
        // 100 x 100 px on 25.4 mm: 100 dpi.
        let low = add(
            &mut d,
            ShapeKind::Bitmap {
                rect: Rect::new(0.0, 0.0, 25.4, 25.4),
                width_px: 100,
                height_px: 100,
                png: Vec::new().into(),
                fx: None,
            },
        );
        let issues = run(&d, &[], &PreflightPrefs::default());
        let i = issues.iter().find(|i| i.shape == Some(low));
        assert_eq!(
            i.map(|i| (i.check, i.detail.as_str())),
            Some((Check::LowResBitmap, "100 dpi"))
        );
        // Scaled to a fifth: 500 dpi, fine.
        if let Ok(s) = d.shape_mut(low) {
            s.transform = Affine::scale(0.2);
        }
        assert!(run(&d, &[], &PreflightPrefs::default()).is_empty());
        // A tenth of that size: 5000 dpi, too much.
        if let Ok(s) = d.shape_mut(low) {
            s.transform = Affine::scale(0.02);
        }
        let issues = run(&d, &[], &PreflightPrefs::default());
        assert_eq!(checks(&issues), vec![Check::HighResBitmap]);
    }

    #[test]
    fn every_check_has_a_name_and_a_hint() {
        for c in Check::ALL {
            let name = crate::i18n::tr(&c.key());
            assert!(!name.starts_with("preflight."), "{}", c.id());
            let hint = crate::i18n::tr(&format!("preflight.hint.{}", c.id()));
            assert!(!hint.starts_with("preflight."), "{}", c.id());
        }
    }
}
