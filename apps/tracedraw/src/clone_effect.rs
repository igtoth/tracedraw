//! Object > Clone Effect: a clone keeps following the source object's
//! drop shadow or transparency until the clone's own effect is edited.
//!
//! The link lives in the clone's object data (`effect.clone_of` =
//! `<source id>:<shadow|transparency>`), so it survives save and load. After
//! every command that changes a shadow, opacity or transparency, the
//! clones of the changed shapes are refreshed; a command that edits a clone
//! directly breaks its link first.

use crate::app::{App, EffectKind};
use crate::i18n::tr;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    live::Effect,
    Command, ShapeId,
};

const KEY: &str = "effect.clone_of";

fn kind_name(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::Shadow => "shadow",
        EffectKind::Transparency => "transparency",
    }
}

/// Parse `effect.clone_of` from object data.
pub fn clone_link(data: &[(String, String)]) -> Option<(ShapeId, EffectKind)> {
    let (_, v) = data.iter().find(|(k, _)| k == KEY)?;
    let (id, kind) = v.split_once(':')?;
    let id = ShapeId(id.parse().ok()?);
    let kind = match kind {
        "shadow" => EffectKind::Shadow,
        "transparency" => EffectKind::Transparency,
        _ => return None,
    };
    Some((id, kind))
}

fn walk<'a>(s: &'a Shape, out: &mut Vec<&'a Shape>) {
    out.push(s);
    match &s.kind {
        ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, out)),
        ShapeKind::ClipFrame { frame, contents } => {
            walk(frame, out);
            contents.iter().for_each(|c| walk(c, out));
        }
        _ => {}
    }
}

/// Which shapes a command touches for the purpose of effect cloning.
fn touched(cmd: &Command) -> Option<(Vec<ShapeId>, EffectKind)> {
    match cmd {
        Command::SetShadow { shapes, .. } => Some((shapes.clone(), EffectKind::Shadow)),
        Command::SetOpacity { shapes, .. } => Some((shapes.clone(), EffectKind::Transparency)),
        Command::SetEffects { shape, .. } => Some((vec![*shape], EffectKind::Transparency)),
        _ => None,
    }
}

impl App {
    /// Every (clone id, source id, kind) link in the document.
    pub fn effect_clones(&self) -> Vec<(ShapeId, ShapeId, EffectKind)> {
        let mut all = Vec::new();
        for l in self.doc().all_layers() {
            for s in &l.shapes {
                walk(s, &mut all);
            }
        }
        all.iter()
            .filter_map(|s| clone_link(&s.data).map(|(src, k)| (s.id, src, k)))
            .collect()
    }

    pub fn is_effect_clone(&self, id: ShapeId) -> bool {
        self.doc()
            .find_shape(id)
            .map(|s| clone_link(&s.data).is_some())
            .unwrap_or(false)
    }

    /// Object > Clone Effect > ... From: copy the effect now and remember the
    /// source so later changes follow.
    pub fn clone_effect_from(&mut self, source: ShapeId, kind: EffectKind) {
        let targets: Vec<ShapeId> = self
            .selection
            .iter()
            .copied()
            .filter(|s| *s != source)
            .collect();
        if targets.is_empty() {
            return;
        }
        self.copy_effect_from(source, kind);
        let mut cmds = Vec::new();
        for id in targets {
            let Some(s) = self.doc().find_shape(id) else {
                continue;
            };
            let mut data = s.data.clone();
            data.retain(|(k, _)| k != KEY);
            data.push((KEY.into(), format!("{}:{}", source.raw(), kind_name(kind))));
            cmds.push(Command::SetObjectData { shape: id, data });
        }
        let _ = self.engine.run_batch("Clone Effect", &cmds);
        self.status = tr("status.cloned_effect");
    }

    /// Before a user command runs: editing a clone's own effect breaks the
    /// link.
    pub fn break_clone_links_for(&mut self, cmd: &Command) {
        let Some((shapes, kind)) = touched(cmd) else {
            return;
        };
        let mut cmds = Vec::new();
        for id in shapes {
            let Some(s) = self.doc().find_shape(id) else {
                continue;
            };
            if clone_link(&s.data).map(|(_, k)| k) != Some(kind) {
                continue;
            }
            let mut data = s.data.clone();
            data.retain(|(k, _)| k != KEY);
            cmds.push(Command::SetObjectData { shape: id, data });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Clone Effect", &cmds);
        }
    }

    /// After a user command ran: push the new effect to the clones of the
    /// changed shapes. Runs on the engine directly so it never recurses.
    pub fn sync_effect_clones_for(&mut self, cmd: &Command) {
        let Some((changed, kind)) = touched(cmd) else {
            return;
        };
        let clones = self.effect_clones();
        for (clone, src, k) in clones {
            if k != kind || !changed.contains(&src) {
                continue;
            }
            let Some(s) = self.doc().find_shape(src).cloned() else {
                continue;
            };
            let cmds = match kind {
                EffectKind::Shadow => vec![Command::SetShadow {
                    shapes: vec![clone],
                    shadow: s.shadow,
                }],
                EffectKind::Transparency => {
                    let Some(c) = self.doc().find_shape(clone).cloned() else {
                        continue;
                    };
                    let mut effects: Vec<Effect> = c
                        .effects
                        .iter()
                        .filter(|e| !matches!(e, Effect::Transparency { .. }))
                        .cloned()
                        .collect();
                    if let Some(t) = s
                        .effects
                        .iter()
                        .find(|e| matches!(e, Effect::Transparency { .. }))
                    {
                        effects.push(t.clone());
                    }
                    vec![
                        Command::SetOpacity {
                            shapes: vec![clone],
                            opacity: s.opacity,
                        },
                        Command::SetEffects {
                            shape: clone,
                            effects,
                        },
                    ]
                }
            };
            let _ = self.engine.run_batch(cmd.label(), &cmds);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    fn rect(app: &mut App, x: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x, 10.0, x + 20.0, 30.0),
            radius: 0.0,
            corners: None,
        })
        .unwrap()
    }

    #[test]
    fn clone_follows_source_until_edited() {
        let mut app = App::headless();
        let src = rect(&mut app, 10.0);
        let clone = rect(&mut app, 50.0);
        app.run(Command::SetOpacity {
            shapes: vec![src],
            opacity: 0.5,
        });
        app.select(vec![clone]);
        app.clone_effect_from(src, EffectKind::Transparency);
        assert!(app.is_effect_clone(clone));
        assert_eq!(app.doc().find_shape(clone).unwrap().opacity, 0.5);
        // The source changes, the clone follows.
        app.run(Command::SetOpacity {
            shapes: vec![src],
            opacity: 0.2,
        });
        assert_eq!(app.doc().find_shape(clone).unwrap().opacity, 0.2);
        // Editing the clone breaks the link.
        app.run(Command::SetOpacity {
            shapes: vec![clone],
            opacity: 0.9,
        });
        assert!(!app.is_effect_clone(clone));
        app.run(Command::SetOpacity {
            shapes: vec![src],
            opacity: 0.1,
        });
        assert_eq!(app.doc().find_shape(clone).unwrap().opacity, 0.9);
    }
}
