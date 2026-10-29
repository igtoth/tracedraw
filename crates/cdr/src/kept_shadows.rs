//! Drop shadows that open live again (#1193).
//!
//! No sample file of the program that saved the samples holds a drop
//! shadow, so how its own files store one is not known. The writer draws
//! a drop shadow as what it shows: a group of the object and, beneath it,
//! a bitmap of the shadow alone with its see-through pixels (the picture
//! Break Drop Shadow Apart makes), which any reader shows as it looked. A
//! file of version 14 or later (a ZIP package) also carries a member of
//! its own, [`MEMBER`], that names each such group by its static id
//! (`usdn`) and holds the shadow's settings. The reader turns those groups
//! back into their object with its live shadow, so a shadow saved by
//! TraceDraw comes back as a shadow, not as a picture. Other readers
//! leave the member alone; a file without it, or one whose groups no
//! longer match it, opens as drawn.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracedraw_core::{document::Shape, Document, Shadow, ShapeId, ShapeKind};

/// The package member holding the shadows.
pub const MEMBER: &str = "tracedraw/shadows.json";

/// The most the member may inflate to when read: far more than any
/// drawing's shadows take.
pub(crate) const MAX_MEMBER: u64 = 16 << 20;

/// The layout of [`MEMBER`]; a reader skips a member of a later one.
const FORMAT: u32 = 1;

/// One shadow as the file holds it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct KeptShadow {
    /// The static id (`usdn`) of the group the shadow is drawn as.
    pub group: u32,
    /// How many objects that group holds: the shadow's bitmap at the
    /// bottom, then the object (one object, or the several it was drawn
    /// as, which come back as a group).
    pub count: usize,
    pub shadow: Shadow,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Member {
    format: u32,
    shadows: Vec<KeptShadow>,
}

/// The member's bytes for `kept`; `None` when there is nothing to keep.
pub(crate) fn encode(kept: &[KeptShadow]) -> Option<Vec<u8>> {
    if kept.is_empty() {
        return None;
    }
    serde_json::to_vec(&Member {
        format: FORMAT,
        shadows: kept.to_vec(),
    })
    .ok()
}

/// The shadows of a member's bytes; none when it is damaged or of a later
/// layout (with a warning in the log).
pub(crate) fn decode(bytes: &[u8]) -> Vec<KeptShadow> {
    match serde_json::from_slice::<Member>(bytes) {
        Ok(m) if m.format == FORMAT => m.shadows,
        Ok(m) => {
            log::warn!("{MEMBER} of layout {}; its shadows open as drawn", m.format);
            Vec::new()
        }
        Err(e) => {
            log::warn!("{MEMBER} unreadable ({e}); its shadows open as drawn");
            Vec::new()
        }
    }
}

/// Turn the groups `kept` names back into objects with shadows: each
/// group, found by the shape its static id was read as (`ids`), gives way
/// to the object above its shadow bitmap (which takes the group's id)
/// with the shadow's settings. A group that does not hold what the member
/// says (another program changed the drawing) stays as it is. Returns
/// how many shadows came back.
pub(crate) fn restore(
    doc: &mut Document,
    kept: &[KeptShadow],
    ids: &HashMap<u32, ShapeId>,
) -> usize {
    let mut n = 0;
    for k in kept {
        let Some(&group) = ids.get(&k.group) else {
            continue;
        };
        let Some(slot) = find_mut(doc, group) else {
            continue;
        };
        let ShapeKind::Group { children } = &slot.kind else {
            continue;
        };
        if children.len() != k.count || k.count < 2 {
            continue;
        }
        if !matches!(children[0].kind, ShapeKind::Bitmap { .. }) {
            continue;
        }
        let mut object = if k.count == 2 {
            children[1].clone()
        } else {
            Shape::new(
                group,
                ShapeKind::Group {
                    children: children[1..].to_vec(),
                },
            )
        };
        if object.shadow.is_some() {
            continue;
        }
        object.id = group;
        object.shadow = Some(k.shadow);
        *slot = object;
        n += 1;
    }
    n
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
