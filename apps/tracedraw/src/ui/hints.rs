//! The Hints panel: a Home page
//! listing topics, one page per topic (an introduction and the tools that
//! serve it), and one page per tool (what it does, how to use it, and a
//! link to its help topic). Choosing a tool shows its page; a bar at the
//! bottom goes home, back and forward.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::{self, Tokens};
use crate::tools::Tool;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

/// A page of the docker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintPage {
    Home,
    Topic(Topic),
    Tool(Tool),
}

/// The Home page's topics, in its order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    Lines,
    ConnectorLines,
    DimensionLines,
    Shapes,
    Select,
    MoveScale,
    RotateSkew,
    ShapeObjects,
    Effects,
    Outline,
    Fill,
    Text,
    Help,
}

impl Topic {
    pub const ALL: [Topic; 13] = [
        Topic::Lines,
        Topic::ConnectorLines,
        Topic::DimensionLines,
        Topic::Shapes,
        Topic::Select,
        Topic::MoveScale,
        Topic::RotateSkew,
        Topic::ShapeObjects,
        Topic::Effects,
        Topic::Outline,
        Topic::Fill,
        Topic::Text,
        Topic::Help,
    ];

    fn id(self) -> &'static str {
        match self {
            Topic::Lines => "lines",
            Topic::ConnectorLines => "connector_lines",
            Topic::DimensionLines => "dimension_lines",
            Topic::Shapes => "shapes",
            Topic::Select => "select",
            Topic::MoveScale => "move_scale",
            Topic::RotateSkew => "rotate_skew",
            Topic::ShapeObjects => "shape_objects",
            Topic::Effects => "effects",
            Topic::Outline => "outline",
            Topic::Fill => "fill",
            Topic::Text => "text",
            Topic::Help => "help",
        }
    }

    fn title(self) -> String {
        let key = match self {
            Topic::Select => "hint.topic_select".to_string(),
            Topic::MoveScale => "hint.topic_move_scale".to_string(),
            Topic::RotateSkew => "hint.topic_rotate_skew".to_string(),
            Topic::ShapeObjects => "hint.topic_shape_objects".to_string(),
            Topic::Effects => "hint.topic_effects".to_string(),
            Topic::Help => "hint.topic_help".to_string(),
            t => format!("hint.topic_{}", t.id()),
        };
        tr(&key)
    }

    /// The tools the topic presents.
    pub fn tools(self) -> &'static [Tool] {
        match self {
            Topic::Lines => &[
                Tool::Freehand,
                Tool::TwoPointLine,
                Tool::Bezier,
                Tool::Pen,
                Tool::BSpline,
                Tool::Polyline,
                Tool::ThreePointCurve,
                Tool::BrushStrokes,
                Tool::ShapeRecognition,
                Tool::Sketch,
            ],
            Topic::ConnectorLines => &[
                Tool::Connector,
                Tool::RightAngleConnector,
                Tool::RoundedConnector,
                Tool::AnchorEditing,
            ],
            Topic::DimensionLines => &[
                Tool::ParallelDimension,
                Tool::HorizontalVerticalDimension,
                Tool::AngularDimension,
                Tool::SegmentDimension,
                Tool::Callout,
            ],
            Topic::Shapes => &[
                Tool::Rectangle,
                Tool::ThreePointRectangle,
                Tool::Ellipse,
                Tool::ThreePointEllipse,
                Tool::Polygon,
                Tool::Star,
                Tool::Spiral,
                Tool::CommonShapes,
                Tool::GraphPaper,
                Tool::ActionLines,
            ],
            Topic::Select => &[Tool::Pick, Tool::FreeformPick],
            Topic::MoveScale => &[Tool::Pick, Tool::FreeTransform],
            Topic::RotateSkew => &[Tool::Pick, Tool::FreeTransform],
            Topic::ShapeObjects => &[
                Tool::Shape,
                Tool::Smooth,
                Tool::Smear,
                Tool::Twirl,
                Tool::AttractRepel,
                Tool::Smudge,
                Tool::Roughen,
                Tool::Crop,
                Tool::Knife,
                Tool::SegmentDelete,
                Tool::Eraser,
            ],
            Topic::Effects => &[
                Tool::DropShadow,
                Tool::Contour,
                Tool::Blend,
                Tool::Distort,
                Tool::Envelope,
                Tool::Extrude,
                Tool::BlockShadow,
                Tool::Transparency,
            ],
            Topic::Outline => &[
                Tool::OutlinePen,
                Tool::OutlineColor,
                Tool::AttributesEyedropper,
            ],
            Topic::Fill => &[
                Tool::InteractiveFill,
                Tool::AreaFill,
                Tool::MeshFill,
                Tool::ColorEyedropper,
            ],
            Topic::Text => &[Tool::Text, Tool::Table],
            Topic::Help => &[],
        }
    }
}

/// Where the docker is and where it has been.
#[derive(Debug, Clone)]
pub struct HintsState {
    history: Vec<HintPage>,
    pos: usize,
    /// The tool the docker last followed: a different active tool shows
    /// that tool's page.
    followed: Tool,
}

impl Default for HintsState {
    fn default() -> Self {
        HintsState {
            history: vec![HintPage::Home],
            pos: 0,
            followed: Tool::Pick,
        }
    }
}

impl HintsState {
    pub fn current(&self) -> HintPage {
        self.history
            .get(self.pos)
            .copied()
            .unwrap_or(HintPage::Home)
    }

    /// Show `page`, dropping the pages ahead of the current one.
    pub fn go(&mut self, page: HintPage) {
        if self.current() == page {
            return;
        }
        self.history.truncate(self.pos + 1);
        self.history.push(page);
        self.pos = self.history.len() - 1;
    }

    pub fn can_back(&self) -> bool {
        self.pos > 0
    }

    pub fn can_forward(&self) -> bool {
        self.pos + 1 < self.history.len()
    }

    pub fn back(&mut self) {
        if self.can_back() {
            self.pos -= 1;
        }
    }

    pub fn forward(&mut self) {
        if self.can_forward() {
            self.pos += 1;
        }
    }

    /// Follow the active tool: a newly chosen tool shows its page.
    pub fn follow(&mut self, tool: Tool) {
        if tool != self.followed {
            self.followed = tool;
            self.go(HintPage::Tool(tool));
        }
    }
}

const DOCS: &str = "https://github.com/igtoth/tracedraw/blob/main/docs/";

/// The help page of a tool (the behaviour notes in the repository).
fn help_page(tool: Tool) -> &'static str {
    match tool {
        Tool::Pick | Tool::FreeformPick => "pick-tool.md",
        Tool::FreeTransform => "free-transform.md",
        Tool::Shape
        | Tool::Smooth
        | Tool::Smear
        | Tool::Twirl
        | Tool::AttractRepel
        | Tool::Smudge
        | Tool::Roughen => "shape-tool.md",
        Tool::Crop | Tool::Knife | Tool::SegmentDelete | Tool::Eraser => "crop-knife-eraser.md",
        Tool::Zoom | Tool::Pan => "zoom-and-pan.md",
        Tool::BrushStrokes => "brush-strokes.md",
        Tool::Freehand
        | Tool::TwoPointLine
        | Tool::Bezier
        | Tool::Pen
        | Tool::BSpline
        | Tool::Polyline
        | Tool::ThreePointCurve
        | Tool::ShapeRecognition
        | Tool::Sketch => "curve-tools.md",
        Tool::Rectangle
        | Tool::ThreePointRectangle
        | Tool::Ellipse
        | Tool::ThreePointEllipse
        | Tool::Polygon
        | Tool::Star
        | Tool::Spiral
        | Tool::CommonShapes
        | Tool::ActionLines
        | Tool::GraphPaper => "shape-tools.md",
        Tool::Text => "text.md",
        Tool::Table => "table.md",
        Tool::ParallelDimension
        | Tool::HorizontalVerticalDimension
        | Tool::AngularDimension
        | Tool::SegmentDimension
        | Tool::Callout => "dimensions.md",
        Tool::Connector
        | Tool::RightAngleConnector
        | Tool::RoundedConnector
        | Tool::AnchorEditing => "connectors-and-anchors.md",
        Tool::DropShadow => "drop-shadow.md",
        Tool::Contour => "contour.md",
        Tool::Blend => "blend.md",
        Tool::Distort => "distort.md",
        Tool::Envelope => "envelope.md",
        Tool::Extrude => "extrude.md",
        Tool::BlockShadow => "block-shadow.md",
        Tool::Transparency => "transparency.md",
        Tool::ColorEyedropper | Tool::AttributesEyedropper => "eyedroppers.md",
        Tool::InteractiveFill => "gradient-fill.md",
        Tool::AreaFill => "area-fill.md",
        Tool::MeshFill => "mesh-fill.md",
        Tool::OutlinePen | Tool::OutlineColor => "outline-pen.md",
    }
}

/// The docker: the page in a scroll area and the navigation bar under it.
pub fn hints_docker(app: &mut App, ui: &mut Ui) {
    app.hints.follow(app.tool);
    let bar_h = 30.0;
    let content_h = (ui.available_height() - bar_h - 6.0).max(40.0);
    let mut go = None;
    egui::ScrollArea::vertical()
        .max_height(content_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            go = match app.hints.current() {
                HintPage::Home => home(app, ui),
                HintPage::Topic(t) => topic(app, ui, t),
                HintPage::Tool(t) => tool_page(app, ui, t),
            };
        });
    if let Some(page) = go {
        app.hints.go(page);
    }
    nav_bar(app, ui);
}

fn title(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text).font(theme::bold(18.0)));
    ui.add_space(6.0);
}

/// The panel's text: 14 px on 21 px lines.
const TEXT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 21.0;

/// Text where `**...**` is bold, wrapped to the docker's width.
fn rich(ui: &mut Ui, text: &str) {
    let job = rich_job(text, ui.available_width(), TEXT_SIZE);
    ui.label(job);
}

fn rich_job(text: &str, width: f32, size: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat {
        font_id: FontId::proportional(size),
        color: Tokens::TEXT,
        line_height: Some(LINE_HEIGHT),
        ..Default::default()
    };
    let strong = TextFormat {
        font_id: theme::bold(size),
        color: Tokens::TEXT,
        line_height: Some(LINE_HEIGHT),
        ..Default::default()
    };
    for (i, part) in text.split("**").enumerate() {
        if !part.is_empty() {
            job.append(
                part,
                0.0,
                if i % 2 == 1 {
                    strong.clone()
                } else {
                    plain.clone()
                },
            );
        }
    }
    job.wrap.max_width = width;
    job
}

fn link(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .color(Tokens::ACCENT)
                .size(TEXT_SIZE),
        )
        .sense(Sense::click()),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn bullet_row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(egui::RichText::new("•").size(TEXT_SIZE));
        ui.vertical(add).inner
    })
    .inner
}

fn home(app: &mut App, ui: &mut Ui) -> Option<HintPage> {
    let mut go = None;
    title(ui, &tr("hint.home"));
    rich(ui, &tr("hint.home_intro"));
    ui.add_space(4.0);
    for t in Topic::ALL {
        bullet_row(ui, |ui| {
            if link(ui, &t.title()).clicked() {
                go = Some(HintPage::Topic(t));
            }
        });
    }
    learn_more(app, ui, &tr("hint.app_help"), "README.md");
    go
}

fn topic(app: &mut App, ui: &mut Ui, t: Topic) -> Option<HintPage> {
    let mut go = None;
    title(ui, &t.title());
    rich(ui, &tr(&format!("hinttopic.{}", t.id())));
    ui.add_space(6.0);
    for &tool in t.tools() {
        if tool_row(ui, tool) {
            go = Some(HintPage::Tool(tool));
        }
        ui.add_space(4.0);
    }
    let page = t
        .tools()
        .first()
        .map(|t| help_page(*t))
        .unwrap_or("../README.md");
    learn_more(app, ui, &t.title(), &format!("behavior/{page}"));
    go
}

/// A tool's icon, its bold name and what it does; true when clicked.
fn tool_row(ui: &mut Ui, tool: Tool) -> bool {
    let width = ui.available_width();
    let text = format!(
        "**{}**: {}",
        tool.name(),
        tr(&format!("tooldesc.{}", tool.id()))
    );
    let job = rich_job(&text, (width - 30.0).max(60.0), TEXT_SIZE);
    let galley = ui.painter().layout_job(job);
    let h = galley.size().y.max(20.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, h), Sense::click());
    let icon = Rect::from_min_size(rect.min + Vec2::new(0.0, 0.0), Vec2::splat(20.0));
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect.expand(2.0), 2.0, Tokens::TOOL_HOVER);
    }
    crate::ui::icons::draw(ui.painter(), icon, tool, Tokens::ICON);
    ui.painter().galley(
        Pos2::new(rect.min.x + 28.0, rect.min.y + 1.0),
        galley,
        Tokens::TEXT,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn tool_page(app: &mut App, ui: &mut Ui, tool: Tool) -> Option<HintPage> {
    title(ui, &tr(&format!("hinttitle.{}", tool.id())));
    tool_row(ui, tool);
    ui.add_space(6.0);
    // The status bar hint, one bullet per instruction, keys in bold.
    let hint = crate::ui::status::tool_hint_for(tool, app.selection.is_empty());
    for part in split_hint(&hint) {
        bullet_row(ui, |ui| rich(ui, &emphasize_keys(&part)));
        ui.add_space(4.0);
    }
    learn_more(
        app,
        ui,
        &tr(&format!("hinttitle.{}", tool.id())),
        &format!("behavior/{}", help_page(tool)),
    );
    // Which topic lists this tool, for a way back up.
    let topic = Topic::ALL.into_iter().find(|t| t.tools().contains(&tool))?;
    let mut go = None;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tr("hint.see_also")).size(TEXT_SIZE));
        if link(ui, &topic.title()).clicked() {
            go = Some(HintPage::Topic(topic));
        }
    });
    go
}

/// Split a hint into its instructions (separated by semicolons in every
/// script the interface uses).
pub fn split_hint(hint: &str) -> Vec<String> {
    hint.split([';', '；', '؛', '。'])
        .map(|s| s.trim().trim_end_matches('.').trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => s,
            }
        })
        .collect()
}

/// Put the modifier and special key names of a hint in bold.
pub fn emphasize_keys(text: &str) -> String {
    const KEYS: [&str; 16] = [
        "Ctrl",
        "Shift",
        "Alt",
        "Enter",
        "Esc",
        "Tab",
        "Strg",
        "Umschalt",
        "Mayús",
        "Maj",
        "Entrée",
        "Intro",
        "Space",
        "Espaço",
        "Espace",
        "Leertaste",
    ];
    let mut out = String::with_capacity(text.len() + 16);
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if KEYS.contains(&word.as_str()) {
            out.push_str("**");
            out.push_str(word);
            out.push_str("**");
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            word.push(ch);
        } else {
            flush(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// "Learn more" with the help topic link.
fn learn_more(app: &mut App, ui: &mut Ui, label: &str, page: &str) {
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("hint.learn_more")).font(theme::bold(18.0)));
    ui.add_space(8.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        let (r, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
        info_icon(ui.painter(), r);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(tr("hint.help_topic")).font(theme::bold(16.0)));
            if link(ui, label).clicked() {
                app.open_url(&format!("{DOCS}{page}"));
            }
        });
    });
}

fn info_icon(p: &egui::Painter, r: Rect) {
    let c = r.center();
    let s = Stroke::new(2.0, Tokens::ICON);
    p.circle_stroke(c, 11.0, s);
    p.circle_filled(c + Vec2::new(0.0, -5.0), 1.6, Tokens::ICON);
    p.line_segment([c + Vec2::new(0.0, -1.5), c + Vec2::new(0.0, 6.0)], s);
}

/// Home at the left, back and forward at the right.
fn nav_bar(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::hover());
    let p = ui.painter();
    p.hline(row.x_range(), row.min.y, Stroke::new(1.0, Tokens::BORDER));
    let home_r = Rect::from_center_size(
        Pos2::new(row.min.x + 14.0, row.center().y + 1.0),
        Vec2::splat(22.0),
    );
    let fwd_r = Rect::from_center_size(
        Pos2::new(row.max.x - 14.0, row.center().y + 1.0),
        Vec2::splat(22.0),
    );
    let back_r = fwd_r.translate(Vec2::new(-28.0, 0.0));
    let home = ui
        .interact(home_r, egui::Id::new("hints_home"), Sense::click())
        .on_hover_text(tr("hint.home"));
    let back = ui
        .interact(back_r, egui::Id::new("hints_back"), Sense::click())
        .on_hover_text(tr("hint.back"));
    let fwd = ui
        .interact(fwd_r, egui::Id::new("hints_forward"), Sense::click())
        .on_hover_text(tr("hint.forward"));
    let p = ui.painter();
    for (r, resp) in [(home_r, &home), (back_r, &back), (fwd_r, &fwd)] {
        if resp.hovered() {
            p.rect_filled(r, 2.0, Tokens::TOOL_HOVER);
        }
    }
    house(p, home_r.center(), Tokens::ICON);
    let off = Color32::from_rgb(0xB0, 0xB0, 0xB0);
    arrow(
        p,
        back_r.center(),
        -1.0,
        if app.hints.can_back() {
            Tokens::ICON
        } else {
            off
        },
    );
    arrow(
        p,
        fwd_r.center(),
        1.0,
        if app.hints.can_forward() {
            Tokens::ICON
        } else {
            off
        },
    );
    if home.clicked() {
        app.hints.go(HintPage::Home);
    }
    if back.clicked() {
        app.hints.back();
    }
    if fwd.clicked() {
        app.hints.forward();
    }
}

fn house(p: &egui::Painter, c: Pos2, color: Color32) {
    let s = Stroke::new(1.2, color);
    let pts = [
        c + Vec2::new(-7.0, 0.0),
        c + Vec2::new(0.0, -7.0),
        c + Vec2::new(7.0, 0.0),
    ];
    p.line_segment([pts[0], pts[1]], s);
    p.line_segment([pts[1], pts[2]], s);
    let body = Rect::from_min_max(c + Vec2::new(-5.0, -1.5), c + Vec2::new(5.0, 7.0));
    p.rect_stroke(body, 0.0, s, egui::StrokeKind::Middle);
    let door = Rect::from_min_max(c + Vec2::new(-1.5, 2.5), c + Vec2::new(1.5, 7.0));
    p.rect_stroke(door, 0.0, s, egui::StrokeKind::Middle);
}

/// A hollow arrow pointing left (`dir` < 0) or right.
fn arrow(p: &egui::Painter, c: Pos2, dir: f32, color: Color32) {
    let s = Stroke::new(1.2, color);
    let pts = vec![
        c + Vec2::new(8.0 * dir, 0.0),
        c + Vec2::new(1.0 * dir, -6.0),
        c + Vec2::new(1.0 * dir, -2.5),
        c + Vec2::new(-8.0 * dir, -2.5),
        c + Vec2::new(-8.0 * dir, 2.5),
        c + Vec2::new(1.0 * dir, 2.5),
        c + Vec2::new(1.0 * dir, 6.0),
    ];
    p.add(egui::epaint::PathShape::closed_line(pts, s));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_goes_back_and_forward_and_drops_the_future() {
        let mut h = HintsState::default();
        assert_eq!(h.current(), HintPage::Home);
        assert!(!h.can_back());
        h.go(HintPage::Topic(Topic::Lines));
        h.go(HintPage::Tool(Tool::Bezier));
        h.back();
        assert_eq!(h.current(), HintPage::Topic(Topic::Lines));
        assert!(h.can_forward());
        h.go(HintPage::Topic(Topic::Fill));
        assert!(!h.can_forward());
        h.back();
        h.back();
        assert_eq!(h.current(), HintPage::Home);
        // Choosing a tool shows its page; the same tool again does nothing.
        h.follow(Tool::Rectangle);
        assert_eq!(h.current(), HintPage::Tool(Tool::Rectangle));
        h.back();
        h.follow(Tool::Rectangle);
        assert_eq!(h.current(), HintPage::Home);
    }

    #[test]
    fn every_tool_has_a_page_title_description_and_topic() {
        crate::i18n::set_language("en");
        for t in Tool::ALL {
            for key in [
                format!("hinttitle.{}", t.id()),
                format!("tooldesc.{}", t.id()),
            ] {
                assert_ne!(tr(&key), key, "missing {key}");
            }
            // Zoom and Pan change the view, not the drawing: no topic.
            let in_topic = Topic::ALL.iter().any(|tp| tp.tools().contains(&t));
            assert!(
                in_topic || matches!(t, Tool::Zoom | Tool::Pan),
                "{t:?} is in no topic"
            );
        }
        for tp in Topic::ALL {
            let key = format!("hinttopic.{}", tp.id());
            assert_ne!(tr(&key), key, "missing {key}");
        }
    }

    #[test]
    fn hints_split_into_instructions_with_keys_in_bold() {
        let parts = split_hint("Ctrl+drag constrains; Shift+drag draws from center");
        assert_eq!(
            parts,
            vec!["Ctrl+drag constrains", "Shift+drag draws from center"]
        );
        assert_eq!(emphasize_keys("Ctrl+drag"), "**Ctrl**+drag");
        assert_eq!(split_hint("单击放大；右键单击缩小").len(), 2);
    }
}
