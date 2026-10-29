//! Interactive Fill tool: the fountain's nodes on the canvas and the
//! property bar that edits the selected object's fill in place.
//!
//! Every fountain but a shaped one shows three nodes, placed freely as
//! the reference program places them (#330): the start (a square in the
//! first colour), the end (a square in the last colour, an arrow on the
//! line) and the skew handle (a round node on a thin line from the
//! start). Dragging the start moves it with the skew handle (the centre
//! of an elliptical, conical or square fill moves the whole fill); the end
//! turns and stretches the axis, the skew handle turning with it; the skew
//! handle tilts the bands (and gives an ellipse or a square its other
//! half axis). The nodes are kept relative to the object's box (see
//! `Fountain::handles`).
//!
//! Each stop is a colour node on the line from the start to the end: a
//! double click on the line adds one, on a node removes it, and a drag
//! moves it between its neighbours. A bar across the line between two
//! nodes is the midpoint slider of their blend. A click picks a node; the
//! node bar under it edits its colour and transparency (see `node_bar`).

use crate::app::App;
use tracedraw_core::{
    geometry::{Point, Rect, Vec2},
    style::{Fountain, FountainHandles, FountainKind},
    Command, Fill, ShapeId,
};

/// Which fountain node a drag holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FountainHandle {
    Start,
    End,
    Skew,
    /// A colour node between the start and the end (index into the
    /// stops, never the first or the last).
    Stop(usize),
    /// The midpoint slider between stop `i` and the next.
    Mid(usize),
}

/// Screen pixels within which a press takes a node or the line.
const PICK_PX: f64 = 6.0;

/// The point at `t` (0 at the start, 1 at the end) on the fill's line.
pub fn on_line(s: Point, e: Point, t: f64) -> Point {
    s + (e - s) * t
}

/// Where `p` falls along the line from `s` to `e` (0 at the start, 1 at
/// the end; outside 0..1 beyond them).
pub fn line_param(s: Point, e: Point, p: Point) -> f64 {
    let d = e - s;
    let l2 = d.hypot2();
    if l2 < 1e-12 {
        0.0
    } else {
        (p - s).dot(d) / l2
    }
}

/// Where each midpoint slider sits on the line (as `t`), one per pair of
/// stops apart from each other.
pub fn midpoint_params(f: &Fountain) -> Vec<(usize, f64)> {
    f.stops
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w[1].pos - w[0].pos > 1e-6)
        .map(|(i, w)| (i, w[0].pos + (w[1].pos - w[0].pos) * w[0].mid))
        .collect()
}

/// The nodes of a fountain on the object's page bounds: start, end and
/// skew handle; none for a shaped fountain (it follows the outline).
pub fn fountain_handles(f: &Fountain, b: Rect) -> Vec<(FountainHandle, Point)> {
    if f.kind == FountainKind::Shaped {
        return Vec::new();
    }
    let h = f.handles.unwrap_or_else(|| f.default_handles(b));
    vec![
        (FountainHandle::Start, Fountain::rel_to_page(b, h.start)),
        (FountainHandle::End, Fountain::rel_to_page(b, h.end)),
        (FountainHandle::Skew, Fountain::rel_to_page(b, h.skew)),
    ]
}

/// The nodes `h` (page points `s`, `e`, `k`) after `handle` is dragged to
/// `p`: the start carries the skew handle (and, with a centre, the end
/// too); the end turns the skew handle by as much as the axis turned.
pub fn moved_nodes(
    kind: FountainKind,
    (s, e, k): (Point, Point, Point),
    handle: FountainHandle,
    p: Point,
) -> (Point, Point, Point) {
    match handle {
        FountainHandle::Start => {
            let d = p - s;
            if kind == FountainKind::Linear {
                (p, e, k + d)
            } else {
                (p, e + d, k + d)
            }
        }
        FountainHandle::End => {
            let (old, new) = (e - s, p - s);
            let turn = if old.hypot() > 1e-9 && new.hypot() > 1e-9 {
                new.y.atan2(new.x) - old.y.atan2(old.x)
            } else {
                0.0
            };
            let kv = k - s;
            let (c, sn) = (turn.cos(), turn.sin());
            let kv = Vec2::new(kv.x * c - kv.y * sn, kv.x * sn + kv.y * c);
            (s, p, s + kv)
        }
        FountainHandle::Skew => (s, e, p),
        FountainHandle::Stop(_) | FountainHandle::Mid(_) => (s, e, k),
    }
}

impl App {
    /// The selected object's fountain, with its id and bounds (those the
    /// renderer places the fill on).
    pub fn selected_fountain(&self) -> Option<(ShapeId, Fountain, Rect)> {
        use tracedraw_core::geometry::Shape as _;
        let s = self.selected_shapes().into_iter().next()?;
        let b = s.page_path().bounding_box();
        if self.tool == crate::tools::Tool::Transparency {
            // The Transparency tool edits the gradient of the mask with the
            // same nodes: start, end, skew, the nodes between and the
            // midpoint sliders (#448).
            return s.effects.iter().find_map(|e| match e {
                tracedraw_core::live::Effect::Transparency {
                    mask: Fill::Fountain(f),
                    ..
                } => Some((s.id, f.clone(), b)),
                _ => None,
            });
        }
        match &s.fill {
            Fill::Fountain(f) => Some((s.id, f.clone(), b)),
            _ => None,
        }
    }

    /// The node under a page point, within 6 screen pixels: the start,
    /// the end and the skew handle first, then the colour nodes on the
    /// line, then the midpoint sliders.
    pub fn fountain_handle_at(&self, p: Point) -> Option<FountainHandle> {
        let (_, f, b) = self.selected_fountain()?;
        let tol = PICK_PX / self.view.zoom as f64;
        let hs = fountain_handles(&f, b);
        let nearest = |pts: Vec<(FountainHandle, Point)>| {
            pts.into_iter()
                .filter(|(_, hp)| (*hp - p).hypot() <= tol)
                .min_by(|a, b| (a.1 - p).hypot().total_cmp(&(b.1 - p).hypot()))
                .map(|(h, _)| h)
        };
        if let Some(h) = nearest(hs.clone()) {
            return Some(h);
        }
        let [(_, s), (_, e), _] = hs.as_slice() else {
            return None;
        };
        let n = f.stops.len();
        let stops = f
            .stops
            .iter()
            .enumerate()
            .skip(1)
            .take(n.saturating_sub(2))
            .map(|(i, st)| (FountainHandle::Stop(i), on_line(*s, *e, st.pos)))
            .collect();
        if let Some(h) = nearest(stops) {
            return Some(h);
        }
        nearest(
            midpoint_params(&f)
                .into_iter()
                .map(|(i, t)| (FountainHandle::Mid(i), on_line(*s, *e, t)))
                .collect(),
        )
    }

    /// Where a page point falls on the selected fill's line (0 at the
    /// start, 1 at the end), within 6 screen pixels of it.
    pub fn fountain_line_at(&self, p: Point) -> Option<f64> {
        let (_, f, b) = self.selected_fountain()?;
        let hs = fountain_handles(&f, b);
        let [(_, s), (_, e), _] = hs.as_slice() else {
            return None;
        };
        let t = line_param(*s, *e, p);
        let tol = PICK_PX / self.view.zoom as f64;
        ((0.0..=1.0).contains(&t) && (on_line(*s, *e, t) - p).hypot() <= tol).then_some(t)
    }

    /// The stop a node stands for: the start the first, the end the last.
    pub fn fountain_stop_of(&self, h: FountainHandle) -> Option<usize> {
        let (_, f, _) = self.selected_fountain()?;
        match h {
            FountainHandle::Start => Some(0),
            FountainHandle::End => Some(f.stops.len().saturating_sub(1)),
            FountainHandle::Stop(i) => Some(i),
            _ => None,
        }
    }

    /// The selected colour node of the selected fill, when it has one.
    pub fn selected_fill_node(&self) -> Option<(Fountain, usize)> {
        let (_, f, _) = self.selected_fountain()?;
        let i = self.fill_node.filter(|i| *i < f.stops.len())?;
        Some((f, i))
    }

    /// Put the edited fountain on the selected object: one undo step
    /// "Fill", which the next edits of the same drag join (`begun`).
    fn set_selected_fountain(&mut self, id: ShapeId, f: Fountain, begun: bool) {
        if self.tool == crate::tools::Tool::Transparency {
            // The transparency mask: one step "Transparency".
            let Some(s) = self.doc().find_shape(id) else {
                return;
            };
            let effects: Vec<tracedraw_core::live::Effect> = s
                .effects
                .iter()
                .map(|e| match e {
                    tracedraw_core::live::Effect::Transparency { merge, target, .. } => {
                        tracedraw_core::live::Effect::Transparency {
                            mask: Fill::Fountain(f.clone()),
                            merge: *merge,
                            target: *target,
                        }
                    }
                    other => other.clone(),
                })
                .collect();
            let cmd = Command::SetEffects { shape: id, effects };
            let r = if begun && self.engine.undo_label() == Some("Transparency") {
                self.engine.amend(&cmd)
            } else {
                self.engine.run_with_label(&cmd, "Transparency")
            };
            if let Err(e) = r {
                self.status = e.to_string();
            }
            return;
        }
        let cmd = Command::SetFill {
            shapes: vec![id],
            fill: Fill::Fountain(f),
        };
        if begun && self.engine.undo_label() == Some("Fill") {
            let _ = self.engine.amend(&cmd);
        } else {
            self.run(cmd);
        }
    }

    /// A drag across the object with the Transparency tool: a linear mask
    /// from opaque where the drag began to clear where it ended (a
    /// gradient mask there already keeps its nodes and gets the new
    /// line), its nodes then shown for editing (#448).
    pub fn transparency_from_drag(&mut self, id: ShapeId, from: Point, to: Point) {
        use tracedraw_core::geometry::Shape as _;
        let Some(s) = self.doc().shape_on_page(id) else {
            return;
        };
        let b = s.page_path().bounding_box();
        let old = s.effects.iter().find_map(|e| match e {
            tracedraw_core::live::Effect::Transparency {
                mask: Fill::Fountain(f),
                merge,
                target,
            } if f.kind != FountainKind::Shaped => Some((f.clone(), *merge, *target)),
            _ => None,
        });
        let (mut f, merge, target) = old.unwrap_or_else(|| {
            (
                Fountain::two(
                    FountainKind::Linear,
                    tracedraw_core::Color::WHITE,
                    tracedraw_core::Color::BLACK,
                    0.0,
                ),
                self.transparency_default.merge,
                self.transparency_default.target,
            )
        });
        let d = to - from;
        let skew = from + Vec2::new(-d.y, d.x);
        let rel = |q: Point| Fountain::page_to_rel(b, q);
        f.set_handles(
            b,
            FountainHandles {
                start: rel(from),
                end: rel(to),
                skew: rel(skew),
            },
        );
        let mut effects: Vec<tracedraw_core::live::Effect> = s
            .effects
            .iter()
            .filter(|e| !matches!(e, tracedraw_core::live::Effect::Transparency { .. }))
            .cloned()
            .collect();
        effects.push(tracedraw_core::live::Effect::Transparency {
            mask: Fill::Fountain(f.clone()),
            merge,
            target,
        });
        self.transparency_default.kind = 1;
        self.transparency_default.mask = Fill::Fountain(f);
        let cmds = vec![
            Command::SetEffects { shape: id, effects },
            Command::SetOpacity {
                shapes: vec![id],
                opacity: 1.0,
            },
        ];
        if let Err(e) = self.engine.run_batch("Transparency", &cmds) {
            self.status = e.to_string();
        }
        self.select(vec![id]);
    }

    /// Add a colour node at `t` on the line, in the colour and opacity
    /// the fill has there, and select it. Its index, or none when there
    /// is a node there already.
    pub fn add_fountain_stop(&mut self, t: f64) -> Option<usize> {
        let (id, mut f, _) = self.selected_fountain()?;
        let t = t.clamp(0.0, 1.0);
        let i = f.stops.iter().position(|s| s.pos > t)?;
        if i == 0 || (f.stops[i].pos - t) < 1e-4 || (t - f.stops[i - 1].pos) < 1e-4 {
            return None;
        }
        let blended = f.blended_stops();
        let at = tracedraw_core::style::sample_stop(&blended, t);
        f.stops.insert(
            i,
            tracedraw_core::Stop {
                opacity: at.opacity,
                ..tracedraw_core::Stop::new(t, at.color)
            },
        );
        self.set_selected_fountain(id, f, false);
        self.fill_node = Some(i);
        Some(i)
    }

    /// Remove colour node `i` (not the start or the end).
    pub fn remove_fountain_stop(&mut self, i: usize) -> bool {
        let Some((id, mut f, _)) = self.selected_fountain() else {
            return false;
        };
        if i == 0 || i + 1 >= f.stops.len() {
            return false;
        }
        f.stops.remove(i);
        self.set_selected_fountain(id, f, false);
        self.fill_node = None;
        true
    }

    /// Change colour node `i`: its colour, opacity or position (kept
    /// between its neighbours; the start and the end stay at the ends).
    pub fn set_fountain_node(
        &mut self,
        i: usize,
        color: Option<tracedraw_core::Color>,
        opacity: Option<f32>,
        pos: Option<f64>,
        begun: bool,
    ) {
        let Some((id, mut f, _)) = self.selected_fountain() else {
            return;
        };
        let n = f.stops.len();
        if i >= n {
            return;
        }
        if let Some(c) = color {
            f.stops[i].color = c;
        }
        if let Some(o) = opacity {
            f.stops[i].opacity = o.clamp(0.0, 1.0);
        }
        if let Some(t) = pos.filter(|_| i > 0 && i + 1 < n) {
            let lo = f.stops[i - 1].pos + 1e-3;
            let hi = f.stops[i + 1].pos - 1e-3;
            if lo <= hi {
                f.stops[i].pos = t.clamp(lo, hi);
            }
        }
        self.set_selected_fountain(id, f, begun);
    }

    /// Move a fountain node to `p` (see [`moved_nodes`]). The first move
    /// of a drag is an undo step "Fill"; the next ones join it (`begun`).
    pub fn drag_fountain_handle(&mut self, handle: FountainHandle, p: Point, begun: bool) {
        let Some((id, mut f, b)) = self.selected_fountain() else {
            return;
        };
        let pts = fountain_handles(&f, b);
        let [(_, s), (_, e), (_, k)] = pts.as_slice() else {
            return;
        };
        match handle {
            FountainHandle::Stop(i) => {
                let t = line_param(*s, *e, p);
                self.set_fountain_node(i, None, None, Some(t), begun);
                return;
            }
            FountainHandle::Mid(i) => {
                let t = line_param(*s, *e, p);
                if let Some([a, z]) = f.stops.get(i..i + 2).map(|w| [w[0].pos, w[1].pos]) {
                    if z - a > 1e-9 {
                        f.stops[i].mid = ((t - a) / (z - a)).clamp(0.01, 0.99);
                        self.set_selected_fountain(id, f, begun);
                    }
                }
                return;
            }
            _ => {}
        }
        let (s, e, k) = moved_nodes(f.kind, (*s, *e, *k), handle, p);
        let rel = |q: Point| Fountain::page_to_rel(b, q);
        f.set_handles(
            b,
            FountainHandles {
                start: rel(s),
                end: rel(e),
                skew: rel(k),
            },
        );
        self.set_selected_fountain(id, f, begun);
    }

    /// A drag across the object with the Interactive Fill tool: the fill
    /// runs from where the drag began to where it ended (its kind and
    /// colours kept when it is a fountain already; else a linear one from
    /// the default colour to white), the skew handle square to the axis.
    pub fn fill_from_drag(&mut self, id: ShapeId, from: Point, to: Point) {
        use tracedraw_core::geometry::Shape as _;
        let Some(s) = self.doc().shape_on_page(id) else {
            return;
        };
        let b = s.page_path().bounding_box();
        let mut f = match &s.fill {
            Fill::Fountain(f) if f.kind != FountainKind::Shaped => f.clone(),
            _ => {
                let first = match &self.default_fill() {
                    Fill::Solid(c) => *c,
                    _ => tracedraw_core::Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
                };
                Fountain::two(
                    FountainKind::Linear,
                    first,
                    tracedraw_core::Color::WHITE,
                    0.0,
                )
            }
        };
        let d = to - from;
        let skew = from + Vec2::new(-d.y, d.x);
        let rel = |q: Point| Fountain::page_to_rel(b, q);
        f.set_handles(
            b,
            FountainHandles {
                start: rel(from),
                end: rel(to),
                skew: rel(skew),
            },
        );
        self.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Fountain(f),
        });
    }

    /// A colour picked from a palette: with Interactive Fill and a colour
    /// node picked, that node takes it; otherwise it fills the selection.
    pub fn palette_fill(&mut self, c: tracedraw_core::Color) {
        if self.tool == crate::tools::Tool::InteractiveFill {
            if let Some((_, i)) = self.selected_fill_node() {
                self.set_fountain_node(i, Some(c), None, None, false);
                return;
            }
        }
        self.apply_fill(Fill::Solid(c));
    }

    /// A colour dropped at page point `p` with Interactive Fill: on a
    /// colour node it recolours it; on the fill's line it adds a node of
    /// that colour there. False when it fell on neither.
    pub fn drop_on_fill_line(&mut self, c: tracedraw_core::Color, p: Point) -> bool {
        if self.tool != crate::tools::Tool::InteractiveFill {
            return false;
        }
        if let Some(i) = self
            .fountain_handle_at(p)
            .and_then(|h| self.fountain_stop_of(h))
        {
            self.set_fountain_node(i, Some(c), None, None, false);
            self.fill_node = Some(i);
            return true;
        }
        let Some(t) = self.fountain_line_at(p) else {
            return false;
        };
        let Some(i) = self.add_fountain_stop(t) else {
            return false;
        };
        self.set_fountain_node(i, Some(c), None, None, true);
        true
    }

    /// Apply a fill to the selection, or make it the default when nothing
    /// is selected.
    pub fn apply_fill_or_default(&mut self, fill: Fill) {
        if self.selection.is_empty() {
            self.request_default_change(crate::defaults::DefaultChange::Fill(fill));
        } else {
            self.apply_fill(fill);
        }
    }
}

/// The node bar: under the picked colour node of the selected fill, as
/// the reference program shows it, the node's colour (a swatch that opens
/// the colour picker) and its transparency (0 % opaque, 100 % clear).
/// Hidden while a node is dragged. An edit made while the pointer stays
/// down (a picker or slider drag) is one undo step. The dotted grip at its
/// left drags it off the object; it keeps that offset from its node.
pub fn node_bar(app: &mut App, ui: &mut egui::Ui) {
    use crate::tools::Tool;
    if app.tool != Tool::InteractiveFill || !matches!(app.drag, crate::app::Drag::None) {
        return;
    }
    let Some((_, f, b)) = app.selected_fountain() else {
        return;
    };
    let Some(i) = app.fill_node.filter(|i| *i < f.stops.len()) else {
        return;
    };
    let hs = fountain_handles(&f, b);
    let [(_, s), (_, e), _] = hs.as_slice() else {
        return;
    };
    let at = app.view.to_screen(on_line(*s, *e, f.stops[i].pos));
    let id = egui::Id::new("fill_node_bar");
    let down = ui.input(|inp| inp.pointer.any_down());
    let begun = down && ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    let mut color = f.stops[i].color;
    let mut transparency = ((1.0 - f.stops[i].opacity) * 100.0).round();
    let mut changed_color = false;
    let mut changed_opacity = false;
    let offset_id = id.with("offset");
    let mut offset = ui.data(|d| d.get_temp::<egui::Vec2>(offset_id).unwrap_or_default());
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(at + egui::vec2(-12.0, 10.0) + offset)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::symmetric(4, 2))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        offset += crate::ui::color_picker::drag_strip(ui, egui::vec2(8.0, 18.0));
                        changed_color = crate::ui::color_picker::color_button(ui, &mut color);
                        ui.label(
                            egui::RichText::new(crate::i18n::tr("toolbar.node_transparency"))
                                .size(11.0),
                        );
                        changed_opacity = ui
                            .add(
                                crate::ui::Slider::new(&mut transparency, 0.0..=100.0)
                                    .suffix(" %")
                                    .fixed_decimals(0),
                            )
                            .changed();
                    });
                });
        });
    ui.data_mut(|d| d.insert_temp(offset_id, offset));
    if changed_color || changed_opacity {
        app.set_fountain_node(
            i,
            changed_color.then_some(color),
            changed_opacity.then_some(1.0 - transparency / 100.0),
            None,
            begun,
        );
        ui.data_mut(|d| d.insert_temp(id, down));
    } else if !down {
        ui.data_mut(|d| d.insert_temp(id, false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{document::ShapeKind, Color};

    fn square(app: &mut App) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        id
    }

    fn near(a: Point, b: Point) -> bool {
        (a - b).hypot() < 1e-6
    }

    /// A linear fill shows its start, end and skew handle; the end turns
    /// the axis with the skew handle, the start carries the skew handle;
    /// a drag is one undo step (#330).
    #[test]
    fn linear_nodes_move_as_the_reference_moves_them() {
        let mut app = App::headless();
        square(&mut app);
        app.apply_fill(Fill::linear(Color::BLACK, Color::WHITE, 0.0));
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let hs = fountain_handles(&f, b);
        assert_eq!(hs.len(), 3);
        assert!(near(hs[0].1, Point::new(0.0, 10.0)), "{hs:?}");
        assert!(near(hs[1].1, Point::new(40.0, 10.0)));
        // Drag the end node straight up (two steps of one drag).
        let depth = app.engine.history_labels().0.len();
        app.drag_fountain_handle(FountainHandle::End, Point::new(0.0, 20.0), false);
        app.drag_fountain_handle(FountainHandle::End, Point::new(0.0, 30.0), true);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let (_, f, b) = app.selected_fountain().expect("fountain");
        assert!((f.angle - 90.0).abs() < 1e-6, "{}", f.angle);
        let hs = fountain_handles(&f, b);
        assert!(near(hs[1].1, Point::new(0.0, 30.0)), "{hs:?}");
        // The skew handle turned with the axis: square to it still.
        let axis = hs[1].1 - hs[0].1;
        let skew = hs[2].1 - hs[0].1;
        assert!(axis.dot(skew).abs() < 1e-6, "{axis:?} {skew:?}");
        // The start moves on its own, carrying the skew handle.
        app.drag_fountain_handle(FountainHandle::Start, Point::new(10.0, 10.0), false);
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let hs2 = fountain_handles(&f, b);
        assert!(near(hs2[0].1, Point::new(10.0, 10.0)));
        assert!(near(hs2[1].1, hs[1].1));
        assert!(((hs2[2].1 - hs2[0].1) - skew).hypot() < 1e-6, "{hs2:?}");
        assert_eq!(
            app.fountain_handle_at(Point::new(0.0, 30.0)),
            Some(FountainHandle::End)
        );
        assert_eq!(app.fountain_handle_at(Point::new(35.0, 2.0)), None);
    }

    /// The Transparency tool: a drag gives a linear mask from where it
    /// began to where it ended, and its nodes are edited as a fill's,
    /// the midpoint slider included; the fill is left alone (#448).
    #[test]
    fn the_transparency_mask_has_nodes_and_a_midpoint() {
        let mut app = App::headless();
        let id = square(&mut app);
        let fill = app.doc().find_shape(id).expect("square").fill.clone();
        app.tool = crate::tools::Tool::Transparency;
        let (from, to) = (Point::new(10.0, 20.0), Point::new(30.0, 20.0));
        app.transparency_from_drag(id, from, to);
        let (_, f, b) = app.selected_fountain().expect("a gradient mask");
        assert_eq!(f.kind, FountainKind::Linear);
        let hs = fountain_handles(&f, b);
        assert!(near(hs[0].1, from) && near(hs[1].1, to), "{hs:?}");
        assert_eq!(app.doc().find_shape(id).expect("square").opacity, 1.0);
        // The midpoint slider sits halfway; dragged to a quarter, the
        // blend's middle moves there.
        let mid = on_line(from, to, 0.5);
        assert_eq!(app.fountain_handle_at(mid), Some(FountainHandle::Mid(0)));
        app.drag_fountain_handle(FountainHandle::Mid(0), on_line(from, to, 0.25), false);
        app.drag_fountain_handle(FountainHandle::Mid(0), on_line(from, to, 0.3), true);
        let (_, f, _) = app.selected_fountain().expect("mask");
        assert!((f.stops[0].mid - 0.3).abs() < 1e-6, "{}", f.stops[0].mid);
        assert_eq!(app.engine.undo_label(), Some("Transparency"));
        // A node added on the line, then the fill untouched.
        assert!(app.add_fountain_stop(0.7).is_some());
        assert_eq!(app.selected_fountain().expect("mask").1.stops.len(), 3);
        assert_eq!(app.doc().find_shape(id).expect("square").fill, fill);
    }

    #[test]
    fn shaped_fountains_have_no_handles() {
        let f = Fountain::two(FountainKind::Shaped, Color::BLACK, Color::WHITE, 0.0);
        assert!(fountain_handles(&f, Rect::new(0.0, 0.0, 40.0, 20.0)).is_empty());
    }

    /// The centre of an elliptical fill moves the whole fill.
    #[test]
    fn the_centre_of_a_radial_fill_moves_it_whole() {
        let mut app = App::headless();
        square(&mut app);
        app.apply_fill(Fill::radial(Color::BLACK, Color::WHITE));
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let before = fountain_handles(&f, b);
        app.drag_fountain_handle(FountainHandle::Start, Point::new(30.0, 15.0), false);
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let after = fountain_handles(&f, b);
        let d = Point::new(30.0, 15.0) - before[0].1;
        for (a, z) in before.iter().zip(&after) {
            assert!(near(z.1, a.1 + d), "{a:?} {z:?}");
        }
        // The offset follows for the formats that know only that.
        assert!((f.offset.x - 0.5).abs() < 1e-9 && (f.offset.y - 0.5).abs() < 1e-9);
    }

    /// A drag across the object puts the start and the end where the drag
    /// began and ended, keeping a fountain's kind and colours.
    #[test]
    fn a_drag_puts_the_nodes_where_it_began_and_ended() {
        let mut app = App::headless();
        let id = square(&mut app);
        app.apply_fill(Fill::radial(Color::rgb8(200, 0, 0), Color::WHITE));
        app.fill_from_drag(id, Point::new(5.0, 5.0), Point::new(15.0, 5.0));
        let (_, f, b) = app.selected_fountain().expect("fountain");
        assert_eq!(f.kind, FountainKind::Radial);
        assert_eq!(f.first_color(), Color::rgb8(200, 0, 0));
        let hs = fountain_handles(&f, b);
        assert!(near(hs[0].1, Point::new(5.0, 5.0)) && near(hs[1].1, Point::new(15.0, 5.0)));
        assert!(near(hs[2].1, Point::new(5.0, 15.0)), "{hs:?}");
    }

    #[test]
    fn colour_nodes_are_added_moved_and_removed() {
        let mut app = App::headless();
        square(&mut app);
        app.apply_fill(Fill::linear(Color::BLACK, Color::WHITE, 0.0));
        // Double click half way along the line: a grey node, picked.
        assert_eq!(app.fountain_line_at(Point::new(20.0, 10.0)), Some(0.5));
        assert_eq!(app.add_fountain_stop(0.5), Some(1));
        assert_eq!(app.fill_node, Some(1));
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert_eq!(f.stops.len(), 3);
        assert!((120..=135).contains(&f.stops[1].color.to_rgb8()[0]));
        assert_eq!(
            app.fountain_handle_at(Point::new(20.0, 10.0)),
            Some(FountainHandle::Stop(1))
        );
        // Not twice at one place.
        assert_eq!(app.add_fountain_stop(0.5), None);
        // Dragged along the line, kept between its neighbours.
        app.drag_fountain_handle(FountainHandle::Stop(1), Point::new(30.0, 12.0), false);
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert!((f.stops[1].pos - 0.75).abs() < 1e-9, "{:?}", f.stops);
        app.drag_fountain_handle(FountainHandle::Stop(1), Point::new(90.0, 10.0), true);
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert!(f.stops[1].pos < 1.0 && f.stops[1].pos > 0.99);
        // The midpoint slider of the first blend, at its middle.
        let mids = midpoint_params(&f);
        assert_eq!(mids.len(), 2);
        let m0 = on_line(Point::new(0.0, 10.0), Point::new(40.0, 10.0), mids[0].1);
        assert_eq!(app.fountain_handle_at(m0), Some(FountainHandle::Mid(0)));
        let a = f.stops[1].pos;
        app.drag_fountain_handle(FountainHandle::Mid(0), Point::new(10.0, 10.0), false);
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert!((f.stops[0].mid - 0.25 / a).abs() < 1e-9, "{:?}", f.stops[0]);
        // Removed with a double click; the ends stay.
        assert!(!app.remove_fountain_stop(0));
        assert!(app.remove_fountain_stop(1));
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert_eq!(f.stops.len(), 2);
        assert_eq!(app.fill_node, None);
    }

    #[test]
    fn a_picked_node_takes_colours_and_transparency() {
        let mut app = App::headless();
        square(&mut app);
        app.tool = crate::tools::Tool::InteractiveFill;
        app.apply_fill(Fill::linear(Color::BLACK, Color::WHITE, 0.0));
        let red = Color::rgb8(255, 0, 0);
        // No node picked: a palette colour fills the object.
        app.palette_fill(red);
        assert!(app.selected_fountain().is_none());
        app.undo();
        // The end picked: a palette colour goes to it.
        app.fill_node = app.fountain_stop_of(FountainHandle::End);
        assert_eq!(app.fill_node, Some(1));
        app.palette_fill(red);
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert_eq!(f.last_color(), red);
        assert_eq!(f.first_color(), Color::BLACK);
        app.set_fountain_node(1, None, Some(0.4), None, false);
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert!((f.stops[1].opacity - 0.4).abs() < 1e-6);
        // A colour dropped on the line adds a node of it, one undo step.
        let blue = Color::rgb8(0, 0, 255);
        assert!(app.drop_on_fill_line(blue, Point::new(10.0, 10.0)));
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert_eq!(f.stops.len(), 3);
        assert_eq!(f.stops[1].color, blue);
        assert!((f.stops[1].pos - 0.25).abs() < 1e-9);
        app.undo();
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert_eq!(f.stops.len(), 2);
        // Dropped on a node, it recolours it.
        assert!(app.drop_on_fill_line(blue, Point::new(0.0, 10.0)));
        let (_, f, _) = app.selected_fountain().expect("fountain");
        assert_eq!(f.first_color(), blue);
    }
}
