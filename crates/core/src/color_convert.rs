//! Converting a drawing's colours between CMYK and RGB (#972): every
//! colour of the objects chosen (fills, gradient stops, mesh nodes,
//! outlines, text, table cells, effects' colours, shadows) or of the
//! whole drawing (with the pages' backgrounds and the symbols) goes to
//! the colour model asked for, through `Color::convert_to`, so the colour
//! engine's profiles (the drawing's colour management) do the conversion.
//! Registration colour never changes; spot colours stay unless asked, and
//! then become process colours of the model. Bitmaps' pixels are not
//! changed. See docs/behavior/colour-management.md.

use crate::document::ShapeKind;
use crate::{Color, Document, Shape, ShapeId};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// The colour model colours are converted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorTarget {
    Cmyk,
    Rgb,
}

impl ColorTarget {
    /// The model's name for `Color::convert_to`.
    pub fn model(self) -> &'static str {
        match self {
            ColorTarget::Cmyk => "CMYK",
            ColorTarget::Rgb => "RGB",
        }
    }
}

/// `c` in model `to`: registration colour as it is, a spot colour as it
/// is unless `spots`, a colour already in the model as it is.
pub fn converted(c: Color, to: ColorTarget, spots: bool) -> Color {
    match c {
        Color::Registration => c,
        Color::Spot { .. } if !spots => c,
        _ => c.convert_to(to.model()),
    }
}

/// Change every colour inside the JSON value `v` (a serialised object)
/// by `f`; the number changed.
fn walk(v: &mut serde_json::Value, f: &dyn Fn(Color) -> Color) -> usize {
    match v {
        serde_json::Value::Object(map) => {
            // A colour is serialised as an object tagged with its model;
            // inside a tagged enum (a solid fill) its fields share the
            // object with the enum's tag, which stays.
            if map.contains_key("model") {
                if let Ok(c) =
                    serde_json::from_value::<Color>(serde_json::Value::Object(map.clone()))
                {
                    let to = f(c);
                    if to == c {
                        return 0;
                    }
                    let (Ok(serde_json::Value::Object(old)), Ok(serde_json::Value::Object(new))) =
                        (serde_json::to_value(c), serde_json::to_value(to))
                    else {
                        return 0;
                    };
                    for k in old.keys() {
                        map.remove(k);
                    }
                    map.extend(new);
                    return 1;
                }
            }
            map.values_mut().map(|x| walk(x, f)).sum()
        }
        serde_json::Value::Array(a) => a.iter_mut().map(|x| walk(x, f)).sum(),
        _ => 0,
    }
}

/// Every colour of `x` changed by `f`, through its serialised form, so no
/// colour of any part is missed; `x` stays as it was when it does not
/// read back. The number changed.
fn convert_value<T: Serialize + DeserializeOwned>(x: &mut T, f: &dyn Fn(Color) -> Color) -> usize {
    let Ok(mut v) = serde_json::to_value(&*x) else {
        return 0;
    };
    let n = walk(&mut v, f);
    if n == 0 {
        return 0;
    }
    match serde_json::from_value(v) {
        Ok(new) => {
            *x = new;
            n
        }
        Err(_) => 0,
    }
}

/// Every colour of `s` and of the objects inside it changed by `f`; the
/// number changed. Groups' and clip frames' contents are converted one by
/// one, and a bitmap's pixels are left out of the serialised form.
pub fn convert_shape(s: &mut Shape, f: &dyn Fn(Color) -> Color) -> usize {
    let heavy = matches!(
        s.kind,
        ShapeKind::Group { .. } | ShapeKind::ClipFrame { .. } | ShapeKind::Bitmap { .. }
    );
    let saved = heavy.then(|| {
        std::mem::replace(
            &mut s.kind,
            ShapeKind::Group {
                children: Vec::new(),
            },
        )
    });
    let mut n = convert_value(s, f);
    if let Some(mut kind) = saved {
        match &mut kind {
            ShapeKind::Group { children } => {
                n += children
                    .iter_mut()
                    .map(|c| convert_shape(c, f))
                    .sum::<usize>();
            }
            ShapeKind::ClipFrame { frame, contents } => {
                n += convert_shape(frame, f);
                n += contents
                    .iter_mut()
                    .map(|c| convert_shape(c, f))
                    .sum::<usize>();
            }
            _ => {}
        }
        s.kind = kind;
    }
    n
}

/// Convert the colours of `shapes` (any depth) to `to`, or, with `shapes`
/// empty, of the whole drawing: every object on every layer, the pages'
/// backgrounds and the symbols. The number of colours changed.
pub fn convert_document(
    doc: &mut Document,
    shapes: &[ShapeId],
    to: ColorTarget,
    spots: bool,
) -> usize {
    let f = move |c: Color| converted(c, to, spots);
    let mut n = 0;
    if shapes.is_empty() {
        for l in doc.all_layers_mut() {
            for s in l.shapes.iter_mut() {
                n += convert_shape(s, &f);
            }
        }
        for p in doc.pages.iter_mut() {
            if let Some(bg) = &mut p.background {
                n += convert_value(bg, &f);
            }
        }
        for sym in doc.symbols.iter_mut() {
            n += convert_shape(&mut sym.shape, &f);
        }
        return n;
    }
    for id in shapes {
        if let Some(s) = doc.find_shape_mut(*id) {
            n += convert_shape(s, &f);
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Rect, Size};
    use crate::style::{Fill, Fountain, Stroke};

    fn rect(id: u64) -> Shape {
        Shape::new(
            ShapeId(id),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        )
    }

    fn is_model(c: Color, m: &str) -> bool {
        c.model_name() == m
    }

    #[test]
    fn every_colour_goes_to_the_model_asked_spots_and_registration_stay() {
        let mut doc = Document::new("t", Size::new(100.0, 100.0));
        let mut a = rect(1);
        a.fill = Fill::Fountain(Fountain::two(
            crate::style::FountainKind::Linear,
            Color::rgb8(255, 0, 0),
            Color::Hsb {
                h: 120.0,
                s: 1.0,
                b: 1.0,
            },
            0.0,
        ));
        a.stroke = Some(Stroke::new(Color::Registration, 0.2));
        let mut b = rect(2);
        let spot = Color::Spot {
            ink: 1,
            tint: 0.5,
            c: 0.0,
            m: 1.0,
            y: 0.0,
            k: 0.0,
        };
        b.fill = Fill::Solid(spot);
        b.stroke = Some(Stroke::new(Color::rgb8(0, 0, 255), 0.2));
        let mut g = Shape::new(ShapeId(3), ShapeKind::Group { children: vec![b] });
        g.fill = Fill::None;
        g.stroke = None;
        doc.pages[0].layers[0].shapes = vec![a, g];
        doc.pages[0].background = Some(Fill::Solid(Color::rgb8(250, 250, 240)));
        let n = convert_document(&mut doc, &[], ColorTarget::Cmyk, false);
        assert_eq!(n, 4, "two stops, the blue outline, the background");
        let shapes = &doc.pages[0].layers[0].shapes;
        let Fill::Fountain(f) = &shapes[0].fill else {
            panic!("fountain");
        };
        assert!(f.stops.iter().all(|s| is_model(s.color, "CMYK")));
        assert_eq!(
            shapes[0].stroke.as_ref().map(|s| s.color),
            Some(Color::Registration)
        );
        let ShapeKind::Group { children } = &shapes[1].kind else {
            panic!("group");
        };
        assert_eq!(children[0].fill, Fill::Solid(spot), "spots stay");
        assert!(children[0]
            .stroke
            .as_ref()
            .is_some_and(|s| is_model(s.color, "CMYK")));
        assert!(matches!(doc.pages[0].background, Some(Fill::Solid(c)) if is_model(c, "CMYK")));
        // Again: nothing left to change.
        assert_eq!(convert_document(&mut doc, &[], ColorTarget::Cmyk, false), 0);
        // Spots when asked, and back to RGB for the selected group only.
        assert_eq!(
            convert_document(&mut doc, &[ShapeId(3)], ColorTarget::Rgb, true),
            2
        );
        let shapes = &doc.pages[0].layers[0].shapes;
        let ShapeKind::Group { children } = &shapes[1].kind else {
            panic!("group");
        };
        assert!(matches!(children[0].fill, Fill::Solid(c) if is_model(c, "RGB")));
        let Fill::Fountain(f) = &shapes[0].fill else {
            panic!("fountain");
        };
        assert!(
            f.stops.iter().all(|s| is_model(s.color, "CMYK")),
            "not selected"
        );
    }

    #[test]
    fn colours_of_effects_and_text_convert_too() {
        let mut s = Shape::new(
            ShapeId(1),
            ShapeKind::Text {
                spans: vec![crate::TextSpan::new("x", "Sans", 12.0)],
                origin: crate::geometry::Point::new(0.0, 0.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        s.fill = Fill::Solid(Color::rgb8(10, 20, 30));
        s.stroke = None;
        s.shadow = Some(crate::document::Shadow {
            color: Color::rgb8(0, 0, 0),
            ..Default::default()
        });
        let before = serde_json::to_string(&s).unwrap_or_default();
        let n = convert_shape(&mut s, &|c| converted(c, ColorTarget::Cmyk, false));
        assert!(n >= 2, "the fill and the shadow's colour: {n}");
        let after = serde_json::to_string(&s).unwrap_or_default();
        assert_ne!(before, after);
        assert!(!after.contains("\"model\":\"rgb\""), "{after}");
    }
}
