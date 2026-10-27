//! Blends that open live again (#986).
//!
//! The writer draws a blend as one plain group of its start object and its
//! steps as curves (#946), which any reader shows as it looked. A file of
//! version 14 or later (a ZIP package) also carries a member of its own,
//! [`MEMBER`], that names each such group by its static id (`usdn`) and
//! holds the blend's settings. The reader turns those groups back into
//! their start object with its blend to the end object, so a blend saved
//! by TraceDraw comes back as a blend, not as loose steps. Other readers
//! leave the member alone; a file without it, or one whose groups no
//! longer match it, opens as before.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracedraw_core::{document::Shape, live::Effect, Document, ShapeId, ShapeKind};

/// The package member holding the blends.
pub const MEMBER: &str = "tracedraw/blends.json";

/// The most the member may inflate to when read: far more than any
/// drawing's blends take.
pub(crate) const MAX_MEMBER: u64 = 16 << 20;

/// The layout of [`MEMBER`]; a reader skips a member of a later one.
const FORMAT: u32 = 1;

/// One blend as the file holds it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct KeptBlend {
    /// The static id (`usdn`) of the group the blend is drawn as.
    pub group: u32,
    /// How many objects that group holds, and which of them (bottom to
    /// top) is the start object.
    pub count: usize,
    pub start: usize,
    /// The static id of the end object, when it is an object of the
    /// drawing; the blend's own copy of it is then left empty here.
    #[serde(default)]
    pub end: Option<u32>,
    pub blend: Effect,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Member {
    format: u32,
    blends: Vec<KeptBlend>,
}

/// A blend of the drawing being written: the group drawn for it and its
/// end object, by shape, until the end's static id is known.
#[derive(Debug, Clone)]
pub(crate) struct Written {
    pub group: u32,
    pub count: usize,
    pub start: usize,
    pub end: Option<ShapeId>,
    pub blend: Effect,
}

/// The member's bytes for `written`, the static ids of the objects
/// written by shape. A blend whose end object was not written (hidden,
/// or on no page) is left out: it stays its drawn group.
pub(crate) fn encode(written: &[Written], static_ids: &HashMap<ShapeId, u32>) -> Option<Vec<u8>> {
    let blends: Vec<KeptBlend> = written
        .iter()
        .filter_map(|w| {
            let end = match w.end {
                Some(id) => Some(*static_ids.get(&id)?),
                None => None,
            };
            let mut blend = w.blend.clone();
            if end.is_some() {
                if let Effect::Blend { end: copy, .. } = &mut blend {
                    // The end object itself is read back; its copy here
                    // would only make the member as large as the drawing.
                    **copy = Shape::new(
                        copy.id,
                        ShapeKind::Group {
                            children: Vec::new(),
                        },
                    );
                }
            }
            Some(KeptBlend {
                group: w.group,
                count: w.count,
                start: w.start,
                end,
                blend,
            })
        })
        .collect();
    if blends.is_empty() {
        return None;
    }
    serde_json::to_vec(&Member {
        format: FORMAT,
        blends,
    })
    .ok()
}

/// The blends of a member's bytes; none when it is damaged or of a later
/// layout (with a warning in the log).
pub(crate) fn decode(bytes: &[u8]) -> Vec<KeptBlend> {
    match serde_json::from_slice::<Member>(bytes) {
        Ok(m) if m.format == FORMAT => m
            .blends
            .into_iter()
            .filter(|b| matches!(b.blend, Effect::Blend { .. }))
            .collect(),
        Ok(m) => {
            log::warn!("{MEMBER} of layout {}; its blends open as drawn", m.format);
            Vec::new()
        }
        Err(e) => {
            log::warn!("{MEMBER} unreadable ({e}); its blends open as drawn");
            Vec::new()
        }
    }
}

/// Turn the groups `kept` names back into blends: each group, found by
/// the shape its static id was read as (`ids`), gives way to its start
/// object (which takes the group's id) with the blend to its end object.
/// A group that does not hold what the member says (another program
/// changed the drawing) stays as it is. Returns how many blends came back.
pub(crate) fn restore(
    doc: &mut Document,
    kept: &[KeptBlend],
    ids: &HashMap<u32, ShapeId>,
) -> usize {
    // First every group gives way to its start object, so that a blend
    // whose end is itself the start of another blend finds that object.
    let mut restored: Vec<(ShapeId, Effect, Option<ShapeId>)> = Vec::new();
    for k in kept {
        let Some(&group) = ids.get(&k.group) else {
            continue;
        };
        let end = match k.end {
            Some(u) => match ids.get(&u) {
                Some(&id) => Some(id),
                None => continue,
            },
            None => None,
        };
        let Some(slot) = find_mut(doc, group) else {
            continue;
        };
        let ShapeKind::Group { children } = &slot.kind else {
            continue;
        };
        if children.len() != k.count || k.start >= k.count {
            continue;
        }
        let mut start = children[k.start].clone();
        if !start.effects.is_empty() {
            continue;
        }
        start.id = group;
        *slot = start;
        restored.push((group, k.blend.clone(), end));
    }
    let mut n = 0;
    for (id, mut blend, end) in restored {
        if let Some(end) = end {
            let Some(copy) = find(doc, end).map(|s| {
                let mut e = s.clone();
                e.effects.clear();
                e
            }) else {
                continue;
            };
            if let Effect::Blend { end: e, end_id, .. } = &mut blend {
                **e = copy;
                *end_id = Some(end);
            }
        }
        if let Some(s) = find_mut(doc, id) {
            s.effects.push(blend);
            n += 1;
        }
    }
    n
}

fn find(doc: &Document, id: ShapeId) -> Option<&Shape> {
    fn walk(shapes: &[Shape], id: ShapeId) -> Option<&Shape> {
        for s in shapes {
            if s.id == id {
                return Some(s);
            }
            if let ShapeKind::Group { children } = &s.kind {
                if let Some(f) = walk(children, id) {
                    return Some(f);
                }
            }
        }
        None
    }
    doc.pages
        .iter()
        .flat_map(|p| &p.layers)
        .find_map(|l| walk(&l.shapes, id))
}

fn find_mut(doc: &mut Document, id: ShapeId) -> Option<&mut Shape> {
    fn walk(shapes: &mut [Shape], id: ShapeId) -> Option<&mut Shape> {
        for s in shapes {
            if s.id == id {
                return Some(s);
            }
            if let ShapeKind::Group { children } = &mut s.kind {
                if let Some(f) = walk(children, id) {
                    return Some(f);
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
