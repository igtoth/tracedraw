//! Live effects and drop shadows that open live again (#1031).
//!
//! The writer draws an object with live effects (a distortion, a contour,
//! an envelope, an extrusion ...) as what the effects draw, loose objects
//! any reader shows as they looked, and an object's drop shadow as a
//! picture of it beneath the object. A file of version 14 or later (a ZIP
//! package) also carries a member of its own, [`MEMBER`], that names those
//! drawn objects by their static ids (`usdn`) and holds the object itself,
//! effects and shadow included. The reader puts the object back in their
//! place, so a distorted rectangle saved by TraceDraw comes back as a
//! rectangle with its distortion, adjustable again. Other readers leave
//! the member alone; a file without it, or one whose drawn objects no
//! longer match it (another program moved or changed them), opens as
//! drawn.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracedraw_core::{document::Shape, geometry::Rect, live::Effect, Document, ShapeId, ShapeKind};

/// The package member holding the objects.
pub const MEMBER: &str = "tracedraw/effects.json";

/// The most the member may inflate to when read: an object's source can
/// hold a picture, so more than the blends' member.
pub(crate) const MAX_MEMBER: u64 = 64 << 20;

/// The layout of [`MEMBER`]; a reader skips a member of a later one.
const FORMAT: u32 = 1;

/// How far (mm) the drawn object's box may be from the one written
/// before the object counts as changed by another program.
const BOX_TOLERANCE: f64 = 0.1;

/// One object as the file holds it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct KeptEffect {
    /// The static ids of the objects the effect was drawn as, bottom to
    /// top, one after the other in their list.
    pub objects: Vec<u32>,
    /// Which of them the object itself was drawn as (the others are what
    /// its effects and shadow draw).
    pub main: usize,
    /// The page box of that object as written (left, bottom, right, top),
    /// to tell a drawing changed elsewhere.
    pub bounds: [f64; 4],
    /// The object with its effects and shadow.
    pub source: Shape,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Member {
    format: u32,
    effects: Vec<KeptEffect>,
}

/// An object of the drawing being written: the static ids of what it was
/// drawn as and the object itself.
#[derive(Debug, Clone)]
pub(crate) struct Written {
    pub objects: Vec<u32>,
    pub main: usize,
    pub bounds: Rect,
    pub source: Shape,
}

/// The member's bytes for `written`; none when nothing was written so.
pub(crate) fn encode(written: &[Written]) -> Option<Vec<u8>> {
    if written.is_empty() {
        return None;
    }
    let effects: Vec<KeptEffect> = written
        .iter()
        .map(|w| KeptEffect {
            objects: w.objects.clone(),
            main: w.main,
            bounds: [w.bounds.x0, w.bounds.y0, w.bounds.x1, w.bounds.y1],
            source: w.source.clone(),
        })
        .collect();
    serde_json::to_vec(&Member {
        format: FORMAT,
        effects,
    })
    .ok()
}

/// The objects of a member's bytes; none when it is damaged or of a
/// later layout (with a warning in the log).
pub(crate) fn decode(bytes: &[u8]) -> Vec<KeptEffect> {
    match serde_json::from_slice::<Member>(bytes) {
        Ok(m) if m.format == FORMAT => m
            .effects
            .into_iter()
            .filter(|k| {
                !k.objects.is_empty()
                    && k.main < k.objects.len()
                    && k.bounds.iter().all(|v| v.is_finite())
                    && (!k.source.effects.is_empty() || k.source.shadow.is_some())
            })
            .collect(),
        Ok(m) => {
            log::warn!("{MEMBER} of layout {}; its objects open as drawn", m.format);
            Vec::new()
        }
        Err(e) => {
            log::warn!("{MEMBER} unreadable ({e}); its objects open as drawn");
            Vec::new()
        }
    }
}

/// Put the objects `kept` holds back in the place of what they were
/// drawn as: each is found by the shape its static id was read as
/// (`ids`), and takes that shape's id; the other objects of its entry
/// are removed. An entry whose objects are not all there, not one after
/// the other in one list, or whose drawn box moved (another program
/// changed the drawing) stays as drawn. Returns how many came back.
pub(crate) fn restore(
    doc: &mut Document,
    kept: &[KeptEffect],
    ids: &HashMap<u32, ShapeId>,
) -> usize {
    let mut n = 0;
    for k in kept {
        let Some(shapes) = k
            .objects
            .iter()
            .map(|u| ids.get(u).copied())
            .collect::<Option<Vec<ShapeId>>>()
        else {
            continue;
        };
        let Some(&main) = shapes.get(k.main) else {
            continue;
        };
        let mut source = k.source.clone();
        source.id = main;
        renumber(&mut source, doc.ids_mut());
        let Some(list) = find_list_mut(doc, main) else {
            continue;
        };
        let Some(at) = list.iter().position(|s| s.id == main) else {
            continue;
        };
        let Some(first) = at.checked_sub(k.main) else {
            continue;
        };
        let end = first + shapes.len();
        if end > list.len()
            || list[first..end]
                .iter()
                .map(|s| s.id)
                .ne(shapes.iter().copied())
        {
            continue;
        }
        let drawn = list[at].bounds();
        let written = Rect::new(k.bounds[0], k.bounds[1], k.bounds[2], k.bounds[3]);
        if !near(drawn, written) {
            continue;
        }
        list.splice(first..end, std::iter::once(source));
        n += 1;
    }
    n
}

fn near(a: Rect, b: Rect) -> bool {
    let tol = BOX_TOLERANCE + 0.005 * b.width().abs().max(b.height().abs());
    (a.x0 - b.x0).abs() <= tol
        && (a.y0 - b.y0).abs() <= tol
        && (a.x1 - b.x1).abs() <= tol
        && (a.y1 - b.y1).abs() <= tol
}

/// Fresh ids for the shapes `s` holds (a group's children, a clip
/// frame's frame and contents, a blend's end copy): the file's ids are
/// the drawing's that was saved, and the read drawing has given its own
/// out already. `s` keeps its id.
fn renumber(s: &mut Shape, ids: &mut tracedraw_core::id::IdSource) {
    match &mut s.kind {
        ShapeKind::Group { children } => {
            for c in children {
                c.id = ids.shape();
                renumber(c, ids);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            frame.id = ids.shape();
            renumber(frame, ids);
            for c in contents {
                c.id = ids.shape();
                renumber(c, ids);
            }
        }
        _ => {}
    }
    for e in &mut s.effects {
        if let Effect::Blend { end, .. } = e {
            end.id = ids.shape();
            renumber(end, ids);
        }
    }
}

/// The list (a layer's shapes or a group's children) holding the shape
/// `id`.
fn find_list_mut(doc: &mut Document, id: ShapeId) -> Option<&mut Vec<Shape>> {
    fn walk(shapes: &mut Vec<Shape>, id: ShapeId) -> Option<&mut Vec<Shape>> {
        if shapes.iter().any(|s| s.id == id) {
            return Some(shapes);
        }
        for s in shapes {
            if let ShapeKind::Group { children } = &mut s.kind {
                if let Some(l) = walk(children, id) {
                    return Some(l);
                }
            }
        }
        None
    }
    doc.pages
        .iter_mut()
        .flat_map(|p| &mut p.layers)
        .find_map(|l| walk(&mut l.shapes, id))
}
