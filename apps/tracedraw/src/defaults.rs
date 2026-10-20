//! Default properties: the fill and outline new objects get, by kind
//! (graphics, artistic text, paragraph text), kept in the drawing
//! (`Document::defaults`). A fill or outline chosen with nothing selected
//! changes them, for the kinds the user picks.

use crate::app::App;
use crate::i18n::{tr, trf};
use serde::{Deserialize, Serialize};
use tracedraw_core::{
    style::{Fill, Stroke},
    Color, Command, DefaultKind, DefaultProperties,
};

/// A change made with nothing selected.
#[derive(Debug, Clone, PartialEq)]
pub enum DefaultChange {
    Fill(Fill),
    /// `None` takes the outline away.
    OutlineColor(Option<Color>),
    /// Only kinds that have an outline change.
    OutlineWidth(f64),
    Outline(Option<Stroke>),
}

impl DefaultChange {
    pub fn apply(&self, p: &mut DefaultProperties) {
        match self {
            DefaultChange::Fill(f) => p.fill = f.clone(),
            DefaultChange::OutlineColor(None) => p.stroke = None,
            DefaultChange::OutlineColor(Some(c)) => {
                let mut s = p.stroke.clone().unwrap_or_default();
                s.color = *c;
                p.stroke = Some(s);
            }
            DefaultChange::OutlineWidth(w) => {
                if let Some(s) = &mut p.stroke {
                    s.width = *w;
                }
            }
            DefaultChange::Outline(s) => p.stroke = s.clone(),
        }
    }
}

/// How changes with nothing selected are handled; kept in the preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DefaultsPrefs {
    /// Ask which kinds a change applies to (else the kinds below).
    pub ask: bool,
    pub graphic: bool,
    pub artistic_text: bool,
    pub paragraph_text: bool,
}

impl Default for DefaultsPrefs {
    fn default() -> Self {
        DefaultsPrefs {
            ask: true,
            graphic: true,
            artistic_text: false,
            paragraph_text: false,
        }
    }
}

impl DefaultsPrefs {
    pub fn kinds(&self) -> [bool; 3] {
        [self.graphic, self.artistic_text, self.paragraph_text]
    }

    pub fn set_kinds(&mut self, kinds: [bool; 3]) {
        [self.graphic, self.artistic_text, self.paragraph_text] = kinds;
    }
}

/// The i18n key naming a kind.
pub fn kind_key(kind: DefaultKind) -> &'static str {
    match kind {
        DefaultKind::Graphic => "defaults.graphic",
        DefaultKind::ArtisticText => "defaults.artistic_text",
        DefaultKind::ParagraphText => "defaults.paragraph_text",
    }
}

impl App {
    /// The fill new graphic objects get.
    pub fn default_fill(&self) -> Fill {
        self.doc().defaults.graphic.fill.clone()
    }

    /// The outline new graphic objects get.
    pub fn default_stroke(&self) -> Option<Stroke> {
        self.doc().defaults.graphic.stroke.clone()
    }

    /// A fill or outline chosen with nothing selected: ask which kinds of
    /// new objects take it, or use the kinds picked before.
    pub fn request_default_change(&mut self, change: DefaultChange) {
        let prefs = self.settings.defaults;
        if prefs.ask {
            self.dialog =
                crate::ui::dialogs::Dialog::ChangeDefaults(crate::ui::defaults_dialog::State {
                    change,
                    kinds: prefs.kinds(),
                    dont_ask: false,
                });
        } else {
            self.change_defaults(&change, prefs.kinds());
        }
    }

    /// Apply `change` to the defaults of the kinds marked in `kinds`
    /// (graphic, artistic text, paragraph text), as one undo step.
    pub fn change_defaults(&mut self, change: &DefaultChange, kinds: [bool; 3]) {
        let mut defaults = self.doc().defaults.clone();
        let mut names = Vec::new();
        for (kind, on) in DefaultKind::ALL.into_iter().zip(kinds) {
            if on {
                change.apply(defaults.get_mut(kind));
                names.push(tr(kind_key(kind)));
            }
        }
        if names.is_empty() || defaults == self.doc().defaults {
            return;
        }
        self.run(Command::SetObjectDefaults { defaults });
        self.status = trf("status.defaults_changed", &[("kinds", &names.join(", "))]);
    }

    /// The tabs, columns, bullets and drop cap new paragraph text gets.
    pub fn set_default_paragraph(&mut self, para: tracedraw_core::ParagraphStyle) {
        let mut defaults = self.doc().defaults.clone();
        defaults.paragraph = para;
        if defaults != self.doc().defaults {
            self.run(Command::SetObjectDefaults { defaults });
        }
    }

    /// Change one kind's defaults at once (the Properties and Object
    /// Styles panels, patterns made into the default fill).
    pub fn set_default_properties(
        &mut self,
        kind: DefaultKind,
        edit: impl FnOnce(&mut DefaultProperties),
    ) {
        let mut defaults = self.doc().defaults.clone();
        edit(defaults.get_mut(kind));
        if defaults != self.doc().defaults {
            self.run(Command::SetObjectDefaults { defaults });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{geometry::Rect, ShapeKind};

    #[test]
    fn changes_reach_the_kinds_picked_and_new_objects() {
        let mut app = App::headless();
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        // Nothing selected: the question comes first.
        app.selection.clear();
        app.apply_fill(red.clone());
        assert!(matches!(
            app.dialog,
            crate::ui::dialogs::Dialog::ChangeDefaults(_)
        ));
        assert_eq!(app.default_fill(), Fill::None);
        // Graphics and paragraph text, not artistic text.
        app.change_defaults(&DefaultChange::Fill(red.clone()), [true, false, true]);
        let d = &app.doc().defaults;
        assert_eq!(
            (d.graphic.fill.clone(), d.paragraph_text.fill.clone()),
            (red.clone(), red.clone())
        );
        // Artistic text keeps its black (in the drawing's colour mode).
        let black = app.doc().metadata.primary_color_mode.black();
        assert_eq!(d.artistic_text.fill, Fill::Solid(black));
        // One undo step.
        app.undo();
        assert_eq!(app.default_fill(), Fill::None);
        app.redo();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        assert_eq!(
            app.doc().shape(id).map(|(_, s)| s.fill.clone()).ok(),
            Some(red)
        );
        // Without the question, the kinds picked before.
        app.settings.defaults = DefaultsPrefs {
            ask: false,
            graphic: false,
            artistic_text: true,
            paragraph_text: false,
        };
        app.dialog = crate::ui::dialogs::Dialog::None;
        app.selection.clear();
        app.apply_outline_color(Some(Color::rgb8(0, 0, 255)));
        assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::None));
        let d = &app.doc().defaults;
        assert_eq!(
            d.artistic_text.stroke.as_ref().map(|s| s.color),
            Some(Color::rgb8(0, 0, 255))
        );
        assert_eq!(d.graphic.stroke.as_ref().map(|s| s.width), Some(0.2));
        // A width only changes outlines that are there.
        let mut p = DefaultProperties {
            fill: Fill::None,
            stroke: None,
        };
        DefaultChange::OutlineWidth(1.0).apply(&mut p);
        assert!(p.stroke.is_none());
        DefaultChange::OutlineColor(None).apply(&mut p);
        assert!(p.stroke.is_none());
    }

    #[test]
    fn new_text_is_black_in_the_drawings_colour_mode() {
        use tracedraw_core::document::{ObjectDefaults, PrimaryColorMode};
        // A CMYK drawing: text and an outline turned on are CMYK black.
        let cmyk = ObjectDefaults::for_mode(PrimaryColorMode::Cmyk);
        let k100 = Color::cmyk_pct(0.0, 0.0, 0.0, 100.0);
        assert_eq!(cmyk.artistic_text.fill, Fill::Solid(k100));
        assert_eq!(cmyk.paragraph_text.fill, Fill::Solid(k100));
        assert_eq!(ObjectDefaults::default(), cmyk);
        // An RGB drawing keeps RGB black, as before.
        let rgb = ObjectDefaults::for_mode(PrimaryColorMode::Rgb);
        assert_eq!(rgb.artistic_text.fill, Fill::Solid(Color::BLACK));
        // Both keep the CMYK black outline of graphics.
        assert_eq!(rgb.graphic, cmyk.graphic);
        // New drawings get the defaults of the mode picked for them.
        let mut app = App::headless();
        for (rgb, want) in [(false, k100), (true, Color::BLACK)] {
            app.settings.new_document.values.rgb = rgb;
            app.new_document();
            assert_eq!(
                app.doc().defaults.artistic_text.fill,
                Fill::Solid(want),
                "rgb {rgb}"
            );
            let lib = crate::ui::line_style::OutlineLib::of(&app);
            assert_eq!(lib.black, want, "the outline turned on, rgb {rgb}");
        }
        // Defaults set by the user stay as they were set.
        let red = Fill::Solid(Color::rgb8(200, 0, 0));
        app.settings.document_defaults.objects.artistic_text.fill = red.clone();
        app.settings.new_document.values.rgb = false;
        app.new_document();
        assert_eq!(app.doc().defaults.artistic_text.fill, red);
    }

    #[test]
    fn paragraph_layout_with_nothing_selected_reaches_new_paragraph_text() {
        let mut app = App::headless();
        app.selection.clear();
        let mut para = app.doc().defaults.paragraph.clone();
        para.columns = 2;
        para.bullets = true;
        app.set_default_paragraph(para);
        app.start_text(
            tracedraw_core::geometry::Point::new(10.0, 10.0),
            Some(tracedraw_core::geometry::Size::new(50.0, 30.0)),
        );
        let id = app.selection[0];
        let Ok((_, s)) = app.doc().shape(id) else {
            panic!("no text");
        };
        let ShapeKind::Text { para, .. } = &s.kind else {
            panic!("not text");
        };
        assert_eq!((para.columns, para.bullets), (2, true));
        assert_eq!(para.hyphenate, app.text_hyphenation);
    }
}
