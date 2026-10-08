//! Higher-level editing operations built from commands: align, distribute,
//! transformations, lock, combine, bitmap import.

use crate::app::App;
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, Point, Rect, Vec2},
    Command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    Top,
    Bottom,
    CenterH,
    CenterV,
    CenterPage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distribute {
    SpacingH,
    SpacingV,
    CentersH,
    CentersV,
}

impl App {
    /// the editor aligns to the last-selected object; with one object, to the page.
    pub fn align(&mut self, how: Align) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return;
        }
        let page = self.page_rect();
        let target: Rect = if shapes.len() == 1 || how == Align::CenterPage {
            page
        } else {
            shapes.last().map(Shape::bounds).unwrap_or(page)
        };
        let mut cmds = Vec::new();
        for s in &shapes {
            let b = s.bounds();
            let d = match how {
                Align::Left => Vec2::new(target.x0 - b.x0, 0.0),
                Align::Right => Vec2::new(target.x1 - b.x1, 0.0),
                Align::Top => Vec2::new(0.0, target.y1 - b.y1),
                Align::Bottom => Vec2::new(0.0, target.y0 - b.y0),
                Align::CenterH => Vec2::new(0.0, target.center().y - b.center().y),
                Align::CenterV => Vec2::new(target.center().x - b.center().x, 0.0),
                Align::CenterPage => target.center() - b.center(),
            };
            if d.hypot() > 1e-9 {
                cmds.push(Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: Affine::translate(d),
                });
            }
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Align", &cmds);
        }
    }

    pub fn distribute(&mut self, how: Distribute) {
        let mut shapes = self.selected_shapes();
        if shapes.len() < 3 {
            return;
        }
        let horizontal = matches!(how, Distribute::SpacingH | Distribute::CentersH);
        shapes.sort_by(|a, b| {
            let (ba, bb) = (a.bounds(), b.bounds());
            let (ka, kb) = if horizontal {
                (ba.center().x, bb.center().x)
            } else {
                (ba.center().y, bb.center().y)
            };
            ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let bounds: Vec<Rect> = shapes.iter().map(Shape::bounds).collect();
        let n = bounds.len();
        let mut cmds = Vec::new();
        match how {
            Distribute::CentersH | Distribute::CentersV => {
                let first = if horizontal {
                    bounds[0].center().x
                } else {
                    bounds[0].center().y
                };
                let last = if horizontal {
                    bounds[n - 1].center().x
                } else {
                    bounds[n - 1].center().y
                };
                let step = (last - first) / (n - 1) as f64;
                for (i, (s, b)) in shapes.iter().zip(&bounds).enumerate() {
                    let want = first + step * i as f64;
                    let have = if horizontal {
                        b.center().x
                    } else {
                        b.center().y
                    };
                    let d = if horizontal {
                        Vec2::new(want - have, 0.0)
                    } else {
                        Vec2::new(0.0, want - have)
                    };
                    cmds.push(Command::TransformShapes {
                        shapes: vec![s.id],
                        transform: Affine::translate(d),
                    });
                }
            }
            Distribute::SpacingH | Distribute::SpacingV => {
                let total: f64 = bounds
                    .iter()
                    .map(|b| if horizontal { b.width() } else { b.height() })
                    .sum();
                let span = if horizontal {
                    bounds[n - 1].x1 - bounds[0].x0
                } else {
                    bounds[n - 1].y1 - bounds[0].y0
                };
                let gap = (span - total) / (n - 1) as f64;
                let mut cursor = if horizontal {
                    bounds[0].x0
                } else {
                    bounds[0].y0
                };
                for (s, b) in shapes.iter().zip(&bounds) {
                    let have = if horizontal { b.x0 } else { b.y0 };
                    let d = if horizontal {
                        Vec2::new(cursor - have, 0.0)
                    } else {
                        Vec2::new(0.0, cursor - have)
                    };
                    cmds.push(Command::TransformShapes {
                        shapes: vec![s.id],
                        transform: Affine::translate(d),
                    });
                    cursor += if horizontal { b.width() } else { b.height() } + gap;
                }
            }
        }
        let _ = self.engine.run_batch("Distribute", &cmds);
    }

    /// Apply a transform about the selection centre; optionally to a duplicate.
    pub fn transform_about_center(&mut self, t: Affine, duplicate: bool) {
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let c = b.center();
        let full = Affine::translate(c.to_vec2()) * t * Affine::translate(-c.to_vec2());
        if duplicate {
            self.duplicate_in_place();
        }
        self.transform_selection(full);
    }

    fn duplicate_in_place(&mut self) {
        let saved = self.duplicate_offset;
        self.duplicate_offset = Vec2::ZERO;
        self.duplicate();
        self.duplicate_offset = saved;
    }

    pub fn set_locked(&mut self, locked: bool) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = self.selection.clone();
        self.run(Command::SetLocked { shapes, locked });
        if locked {
            self.selection.clear();
        }
    }

    pub fn unlock_all(&mut self) {
        let ids: Vec<_> = self
            .doc()
            .page(self.page)
            .map(|p| {
                p.layers
                    .iter()
                    .flat_map(|l| &l.shapes)
                    .filter(|s| s.locked)
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default();
        if !ids.is_empty() {
            self.run(Command::SetLocked {
                shapes: ids,
                locked: false,
            });
        }
    }

    pub fn combine(&mut self) {
        if self.selection.len() < 2 {
            return;
        }
        let shapes = self.selection.clone();
        // Make everything a curve first so Combine has paths to merge.
        self.convert_to_curves();
        let keep = shapes.last().copied();
        self.run(Command::Combine { shapes });
        self.selection = keep.into_iter().collect();
    }

    pub fn break_apart(&mut self) {
        let ids = self.selection.clone();
        for id in ids {
            self.run(Command::BreakApart { shape: id });
        }
    }

    pub fn set_opacity(&mut self, opacity: f64) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = self.selection.clone();
        if self.engine.undo_label() == Some("Transparency") {
            let _ = self.engine.undo();
        }
        self.run(Command::SetOpacity { shapes, opacity });
    }

    /// Import a raster image as a bitmap object at the page centre (96 dpi).
    pub fn import_bitmap(&mut self, path: &std::path::Path) {
        let img = match image::open(path) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.status = format!("Could not read {}: {e}", path.display());
                return;
            }
        };
        let (w, h) = img.dimensions();
        let mut png = Vec::new();
        if let Err(e) = image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        {
            self.status = format!("Could not encode image: {e}");
            return;
        }
        let mm_w = w as f64 * 25.4 / 96.0;
        let mm_h = h as f64 * 25.4 / 96.0;
        let page = self.page_rect();
        let c = page.center();
        let rect = Rect::new(
            c.x - mm_w / 2.0,
            c.y - mm_h / 2.0,
            c.x + mm_w / 2.0,
            c.y + mm_h / 2.0,
        );
        let Some(layer) = self.active_layer() else {
            return;
        };
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(
            id,
            ShapeKind::Bitmap {
                rect,
                width_px: w,
                height_px: h,
                png,
            },
        );
        shape.stroke = None;
        shape.name = path.file_name().and_then(|n| n.to_str()).map(String::from);
        self.run(Command::AddShape { layer, shape });
        self.select(vec![id]);
        let _ = Point::ZERO;
    }
}
