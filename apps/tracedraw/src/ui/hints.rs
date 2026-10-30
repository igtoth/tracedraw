//! The Hints panel: a Home page
//! listing topics, one page per topic (an introduction and the tools that
//! serve it), one page per tool (what it does, how to use it, and a
//! link to its help topic) and one page per kind of object (what can be
//! done with it, its tools and its property bar). Choosing a tool shows
//! its page, selecting an object shows the page of what was picked, and
//! letting go of the selection goes back to the tool's page; a bar at the
//! bottom goes home, back and forward.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::{self, Tokens};
use crate::tools::Tool;
use crate::ui::tips::RichTip;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use tracedraw_core::live::Effect as LiveEffect;
use tracedraw_core::{PageId, Shape, ShapeId, ShapeKind};

/// What the selection was when the panel last looked: the document, its
/// revision, the page, how many objects and the first and last of them.
/// Cheap to build and compare every frame; the selected objects are only
/// read when it changes.
pub type SelectionKey = (u64, u64, PageId, usize, Option<ShapeId>, Option<ShapeId>);

/// A page of the docker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintPage {
    Home,
    Topic(Topic),
    Tool(Tool),
    Object(ObjectKind),
}

/// The kinds of selected object the panel has a page for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Curve,
    Rectangle,
    Ellipse,
    Polygon,
    Text,
    ParagraphText,
    Bitmap,
    Group,
    ClipFrame,
    Blend,
    Dimension,
    Connector,
    Callout,
    Table,
    Symbol,
    Effect,
    Several,
}

impl ObjectKind {
    #[cfg(test)]
    pub const ALL: [ObjectKind; 17] = [
        ObjectKind::Curve,
        ObjectKind::Rectangle,
        ObjectKind::Ellipse,
        ObjectKind::Polygon,
        ObjectKind::Text,
        ObjectKind::ParagraphText,
        ObjectKind::Bitmap,
        ObjectKind::Group,
        ObjectKind::ClipFrame,
        ObjectKind::Blend,
        ObjectKind::Dimension,
        ObjectKind::Connector,
        ObjectKind::Callout,
        ObjectKind::Table,
        ObjectKind::Symbol,
        ObjectKind::Effect,
        ObjectKind::Several,
    ];

    pub fn id(self) -> &'static str {
        match self {
            ObjectKind::Curve => "curve",
            ObjectKind::Rectangle => "rectangle",
            ObjectKind::Ellipse => "ellipse",
            ObjectKind::Polygon => "polygon",
            ObjectKind::Text => "text",
            ObjectKind::ParagraphText => "paragraph_text",
            ObjectKind::Bitmap => "bitmap",
            ObjectKind::Group => "group",
            ObjectKind::ClipFrame => "clip_frame",
            ObjectKind::Blend => "blend",
            ObjectKind::Dimension => "dimension",
            ObjectKind::Connector => "connector",
            ObjectKind::Callout => "callout",
            ObjectKind::Table => "table",
            ObjectKind::Symbol => "symbol",
            ObjectKind::Effect => "effect",
            ObjectKind::Several => "several",
        }
    }

    /// The tools that work on this kind of object.
    pub fn tools(self) -> &'static [Tool] {
        match self {
            ObjectKind::Curve => &[Tool::Shape, Tool::Knife, Tool::Eraser],
            ObjectKind::Rectangle => &[Tool::Shape, Tool::Rectangle],
            ObjectKind::Ellipse => &[Tool::Shape, Tool::Ellipse],
            ObjectKind::Polygon => &[Tool::Shape, Tool::Polygon, Tool::Star],
            ObjectKind::Text => &[Tool::Text, Tool::Shape],
            ObjectKind::ParagraphText => &[Tool::Text],
            ObjectKind::Bitmap => &[Tool::Crop, Tool::ColorEyedropper],
            ObjectKind::Group | ObjectKind::ClipFrame | ObjectKind::Symbol => &[Tool::Pick],
            ObjectKind::Blend => &[Tool::Blend],
            ObjectKind::Dimension => &[
                Tool::ParallelDimension,
                Tool::HorizontalVerticalDimension,
                Tool::AngularDimension,
            ],
            ObjectKind::Connector => &[Tool::Connector, Tool::AnchorEditing],
            ObjectKind::Callout => &[Tool::Callout, Tool::Text],
            ObjectKind::Table => &[Tool::Table],
            ObjectKind::Effect => &[Tool::Contour, Tool::Envelope, Tool::Extrude, Tool::Distort],
            ObjectKind::Several => &[Tool::Pick, Tool::FreeTransform],
        }
    }

    /// The Help page about this kind of object.
    pub fn help_topic(self) -> &'static str {
        match self {
            ObjectKind::Curve => "lines-shapes/shape-tool",
            ObjectKind::Rectangle | ObjectKind::Ellipse => "lines-shapes/rectangles-and-ellipses",
            ObjectKind::Polygon => "lines-shapes/polygons-stars-spirals",
            ObjectKind::Text => "text/adding-text",
            ObjectKind::ParagraphText => "text/paragraphs",
            ObjectKind::Bitmap => "bitmaps/index",
            ObjectKind::Group | ObjectKind::Several => "objects/selecting",
            ObjectKind::ClipFrame => "objects/clip-frames",
            ObjectKind::Blend => "effects/blend",
            ObjectKind::Dimension | ObjectKind::Callout => "lines-shapes/dimensions",
            ObjectKind::Connector => "lines-shapes/connectors",
            ObjectKind::Table => "objects/tables",
            ObjectKind::Symbol => "objects/symbols",
            ObjectKind::Effect => "effects/index",
        }
    }

    /// The page for a selection: the kind its objects share, Several when
    /// they differ, None when nothing is selected.
    pub fn of_selection(shapes: &[Shape]) -> Option<ObjectKind> {
        let first = ObjectKind::of(shapes.first()?);
        if shapes.iter().skip(1).all(|s| ObjectKind::of(s) == first) {
            Some(first)
        } else {
            Some(ObjectKind::Several)
        }
    }

    /// The kind of one object. Dimensions, connectors, callouts and blends
    /// are known by their data and effects before their plain shape.
    pub fn of(s: &Shape) -> ObjectKind {
        if crate::dimensions::is_dimension(s) {
            return ObjectKind::Dimension;
        }
        if crate::connectors::Connection::of(s).is_some() {
            return ObjectKind::Connector;
        }
        if crate::callouts::Callout::of(s).is_some() {
            return ObjectKind::Callout;
        }
        if s.effects
            .iter()
            .any(|e| matches!(e, LiveEffect::Blend { .. }))
        {
            return ObjectKind::Blend;
        }
        if s.effects.iter().any(|e| {
            matches!(
                e,
                LiveEffect::Contour { .. }
                    | LiveEffect::Distort(_)
                    | LiveEffect::Envelope { .. }
                    | LiveEffect::Perspective { .. }
                    | LiveEffect::Extrude { .. }
                    | LiveEffect::Bevel { .. }
                    | LiveEffect::BlockShadow { .. }
            )
        }) {
            return ObjectKind::Effect;
        }
        match &s.kind {
            ShapeKind::Rect { .. } => ObjectKind::Rectangle,
            ShapeKind::Ellipse { .. } => ObjectKind::Ellipse,
            ShapeKind::Polygon { .. } => ObjectKind::Polygon,
            ShapeKind::Path { .. } => ObjectKind::Curve,
            ShapeKind::Text { frame: Some(_), .. } => ObjectKind::ParagraphText,
            ShapeKind::Text { .. } => ObjectKind::Text,
            ShapeKind::Group { .. } => ObjectKind::Group,
            ShapeKind::ClipFrame { .. } => ObjectKind::ClipFrame,
            ShapeKind::Table(_) => ObjectKind::Table,
            ShapeKind::SymbolInstance { .. } => ObjectKind::Symbol,
            ShapeKind::Bitmap { .. } => ObjectKind::Bitmap,
        }
    }
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
    Ai,
    Help,
}

impl Topic {
    pub const ALL: [Topic; 14] = [
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
        Topic::Ai,
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
            Topic::Ai => "ai",
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
                Tool::PainterlyBrush,
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
                Tool::VariableOutline,
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
            Topic::Ai | Topic::Help => &[],
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
    /// The selection the docker last followed.
    selection: Option<SelectionKey>,
}

impl Default for HintsState {
    fn default() -> Self {
        HintsState {
            history: vec![HintPage::Home],
            pos: 0,
            followed: Tool::Pick,
            selection: None,
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

    /// Follow the selection after the tool. `key` names the selection;
    /// `kind` reads it and is only called when the key changed. A newly
    /// picked object shows its kind's page; an empty selection goes back
    /// to the tool's page. With a tool that draws, a selection that came
    /// with a change to the drawing (the object just drawn) leaves the
    /// tool's steps in view; with the Pick and Shape tools every new
    /// selection shows its page. A change to the drawing that keeps the
    /// selection only updates an object page already in view (Convert to
    /// Curves turns the rectangle's page into the curve's).
    pub fn follow_selection(
        &mut self,
        tool: Tool,
        key: SelectionKey,
        kind: impl FnOnce() -> Option<ObjectKind>,
    ) {
        let Some(old) = self.selection.replace(key) else {
            // First look: follow what is selected now, as a pick.
            if let Some(k) = kind() {
                self.go(HintPage::Object(k));
            }
            return;
        };
        if old == key {
            return;
        }
        let same_selection =
            (old.0, old.2, old.3, old.4, old.5) == (key.0, key.2, key.3, key.4, key.5);
        let edited = old.0 != key.0 || old.1 != key.1;
        let kind = kind();
        let on_object = matches!(self.current(), HintPage::Object(_));
        if same_selection {
            if let (true, Some(k)) = (on_object, kind) {
                self.go(HintPage::Object(k));
            }
            return;
        }
        match kind {
            None => {
                if on_object {
                    self.go(HintPage::Tool(tool));
                }
            }
            Some(k) => {
                let selects = matches!(
                    tool,
                    Tool::Pick | Tool::FreeformPick | Tool::FreeTransform | Tool::Shape
                );
                if selects || !edited {
                    self.go(HintPage::Object(k));
                }
            }
        }
    }
}

/// The key of the app's selection for [`HintsState::follow_selection`].
pub fn selection_key(app: &App) -> SelectionKey {
    (
        app.engine.instance(),
        app.engine.revision(),
        app.page,
        app.selection.len(),
        app.selection.first().copied(),
        app.selection.last().copied(),
    )
}

/// The docker: the page in a scroll area and the navigation bar under it.
pub fn hints_docker(app: &mut App, ui: &mut Ui) {
    app.hints.follow(app.tool);
    let key = selection_key(app);
    let tool = app.tool;
    // The selection is read only when its key changed (once per change,
    // not every frame).
    let mut hints = std::mem::take(&mut app.hints);
    hints.follow_selection(tool, key, || {
        ObjectKind::of_selection(app.selected_rc().as_slice())
    });
    app.hints = hints;
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
                HintPage::Object(k) => object_page(app, ui, k),
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

/// The widest a step's picture grows, in points.
const PICTURE_MAX_W: f32 = 320.0;

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
        color: Tokens::text(),
        line_height: Some(LINE_HEIGHT),
        ..Default::default()
    };
    let strong = TextFormat {
        font_id: theme::bold(size),
        color: Tokens::text(),
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
                .color(crate::theme::link_color())
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
    learn_more(app, ui, &tr("hint.app_help"), crate::help::HOME);
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
    if t == Topic::Ai {
        ai_topic(app, ui);
    }
    let page = match t {
        Topic::Ai => "customize/ai",
        _ => t
            .tools()
            .first()
            .map(|t| crate::help::tool_topic(*t))
            .unwrap_or(crate::help::HOME),
    };
    learn_more(app, ui, &t.title(), page);
    go
}

/// The AI topic: where each service stands, the ways to connect one and
/// a button to the options.
fn ai_topic(app: &mut App, ui: &mut Ui) {
    let pictures = app.settings.ai.can_make_pictures();
    for (key, ready, offered) in [
        ("aiui.drawings", app.ai_drawing_ready(), true),
        ("aiui.pictures", app.ai_image_ready(), pictures),
    ] {
        bullet_row(ui, |ui| {
            let state = if !offered {
                tr("aiui.drawings_only_state")
            } else if ready {
                tr("aiui.ready")
            } else {
                tr("aiui.not_ready")
            };
            rich(ui, &format!("**{}** {state}", tr(key)));
        });
    }
    ui.add_space(6.0);
    for key in [
        "hintai.cloud",
        "hintai.local",
        "hintai.pictures",
        "hintai.web",
        "hintai.privacy",
    ] {
        bullet_row(ui, |ui| rich(ui, &tr(key)));
        ui.add_space(4.0);
    }
    ui.add_space(4.0);
    if ui
        .button(tr("aiui.set_up"))
        .rich_tip(crate::ui::tips::keyed("aiui.set_up"))
        .clicked()
    {
        crate::ui::ai_ui::open_options(app, crate::ui::dialogs::OptionsPage::AiGeneral);
    }
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
    let resp = resp.rich_tip(format!("{}\n{}", tool.name(), tr("hint.tool_row_desc")));
    let icon = Rect::from_min_size(rect.min + Vec2::new(0.0, 0.0), Vec2::splat(20.0));
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect.expand(2.0), 2.0, Tokens::tool_hover());
    }
    crate::ui::icons::draw(
        ui.painter(),
        icon,
        tool,
        crate::ui::icons::tool_color(tool, Tokens::icon()),
    );
    ui.painter().galley(
        Pos2::new(rect.min.x + 28.0, rect.min.y + 1.0),
        galley,
        Tokens::text(),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn tool_page(app: &mut App, ui: &mut Ui, tool: Tool) -> Option<HintPage> {
    title(ui, &tr(&format!("hinttitle.{}", tool.id())));
    tool_row(ui, tool);
    ui.add_space(8.0);
    // The steps, keys in bold, each with its picture under it.
    for (i, pic) in crate::ui::hint_pics::steps(tool).iter().enumerate() {
        let text = tr(&format!("hintstep.{}.{}", tool.id(), i + 1));
        bullet_row(ui, |ui| {
            rich(ui, &emphasize_keys(&text));
            if let Some(pic) = pic {
                ui.add_space(6.0);
                let w = ui.available_width().min(PICTURE_MAX_W);
                let h = w * crate::ui::hint_pics::H / crate::ui::hint_pics::W;
                let (r, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
                crate::ui::hint_pics::draw(ui.painter(), r, *pic);
            }
        });
        ui.add_space(10.0);
    }
    learn_more(
        app,
        ui,
        &tr(&format!("hinttitle.{}", tool.id())),
        crate::help::tool_topic(tool),
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

/// The page of a kind of object: what it is and what can be done with it,
/// the tools that work on it (each opens its own page) and what the
/// property bar offers for it.
fn object_page(app: &mut App, ui: &mut Ui, kind: ObjectKind) -> Option<HintPage> {
    let id = kind.id();
    title(ui, &tr(&format!("hintobj.{id}.title")));
    for n in 1..=2 {
        bullet_row(ui, |ui| {
            rich(ui, &emphasize_keys(&tr(&format!("hintobj.{id}.{n}"))));
        });
        ui.add_space(6.0);
    }
    let mut go = None;
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("hint.obj_tools")).font(theme::bold(TEXT_SIZE)));
    ui.add_space(4.0);
    for &tool in kind.tools() {
        if tool_row(ui, tool) {
            go = Some(HintPage::Tool(tool));
        }
        ui.add_space(4.0);
    }
    ui.add_space(4.0);
    ui.label(egui::RichText::new(tr("hint.obj_bar")).font(theme::bold(TEXT_SIZE)));
    ui.add_space(2.0);
    rich(ui, &tr(&format!("hintobj.{id}.bar")));
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(tr("hint.obj_back"))
            .size(TEXT_SIZE - 1.0)
            .color(Tokens::text_dim()),
    );
    learn_more(
        app,
        ui,
        &tr(&format!("hintobj.{id}.title")),
        kind.help_topic(),
    );
    go
}

/// Put the modifier and special key names of a hint in bold.
pub fn emphasize_keys(text: &str) -> String {
    const KEYS: [&str; 27] = [
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
        "Espacio",
        "Spazio",
        "Delete",
        "Entf",
        "Suppr",
        "Supr",
        "Canc",
        "Échap",
        "Eingabetaste",
        "Invio",
        "Maiusc",
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
    // Key names are Latin; a change of script ends a word, so "Ctrlキー" still finds Ctrl.
    let latin = |c: char| (c as u32) < 0x0250;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            if word
                .chars()
                .last()
                .is_some_and(|last| latin(last) != latin(ch))
            {
                flush(&mut word, &mut out);
            }
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
fn learn_more(app: &mut App, ui: &mut Ui, label: &str, topic: &str) {
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
                app.open_help(topic);
            }
        });
    });
}

fn info_icon(p: &egui::Painter, r: Rect) {
    let c = r.center();
    let s = Stroke::new(2.0, Tokens::icon());
    p.circle_stroke(c, 11.0, s);
    p.circle_filled(c + Vec2::new(0.0, -5.0), 1.6, Tokens::icon());
    p.line_segment([c + Vec2::new(0.0, -1.5), c + Vec2::new(0.0, 6.0)], s);
}

/// Home at the left, back and forward at the right.
fn nav_bar(app: &mut App, ui: &mut Ui) {
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::hover());
    let p = ui.painter();
    p.hline(row.x_range(), row.min.y, Stroke::new(1.0, Tokens::border()));
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
        .rich_tip(tr("hint.home"));
    let back = ui
        .interact(back_r, egui::Id::new("hints_back"), Sense::click())
        .rich_tip(tr("hint.back"));
    let fwd = ui
        .interact(fwd_r, egui::Id::new("hints_forward"), Sense::click())
        .rich_tip(tr("hint.forward"));
    let p = ui.painter();
    for (r, resp) in [(home_r, &home), (back_r, &back), (fwd_r, &fwd)] {
        if resp.hovered() {
            p.rect_filled(r, 2.0, Tokens::tool_hover());
        }
    }
    house(p, home_r.center(), Tokens::icon());
    let off = Color32::from_rgb(0xB0, 0xB0, 0xB0);
    arrow(
        p,
        back_r.center(),
        -1.0,
        if app.hints.can_back() {
            Tokens::icon()
        } else {
            off
        },
    );
    arrow(
        p,
        fwd_r.center(),
        1.0,
        if app.hints.can_forward() {
            Tokens::icon()
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
    fn every_object_kind_has_its_texts_tools_and_a_help_page() {
        crate::i18n::set_language("en");
        let pages = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/help/contents.txt"
        ))
        .expect("contents");
        for k in ObjectKind::ALL {
            for part in ["title", "1", "2", "bar"] {
                let key = format!("hintobj.{}.{part}", k.id());
                assert_ne!(tr(&key), key, "missing {key}");
            }
            assert!(!k.tools().is_empty(), "{k:?}");
            assert!(
                pages.lines().any(|l| l.trim() == k.help_topic()),
                "{k:?}: {} is not a Help page",
                k.help_topic()
            );
        }
        for key in ["hint.obj_tools", "hint.obj_bar", "hint.obj_back"] {
            assert_ne!(tr(key), key, "missing {key}");
        }
    }

    #[test]
    fn the_selection_shows_its_page_and_letting_go_shows_the_tool() {
        let key = |rev: u64, ids: &[u64]| -> SelectionKey {
            (
                1,
                rev,
                PageId(1),
                ids.len(),
                ids.first().map(|i| ShapeId(*i)),
                ids.last().map(|i| ShapeId(*i)),
            )
        };
        let mut h = HintsState::default();
        h.follow(Tool::Pick);
        h.follow_selection(Tool::Pick, key(0, &[]), || None);
        assert_eq!(h.current(), HintPage::Home);
        // A pick shows the object's page; the same key reads nothing.
        h.follow_selection(Tool::Pick, key(0, &[5]), || Some(ObjectKind::Text));
        assert_eq!(h.current(), HintPage::Object(ObjectKind::Text));
        h.follow_selection(Tool::Pick, key(0, &[5]), || panic!("read again"));
        // Two objects of different kinds.
        h.follow_selection(Tool::Pick, key(0, &[5, 6]), || Some(ObjectKind::Several));
        assert_eq!(h.current(), HintPage::Object(ObjectKind::Several));
        // Letting go goes back to the tool's page.
        h.follow_selection(Tool::Pick, key(0, &[]), || None);
        assert_eq!(h.current(), HintPage::Tool(Tool::Pick));
        // An edit that keeps the selection leaves a page that is not an
        // object page alone.
        h.follow_selection(Tool::Pick, key(0, &[7]), || Some(ObjectKind::Rectangle));
        h.go(HintPage::Home);
        h.follow_selection(Tool::Pick, key(1, &[7]), || Some(ObjectKind::Curve));
        assert_eq!(h.current(), HintPage::Home);
        // A drawing tool: the object it just drew keeps the tool's steps,
        // a plain pick (no edit) shows the object.
        h.follow(Tool::Ellipse);
        h.follow_selection(Tool::Ellipse, key(2, &[8]), || Some(ObjectKind::Ellipse));
        assert_eq!(h.current(), HintPage::Tool(Tool::Ellipse));
        h.follow_selection(Tool::Ellipse, key(2, &[7]), || Some(ObjectKind::Curve));
        assert_eq!(h.current(), HintPage::Object(ObjectKind::Curve));
    }

    #[test]
    fn step_keys_are_in_bold() {
        assert_eq!(emphasize_keys("Ctrl+drag"), "**Ctrl**+drag");
        assert_eq!(emphasize_keys("Hold Alt"), "Hold **Alt**");
        assert_eq!(emphasize_keys("Ctrlキーを押す"), "**Ctrl**キーを押す");
    }
}
