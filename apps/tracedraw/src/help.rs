//! Context help: which page of the user Help each dialog, options page,
//! docker and tool opens, in the language of the interface.
//!
//! A topic is a page of `docs/help/<lang>/` without the `.md`, with an
//! optional `#anchor` (an explicit `{#id}` on one of its headings, the
//! same in every language). The published Help lives at
//! `<BASE><lang>/<page>.html#anchor`; every language has every page (a
//! missing translation is published from the English page), so a link
//! never breaks. See docs/behavior/help.md.

use crate::app::{App, DockerTab};
use crate::tools::Tool;
use crate::ui::dialogs::{Dialog, OptionsPage};

/// Where the Help is published (`web/help_site.py` builds it into
/// `site/help/`, the project's page).
pub const BASE: &str = "https://igtoth.github.io/tracedraw/help/";

/// The first page of the Help.
pub const HOME: &str = "index";

/// The address of `topic` in `lang` (an interface language code, such as
/// "pt-BR"); an unknown language opens the English Help.
pub fn url_in(topic: &str, lang: &str) -> String {
    let lang = if crate::i18n::LANGUAGES.iter().any(|(c, _)| *c == lang) {
        lang
    } else {
        "en"
    };
    let (page, anchor) = match topic.split_once('#') {
        Some((p, a)) => (p, Some(a)),
        None => (topic, None),
    };
    let page = if page.is_empty() { HOME } else { page };
    match anchor {
        Some(a) => format!("{BASE}{lang}/{page}.html#{a}"),
        None => format!("{BASE}{lang}/{page}.html"),
    }
}

/// The address of `topic` in the interface language.
pub fn url(topic: &str) -> String {
    url_in(topic, &crate::i18n::language())
}

/// The page a dialog's "?" opens. Options goes by its page.
pub fn dialog_topic(d: &Dialog, options: OptionsPage) -> &'static str {
    match d {
        Dialog::None => HOME,
        Dialog::NewFromTemplate(_) => "templates-styles/templates#new-from-template",
        Dialog::BitmapBw(_) => "bitmaps/color-modes#black-and-white",
        Dialog::BitmapDuotone(_) => "bitmaps/color-modes#duotone",
        Dialog::BitmapPaletted(_) => "bitmaps/color-modes#paletted",
        Dialog::RemoveBackground(_) => "bitmaps/remove-background",
        Dialog::NodeAlign { .. } => "lines-shapes/shape-tool#align-nodes",
        Dialog::RenamePage { .. } => "pages-layout/pages#rename-page",
        Dialog::GoToPage { .. } => "pages-layout/pages#go-to-page",
        Dialog::InsertPage { .. } => "pages-layout/pages#insert-page",
        Dialog::DeletePage { .. } => "pages-layout/pages#delete-page",
        Dialog::DuplicatePage { .. } => "pages-layout/pages#duplicate-page",
        Dialog::PageSize { .. } => "pages-layout/pages#page-size",
        Dialog::Options => options_topic(options),
        Dialog::PageNumberSettings => "pages-layout/pages#page-numbers",
        Dialog::LineStyle(_) | Dialog::OutlinePen(_) => "lines-shapes/outlines#outline-pen",
        Dialog::EditFill(_) => "color/fills#edit-fill",
        Dialog::TableCount(_) => "objects/tables",
        Dialog::OldCdrVersion { .. } => "get-started/open-and-save#save-as",
        Dialog::RenameLayer { .. } | Dialog::RenameObject { .. } => {
            "objects/objects-and-layers#rename"
        }
        Dialog::DocumentProperties(_) => "pages-layout/document-properties",
        Dialog::ConfirmClose => "get-started/open-and-save#closing",
        Dialog::ConfirmResetSettings => "customize/reset-settings",
        Dialog::Recover(_) => "get-started/open-and-save#recovery",
        Dialog::Updates => "welcome/about-and-updates#updates",
        Dialog::UpdatePolicy => "welcome/about-and-updates#automatic-check",
        Dialog::OpenNotes(_) | Dialog::MissingFonts(_) => "get-started/open-and-save#open-notes",
        Dialog::ConfirmShelfRemove { .. } => "objects/shelf#remove",
        Dialog::Export(_) => "file-formats/export",
        Dialog::Print(_) => "printing/print",
        Dialog::PrintMerge(_) => "printing/print-merge",
        Dialog::CreatePattern { .. } => "color/patterns#create-pattern",
        Dialog::CreateTable { .. } => "objects/tables#create-table",
        Dialog::TextToTable(_) => "objects/tables#convert",
        Dialog::PasteSpecial(_) => "objects/paste-special",
        Dialog::Barcode(_) => "objects/barcodes",
        Dialog::ConvertToBitmap(_) => "bitmaps/convert-to-bitmap",
        Dialog::StraightenImage(_) => "bitmaps/resample-and-straighten#straighten",
        Dialog::Resample(_) => "bitmaps/resample-and-straighten#resample",
        Dialog::InflateBitmap(_) => "bitmaps/resample-and-straighten#inflate",
        Dialog::Trace(_) => "bitmaps/tracing",
        Dialog::Effect(_) => "effects/bitmap-effects",
        Dialog::TextDefaults { .. } => "text/fonts",
        Dialog::CopyProperties(_) => "effects/copy-and-clone#clone",
        Dialog::EditText { .. } => "text/adding-text#edit",
        Dialog::TextTabs => "text/paragraphs#tabs",
        Dialog::TextColumns => "text/paragraphs#columns",
        Dialog::TextBullets => "text/paragraphs#bullets",
        Dialog::TextDropCap => "text/paragraphs#drop-cap",
        Dialog::TextStatistics => "text/writing-tools#statistics",
        Dialog::ChangeCase(_) => "text/writing-tools#change-case",
        Dialog::WritingTools(_) => "text/writing-tools",
        Dialog::CustomHyphens(_) => "text/writing-tools#custom-hyphens",
        Dialog::Encode(_) => "text/writing-tools#encode",
        Dialog::BorderGrommet(_) => "printing/border-and-grommet",
        Dialog::ColorManagement(_) | Dialog::ConvertColors(_) => "color/color-management",
        Dialog::FontManager(_) => "text/fonts#font-manager",
        Dialog::PaletteEditor(_) => "color/palettes#palette-editor",
        Dialog::NewDocument(_) => "get-started/new-document",
        Dialog::About => "welcome/about-and-updates#about",
        Dialog::GithubOpen(_) => "get-started/github#open",
        Dialog::GithubSave(_) => "get-started/github#save",
        Dialog::GithubHistory => "get-started/github#history",
        Dialog::WebNotice => "get-started/browser-version",
        Dialog::MacrosDesktopOnly => "get-started/browser-version#differences",
        Dialog::TimeTracker(_) => "customize/time-tracker",
        Dialog::Preflight(_) => "printing/preflight",
        Dialog::PublishPdf(_) => "file-formats/publish-pdf",
        Dialog::PdfPassword(_) => "file-formats/publish-pdf#password",
        Dialog::PdfText(_) => "file-formats/import#formats",
        Dialog::ChangeDefaults(_) => "templates-styles/object-styles#document-defaults",
        Dialog::SaveAs(_) | Dialog::Package(_) => "get-started/open-and-save#save-as",
    }
}

/// The page of an options page: one page per options dialog, one
/// section per page of it.
pub fn options_topic(p: OptionsPage) -> &'static str {
    use OptionsPage::*;
    match p {
        General => "customize/options-app#general",
        Display => "customize/options-app#display",
        Edit => "customize/options-app#edit",
        NodesAndHandles => "customize/options-app#nodes-and-handles",
        ClipFrame => "customize/options-app#clip-frame",
        Snapping | AlignmentGuides | DynamicGuides => "customize/options-app#snapping",
        Save => "customize/options-app#save",
        Text => "customize/options-app#text",
        Appearance => "customize/customization#appearance",
        Commands => "customize/customization#commands",
        CommandBars => "customize/customization#command-bars",
        ColorPalette => "customize/customization#color-palette",
        Pick => "customize/tool-options#pick",
        ZoomPan => "customize/tool-options#zoom-pan",
        Curves => "customize/tool-options#curves",
        Rectangle => "customize/tool-options#rectangle",
        Ellipse => "customize/tool-options#ellipse",
        Polygon => "customize/tool-options#polygon",
        Spiral => "customize/tool-options#spiral",
        GraphPaper => "customize/tool-options#graph-paper",
        Table => "customize/tool-options#table",
        Eraser => "customize/tool-options#eraser",
        GlobalGeneral => "customize/global-options#general",
        Printing => "customize/global-options#printing",
        FileFormats => "customize/global-options#file-formats",
        BitmapTracing => "customize/global-options#bitmap-tracing",
        Warnings => "customize/global-options#warnings",
        FileLocations => "customize/global-options#file-locations",
        Workspace => "customize/workspaces",
        AiGeneral => "customize/ai#general",
        AiService => "customize/ai#service",
        DocumentGeneral => "pages-layout/document-options#general",
        PageSize => "pages-layout/document-options#page-size",
        Layout => "pages-layout/document-options#layout",
        Background => "pages-layout/document-options#background",
        Bleed => "pages-layout/document-options#bleed",
        Rulers => "pages-layout/document-options#rulers",
        Grid => "pages-layout/document-options#grid",
        Guidelines => "pages-layout/document-options#guidelines",
    }
}

/// The page of a docker.
pub fn docker_topic(t: DockerTab) -> &'static str {
    match t {
        DockerTab::Properties => "objects/properties-docker",
        DockerTab::Objects => "objects/objects-and-layers",
        DockerTab::Hints => "get-started/hints",
        DockerTab::Transformations => "objects/transforming#transformations-docker",
        DockerTab::Undo => "get-started/undo-and-views#undo-docker",
        DockerTab::AlignDistribute => "objects/align-distribute",
        DockerTab::Shaping => "lines-shapes/shaping",
        DockerTab::StepAndRepeat => "objects/step-and-repeat",
        DockerTab::Text => "text/adding-text#text-docker",
        DockerTab::Glyphs => "text/fonts#glyphs",
        DockerTab::Color => "color/color-docker",
        DockerTab::ColorStyles => "color/color-styles",
        DockerTab::ObjectStyles => "templates-styles/object-styles",
        DockerTab::FindReplace => "objects/find-replace",
        DockerTab::Scripts => "customize/scripts",
        DockerTab::Macros => "customize/scripts#macros",
        DockerTab::PrintMerge => "printing/print-merge#docker",
        DockerTab::Palettes => "color/palettes",
        DockerTab::Lens => "effects/lens",
        DockerTab::Blend => "effects/blend",
        DockerTab::Contour => "effects/contour",
        DockerTab::Envelope => "effects/envelope",
        DockerTab::Extrude => "effects/extrude",
        DockerTab::Bevel => "effects/bevel",
        DockerTab::BrushStrokes => "lines-shapes/artistic-media#brush-strokes",
        DockerTab::BitmapMask => "bitmaps/bitmap-mask",
        DockerTab::ObjectData => "objects/object-data",
        DockerTab::Links => "objects/links",
        DockerTab::Symbols => "objects/symbols",
        DockerTab::Pages => "pages-layout/pages#pages-docker",
        DockerTab::Guidelines => "pages-layout/guidelines-and-grid#guidelines-docker",
        DockerTab::Fonts => "text/fonts",
        DockerTab::FontSampler => "text/fonts#font-sampler",
        DockerTab::Corners => "lines-shapes/rectangles-and-ellipses#corners",
        DockerTab::JoinCurves => "lines-shapes/join-curves",
        DockerTab::FitToPath => "objects/align-distribute#fit-to-path",
        DockerTab::Coordinates => "objects/transforming#coordinates",
        DockerTab::VectorMosaic => "effects/mosaics#vector-mosaic",
        DockerTab::PictureMosaic => "effects/mosaics#picture-mosaic",
        DockerTab::Shelf => "objects/shelf",
        DockerTab::Export => "web-graphics/export-docker",
        DockerTab::Comments => "objects/comments",
        DockerTab::AiGenerate => "customize/ai#generate",
        DockerTab::Effects => "effects/effects-docker",
    }
}

/// The page of a tool.
pub fn tool_topic(t: Tool) -> &'static str {
    match t {
        Tool::Pick => "objects/selecting#pick",
        Tool::FreeformPick => "objects/selecting#freehand-pick",
        Tool::FreeTransform => "objects/transforming#free-transform",
        Tool::Shape => "lines-shapes/shape-tool",
        Tool::Smooth => "lines-shapes/distortion-tools#smooth",
        Tool::Smear => "lines-shapes/distortion-tools#smear",
        Tool::Twirl => "lines-shapes/distortion-tools#twirl",
        Tool::AttractRepel => "lines-shapes/distortion-tools#attract-repel",
        Tool::Smudge => "lines-shapes/distortion-tools#smudge",
        Tool::Roughen => "lines-shapes/distortion-tools#roughen",
        Tool::VariableOutline => "lines-shapes/outlines#variable-outline",
        Tool::Crop => "lines-shapes/cut-and-erase#crop",
        Tool::Knife => "lines-shapes/cut-and-erase#knife",
        Tool::SegmentDelete => "lines-shapes/cut-and-erase#segment-delete",
        Tool::Eraser => "lines-shapes/cut-and-erase#eraser",
        Tool::Zoom => "get-started/undo-and-views#zoom",
        Tool::Pan => "get-started/undo-and-views#pan",
        Tool::Freehand => "lines-shapes/lines#freehand",
        Tool::TwoPointLine => "lines-shapes/lines#two-point-line",
        Tool::Bezier => "lines-shapes/lines#bezier",
        Tool::Pen => "lines-shapes/lines#pen",
        Tool::BSpline => "lines-shapes/lines#b-spline",
        Tool::Polyline => "lines-shapes/lines#polyline",
        Tool::ThreePointCurve => "lines-shapes/lines#three-point-curve",
        Tool::BrushStrokes => "lines-shapes/artistic-media#brush-strokes",
        Tool::PainterlyBrush => "lines-shapes/artistic-media#painterly-brush",
        Tool::ShapeRecognition => "lines-shapes/shape-recognition#shape-recognition",
        Tool::Sketch => "lines-shapes/shape-recognition#sketch",
        Tool::Rectangle => "lines-shapes/rectangles-and-ellipses#rectangle",
        Tool::ThreePointRectangle => "lines-shapes/rectangles-and-ellipses#three-point-rectangle",
        Tool::Ellipse => "lines-shapes/rectangles-and-ellipses#ellipse",
        Tool::ThreePointEllipse => "lines-shapes/rectangles-and-ellipses#three-point-ellipse",
        Tool::Polygon => "lines-shapes/polygons-stars-spirals#polygon",
        Tool::Star => "lines-shapes/polygons-stars-spirals#star",
        Tool::Spiral => "lines-shapes/polygons-stars-spirals#spiral",
        Tool::CommonShapes => "lines-shapes/polygons-stars-spirals#common-shapes",
        Tool::ActionLines => "lines-shapes/polygons-stars-spirals#action-lines",
        Tool::GraphPaper => "lines-shapes/polygons-stars-spirals#graph-paper",
        Tool::Text => "text/adding-text",
        Tool::Table => "objects/tables",
        Tool::ParallelDimension => "lines-shapes/dimensions#parallel",
        Tool::HorizontalVerticalDimension => "lines-shapes/dimensions#horizontal-vertical",
        Tool::AngularDimension => "lines-shapes/dimensions#angular",
        Tool::SegmentDimension => "lines-shapes/dimensions#segment",
        Tool::Callout => "lines-shapes/dimensions#callout",
        Tool::Connector | Tool::RightAngleConnector | Tool::RoundedConnector => {
            "lines-shapes/connectors#connector-tools"
        }
        Tool::AnchorEditing => "lines-shapes/connectors#anchors",
        Tool::DropShadow => "effects/drop-shadow",
        Tool::Contour => "effects/contour",
        Tool::Blend => "effects/blend",
        Tool::Distort => "effects/distort",
        Tool::Envelope => "effects/envelope",
        Tool::Extrude => "effects/extrude",
        Tool::BlockShadow => "effects/block-shadow",
        Tool::Transparency => "color/transparency",
        Tool::ColorEyedropper => "color/eyedroppers#color-eyedropper",
        Tool::AttributesEyedropper => "color/eyedroppers#attributes-eyedropper",
        Tool::InteractiveFill => "color/fills#interactive-fill",
        Tool::AreaFill => "color/fills#area-fill",
        Tool::MeshFill => "color/fills#mesh-fill",
        Tool::OutlinePen => "lines-shapes/outlines#outline-pen",
        Tool::OutlineColor => "lines-shapes/outlines#outline-color",
    }
}

/// What F1 opens: the open dialog's page, else the active docker's,
/// else the active tool's.
pub fn context_topic(app: &App) -> &'static str {
    if !matches!(app.dialog, Dialog::None) {
        return dialog_topic(&app.dialog, app.options_page);
    }
    if app.show_dockers && app.open_dockers.contains(&app.docker_tab) {
        return docker_topic(app.docker_tab);
    }
    tool_topic(app.tool)
}

impl App {
    /// Open `topic` of the Help in the browser.
    pub fn open_help(&mut self, topic: &str) {
        let u = url(topic);
        self.open_url(&u);
    }

    /// The "?" of the dialog being drawn.
    pub fn open_dialog_help(&mut self) {
        self.open_help(self.dialog_help);
    }
}

/// F1, from anywhere: the help of what is in front.
pub fn handle_f1(app: &mut App, ctx: &egui::Context) {
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1)) {
        let t = context_topic(app);
        app.open_help(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    fn help_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/help")
    }

    fn lang_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("lang")
    }

    /// Every page of the contents, as listed in `docs/help/contents.txt`.
    fn contents() -> Vec<String> {
        let text = std::fs::read_to_string(help_dir().join("contents.txt")).unwrap();
        text.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect()
    }

    /// The explicit `{#id}` anchors of a page.
    fn anchors(md: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = md;
        while let Some(i) = rest.find("{#") {
            rest = &rest[i + 2..];
            if let Some(j) = rest.find('}') {
                out.insert(rest[..j].to_string());
                rest = &rest[j..];
            }
        }
        out
    }

    /// The `[[key]]` and `[[key > key]]` interface labels of a page.
    fn ui_keys(md: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut rest = md;
        while let Some(i) = rest.find("[[") {
            rest = &rest[i + 2..];
            if let Some(j) = rest.find("]]") {
                for k in rest[..j].split('>') {
                    out.insert(k.trim().to_string());
                }
                rest = &rest[j + 2..];
            }
        }
        out
    }

    fn lang_keys(code: &str) -> BTreeSet<String> {
        let text = std::fs::read_to_string(lang_dir().join(format!("{code}.toml"))).unwrap();
        text.lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.starts_with('#') {
                    return None;
                }
                l.split_once(" = ").map(|(k, _)| k.trim().to_string())
            })
            .collect()
    }

    fn all_topics() -> Vec<&'static str> {
        use crate::ui::options::OptionsSet;
        let mut t = vec![HOME];
        for s in [
            OptionsSet::App,
            OptionsSet::Customization,
            OptionsSet::Tools,
            OptionsSet::Global,
            OptionsSet::Workspaces,
            OptionsSet::Ai,
            OptionsSet::Document,
        ] {
            t.extend(s.pages().iter().map(|p| options_topic(*p)));
        }
        t.extend(DockerTab::ALL.iter().map(|d| docker_topic(*d)));
        t.extend(Tool::ALL.iter().map(|x| tool_topic(*x)));
        t.extend(
            sample_dialogs()
                .iter()
                .map(|d| dialog_topic(d, OptionsPage::General)),
        );
        t
    }

    /// Dialogs without state; the others are checked through
    /// `dialog_topics_in_source`.
    fn sample_dialogs() -> Vec<Dialog> {
        vec![
            Dialog::None,
            Dialog::Options,
            Dialog::PageNumberSettings,
            Dialog::ConfirmClose,
            Dialog::ConfirmResetSettings,
            Dialog::Updates,
            Dialog::UpdatePolicy,
            Dialog::TextTabs,
            Dialog::TextColumns,
            Dialog::TextBullets,
            Dialog::TextDropCap,
            Dialog::TextStatistics,
            Dialog::About,
            Dialog::GithubHistory,
            Dialog::WebNotice,
            Dialog::MacrosDesktopOnly,
        ]
    }

    /// The topics written in `dialog_topic`, read from this file, so
    /// variants with heavy state are checked too.
    fn dialog_topics_in_source() -> Vec<String> {
        let src =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/help.rs"))
                .unwrap();
        let start = src.find("pub fn dialog_topic").unwrap();
        let end = src[start..].find("\n}\n").unwrap() + start;
        let body = &src[start..end];
        let mut out = Vec::new();
        let mut rest = body;
        while let Some(i) = rest.find("=> \"") {
            rest = &rest[i + 4..];
            if let Some(j) = rest.find('"') {
                out.push(rest[..j].to_string());
            }
        }
        // Arms split over lines.
        let mut rest = body;
        while let Some(i) = rest.find("{\n            \"") {
            rest = &rest[i + 15..];
            if let Some(j) = rest.find('"') {
                out.push(rest[..j].to_string());
            }
        }
        out
    }

    fn page_path(lang: &str, page: &str) -> PathBuf {
        help_dir().join(lang).join(format!("{page}.md"))
    }

    #[test]
    fn urls_carry_the_language_and_fall_back_to_english() {
        assert_eq!(
            url_in("color/fills#mesh-fill", "pt-BR"),
            "https://igtoth.github.io/tracedraw/help/pt-BR/color/fills.html#mesh-fill"
        );
        assert_eq!(
            url_in("printing/print", "xx"),
            "https://igtoth.github.io/tracedraw/help/en/printing/print.html"
        );
        assert_eq!(url_in("", "ja"), format!("{BASE}ja/index.html"));
    }

    #[test]
    fn every_topic_is_a_page_of_the_contents_with_its_anchor() {
        let pages: BTreeSet<String> = contents().into_iter().collect();
        let mut topics: Vec<String> = all_topics().into_iter().map(str::to_string).collect();
        topics.extend(dialog_topics_in_source());
        assert!(topics.len() > 150);
        for t in topics {
            let (page, anchor) = t.split_once('#').unwrap_or((&t, ""));
            assert!(pages.contains(page), "{t}: {page} is not in contents.txt");
            let md = std::fs::read_to_string(page_path("en", page))
                .unwrap_or_else(|_| panic!("{t}: docs/help/en/{page}.md is missing"));
            if !anchor.is_empty() {
                assert!(
                    anchors(&md).contains(anchor),
                    "{t}: no {{#{anchor}}} heading in docs/help/en/{page}.md"
                );
            }
        }
    }

    #[test]
    fn f1_follows_dialog_then_docker_then_tool() {
        let mut app = App::headless();
        app.tool = Tool::Envelope;
        app.show_dockers = false;
        assert_eq!(context_topic(&app), "effects/envelope");
        app.show_dockers = true;
        app.open_dockers = vec![DockerTab::Contour];
        app.docker_tab = DockerTab::Contour;
        assert_eq!(context_topic(&app), "effects/contour");
        app.dialog = Dialog::TextTabs;
        assert_eq!(context_topic(&app), "text/paragraphs#tabs");
        app.dialog = Dialog::Options;
        app.options_page = OptionsPage::Bleed;
        assert_eq!(context_topic(&app), "pages-layout/document-options#bleed");
    }

    #[test]
    fn translated_pages_keep_the_anchors_and_labels_of_english() {
        let pages = contents();
        for page in &pages {
            assert!(
                page_path("en", page).exists(),
                "docs/help/en/{page}.md is missing"
            );
        }
        for (code, _) in crate::i18n::LANGUAGES {
            let keys = lang_keys(code);
            for page in &pages {
                let en = std::fs::read_to_string(page_path("en", page)).unwrap();
                // A page not translated yet shows the English one (see help_site.py).
                let Ok(md) = std::fs::read_to_string(page_path(code, page)) else {
                    continue;
                };
                assert_eq!(
                    anchors(&md),
                    anchors(&en),
                    "docs/help/{code}/{page}.md: the {{#id}} anchors differ from English"
                );
                // Menu, dialog and field names come from the language file.
                let used = ui_keys(&md);
                assert_eq!(
                    used,
                    ui_keys(&en),
                    "docs/help/{code}/{page}.md: the [[labels]] differ from English"
                );
                for k in used {
                    assert!(
                        keys.contains(&k),
                        "docs/help/{code}/{page}.md: [[{k}]] is not in lang/{code}.toml"
                    );
                }
            }
        }
    }
}
