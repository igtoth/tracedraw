//! The property bar's pieces: values in
//! white fields stacked two to a column with a small icon before each,
//! spin arrows, icon buttons that stay pressed, the object origin selector,
//! and the page, object, outline and zoom bars built from them.

use crate::app::{App, Units};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::tips::RichTip;
use egui::{epaint, Color32, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, Point, Size},
    Command, CornerKind, Corners,
};

/// Height of the bar's content: two stacked rows of fields.
/// Two fields stacked: 19 px each, sharing their middle border.
pub const BAR_H: f32 = 37.0;
const ROW: f32 = 19.0;
/// Icon buttons are 28 px square with an 18 px icon.
/// A property bar button's side: 28 px at the medium button size.
fn btn() -> f32 {
    crate::theme::Tokens::prop_button()
}

/// The small pictures before fields and on the bar's buttons, drawn on a
/// 16 x 16 grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pic {
    Width,
    Height,
    PageWidth,
    PageHeight,
    Portrait,
    Landscape,
    AllPages,
    CurrentPage,
    /// Fit page to drawing: a page shrunk onto the object on it.
    FitPage,
    Nudge,
    DupX,
    DupY,
    TreatFilled,
    Rotate,
    Lock(bool),
    MirrorH,
    MirrorV,
    Pen,
    ZoomIn,
    ZoomOut,
    ZoomSelected,
    ZoomAll,
    ZoomPage,
    ZoomWidth,
    ZoomHeight,
    Ellipse,
    Pie,
    Arc,
    ArcDirection,
    CornerRound,
    CornerScallop,
    CornerChamfer,
    /// Relative corner scaling: a small corner, a larger one, an arrow.
    CornerScaling,
    ToFront,
    ToBack,
    ToCurves,
    // The Shape tool's buttons.
    AddNode,
    DeleteNode,
    JoinNodes,
    BreakCurve,
    ToLine,
    ToCurve,
    NodeCusp,
    NodeSmooth,
    NodeSymmetrical,
    ReverseDirection,
    ExtendToClose,
    ExtractSubpath,
    CloseCurve,
    /// The Pen's preview mode: a node, a dashed segment to a pointer.
    PenPreview,
    /// The Pen's automatic adding and deleting of nodes: a curve with a
    /// node, a plus and a minus.
    AutoAddDelete,
    StretchNodes,
    RotateNodes,
    AlignNodes,
    ReflectH,
    ReflectV,
    Elastic,
    SelectAllNodes,
    ReduceNodes,
    // The Star tool's kinds.
    Star,
    ComplexStar,
    // Arranging several objects: grouping, combining and shaping, drawn
    // as a square behind a circle.
    Group,
    Ungroup,
    UngroupAll,
    Combine,
    BreakApart,
    Weld,
    Trim,
    Intersect,
    Simplify,
    FrontMinusBack,
    BackMinusFront,
    Boundary,
    AlignDistribute,
    // Text: alignments, bullets, drop cap, the Text docker, editing, and
    // wrapping paragraph text around an object.
    TextLeft,
    /// Text in lines, and text in columns (vertical text).
    TextHorizontal,
    TextVertical,
    TextCenter,
    TextRight,
    TextJustify,
    /// No alignment: lines crossed by a red X.
    TextNone,
    /// Forced justification: every line full width.
    TextForceJustify,
    Bullets,
    DropCap,
    TextProperties,
    EditText,
    WrapText,
    // The Transformations docker's pages (Rotate has its own above).
    Position,
    ScaleMirror,
    SizeBox,
    SkewBox,
    // Envelopes (drawn in `ui::envelope_ui`): the four modes, Add New,
    // Clear, Copy and Keep Lines.
    EnvelopeStraight,
    EnvelopeSingleArc,
    EnvelopeDoubleArc,
    EnvelopeUnconstrained,
    EnvelopeAddNew,
    EnvelopeClear,
    EnvelopeCopy,
    KeepLines,
    // Menu rows (drawn by `menu_pics`).
    Tabs,
    Columns,
    TextStatistics,
    ConvertText,
    FitTextToPath,
    StraightenText,
    AlignBaseline,
    ChangeCase,
    Encode,
    ConvertToBitmap,
    StraightenImage,
    EditBitmap,
    CropBitmap,
    Resample,
    BreakLink,
    UpdateLink,
    QuickTrace,
    NewWindow,
    RefreshWindow,
    CloseWindow,
    CloseAll,
    Cascade,
    TileHorizontally,
    TileVertically,
    CombineWindows,
    DockWindow,
}

fn q(r: Rect, x: f32, y: f32) -> Pos2 {
    r.min + Vec2::new(x, y) * (r.width() / 16.0)
}

pub fn draw_pic(painter: &Painter, r: Rect, pic: Pic, color: Color32) {
    let k = r.width() / 16.0;
    let s = Stroke::new(1.1 * k, color);
    let thin = Stroke::new(0.85 * k, color);
    let line = |a: (f32, f32), b: (f32, f32), st: Stroke| {
        painter.line_segment([q(r, a.0, a.1), q(r, b.0, b.1)], st);
    };
    let poly = |pts: &[(f32, f32)], fill: Color32, st: Stroke| {
        painter.add(epaint::PathShape::convex_polygon(
            pts.iter().map(|(x, y)| q(r, *x, *y)).collect(),
            fill,
            st,
        ));
    };
    let outline = |pts: &[(f32, f32)], st: Stroke| {
        painter.add(epaint::PathShape::closed_line(
            pts.iter().map(|(x, y)| q(r, *x, *y)).collect(),
            st,
        ));
    };
    let arrow_h = |y: f32, x0: f32, x1: f32| {
        line((x0, y), (x1, y), thin);
        poly(
            &[(x0, y), (x0 + 2.5, y - 2.0), (x0 + 2.5, y + 2.0)],
            color,
            Stroke::NONE,
        );
        poly(
            &[(x1, y), (x1 - 2.5, y - 2.0), (x1 - 2.5, y + 2.0)],
            color,
            Stroke::NONE,
        );
    };
    let arrow_v = |x: f32, y0: f32, y1: f32| {
        line((x, y0), (x, y1), thin);
        poly(
            &[(x, y0), (x - 2.0, y0 + 2.5), (x + 2.0, y0 + 2.5)],
            color,
            Stroke::NONE,
        );
        poly(
            &[(x, y1), (x - 2.0, y1 - 2.5), (x + 2.0, y1 - 2.5)],
            color,
            Stroke::NONE,
        );
    };
    let magnifier = |painter: &Painter| {
        painter.circle_stroke(q(r, 6.5, 6.5), 4.6 * k, s);
        painter.line_segment(
            [q(r, 9.8, 9.8), q(r, 14.5, 14.5)],
            Stroke::new(2.0 * k, color),
        );
    };
    match pic {
        Pic::EnvelopeStraight
        | Pic::EnvelopeSingleArc
        | Pic::EnvelopeDoubleArc
        | Pic::EnvelopeUnconstrained
        | Pic::EnvelopeAddNew
        | Pic::EnvelopeClear
        | Pic::EnvelopeCopy
        | Pic::KeepLines => crate::ui::envelope_ui::draw_envelope_pic(painter, r, pic, color),
        Pic::Tabs
        | Pic::Columns
        | Pic::TextStatistics
        | Pic::ConvertText
        | Pic::FitTextToPath
        | Pic::StraightenText
        | Pic::AlignBaseline
        | Pic::ChangeCase
        | Pic::Encode
        | Pic::ConvertToBitmap
        | Pic::StraightenImage
        | Pic::EditBitmap
        | Pic::CropBitmap
        | Pic::Resample
        | Pic::BreakLink
        | Pic::UpdateLink
        | Pic::QuickTrace
        | Pic::NewWindow
        | Pic::RefreshWindow
        | Pic::CloseWindow
        | Pic::CloseAll
        | Pic::Cascade
        | Pic::TileHorizontally
        | Pic::TileVertically
        | Pic::CombineWindows
        | Pic::DockWindow => crate::ui::menu_pics::draw(painter, r, pic, color),
        Pic::Width => {
            line((2.0, 3.0), (2.0, 13.0), thin);
            line((14.0, 3.0), (14.0, 13.0), thin);
            arrow_h(8.0, 3.0, 13.0);
        }
        Pic::Height => {
            line((3.0, 2.0), (13.0, 2.0), thin);
            line((3.0, 14.0), (13.0, 14.0), thin);
            arrow_v(8.0, 3.0, 13.0);
        }
        Pic::PageWidth => {
            outline(&[(2.0, 1.5), (14.0, 1.5), (14.0, 14.5), (2.0, 14.5)], thin);
            line((2.0, 5.0), (14.0, 5.0), thin);
            arrow_h(9.5, 3.0, 13.0);
        }
        Pic::PageHeight => {
            outline(&[(2.0, 1.5), (14.0, 1.5), (14.0, 14.5), (2.0, 14.5)], thin);
            line((5.5, 1.5), (5.5, 14.5), thin);
            arrow_v(10.0, 3.0, 13.0);
        }
        Pic::Portrait => outline(&[(4.0, 1.5), (12.0, 1.5), (12.0, 14.5), (4.0, 14.5)], s),
        Pic::Landscape => outline(&[(1.5, 4.0), (14.5, 4.0), (14.5, 12.0), (1.5, 12.0)], s),
        Pic::AllPages => {
            outline(&[(1.5, 1.5), (9.5, 1.5), (9.5, 11.5), (1.5, 11.5)], s);
            poly(
                &[(6.0, 4.5), (14.5, 4.5), (14.5, 14.5), (6.0, 14.5)],
                crate::theme::Tokens::field(),
                s,
            );
        }
        Pic::CurrentPage => {
            outline(&[(1.5, 1.5), (9.5, 1.5), (9.5, 14.5), (1.5, 14.5)], s);
            for x in [11.5, 13.5] {
                line((x, 5.0), (x, 14.5), Stroke::new(1.4 * k, color));
            }
        }
        Pic::FitPage => {
            outline(&[(1.5, 1.5), (14.5, 1.5), (14.5, 14.5), (1.5, 14.5)], thin);
            poly(
                &[(5.0, 5.0), (11.0, 5.0), (11.0, 11.0), (5.0, 11.0)],
                color,
                s,
            );
            arrow_h(8.0, 1.5, 4.5);
            arrow_h(8.0, 11.5, 14.5);
        }
        Pic::Nudge => {
            arrow_h(8.0, 1.0, 15.0);
            arrow_v(8.0, 1.0, 15.0);
        }
        Pic::DupX | Pic::DupY => {
            outline(&[(1.5, 1.5), (9.0, 1.5), (9.0, 9.0), (1.5, 9.0)], thin);
            outline(&[(5.0, 5.0), (12.5, 5.0), (12.5, 12.5), (5.0, 12.5)], thin);
            painter.text(
                q(r, 14.5, 13.5),
                egui::Align2::CENTER_CENTER,
                if pic == Pic::DupX { "x" } else { "y" },
                egui::FontId::proportional(7.5 * k),
                color,
            );
        }
        Pic::TreatFilled => {
            for (a, b) in [
                ((2.0, 2.0), (14.0, 2.0)),
                ((14.0, 2.0), (14.0, 14.0)),
                ((14.0, 14.0), (2.0, 14.0)),
                ((2.0, 14.0), (2.0, 2.0)),
            ] {
                // Dashed frame.
                for i in 0..4 {
                    let t0 = i as f32 / 4.0;
                    let t1 = t0 + 0.14;
                    line(
                        (a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0),
                        (a.0 + (b.0 - a.0) * t1, a.1 + (b.1 - a.1) * t1),
                        s,
                    );
                }
            }
            for (x, y) in [(2.0, 2.0), (14.0, 2.0), (2.0, 14.0), (14.0, 14.0)] {
                painter.rect_filled(
                    Rect::from_center_size(q(r, x, y), Vec2::splat(2.6 * k)),
                    0.0,
                    color,
                );
            }
        }
        Pic::Rotate => {
            let mut pts = Vec::new();
            for i in 0..=14 {
                let t = std::f32::consts::PI * (0.15 + 1.5 * i as f32 / 14.0);
                pts.push(q(r, 8.0 + 5.5 * t.cos(), 8.0 - 5.5 * t.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
            poly(
                &[(13.5, 5.0), (15.5, 9.5), (11.0, 8.5)],
                color,
                Stroke::NONE,
            );
        }
        Pic::Lock(locked) => {
            painter.rect_filled(
                Rect::from_min_max(q(r, 3.5, 7.5), q(r, 12.5, 14.5)),
                k,
                color,
            );
            let arm = if locked {
                vec![
                    q(r, 5.5, 7.5),
                    q(r, 5.5, 4.0),
                    q(r, 8.0, 2.0),
                    q(r, 10.5, 4.0),
                    q(r, 10.5, 7.5),
                ]
            } else {
                vec![
                    q(r, 5.5, 7.5),
                    q(r, 5.5, 4.0),
                    q(r, 8.0, 2.0),
                    q(r, 10.5, 3.5),
                ]
            };
            painter.add(epaint::PathShape::line(arm, Stroke::new(1.4 * k, color)));
        }
        Pic::Position => {
            // An object moved: its old place, the arrow, its new place.
            outline(&[(1.5, 8.5), (7.5, 8.5), (7.5, 14.5), (1.5, 14.5)], thin);
            painter.rect_filled(
                Rect::from_min_max(q(r, 8.5, 1.5), q(r, 14.5, 7.5)),
                0.0,
                color,
            );
            line((5.0, 11.0), (9.5, 6.5), s);
            poly(&[(10.5, 5.5), (6.6, 6.4), (9.6, 9.4)], color, Stroke::NONE);
        }
        Pic::ScaleMirror => {
            // A small object in the corner of the large one it becomes.
            outline(&[(1.5, 1.5), (14.5, 1.5), (14.5, 14.5), (1.5, 14.5)], thin);
            painter.rect_filled(
                Rect::from_min_max(q(r, 1.5, 9.5), q(r, 6.5, 14.5)),
                0.0,
                color,
            );
            line((6.5, 9.5), (11.0, 5.0), s);
            poly(&[(12.5, 3.5), (8.4, 4.4), (11.6, 7.6)], color, Stroke::NONE);
        }
        Pic::SizeBox => {
            outline(&[(5.0, 5.0), (14.5, 5.0), (14.5, 14.5), (5.0, 14.5)], s);
            arrow_h(2.0, 5.0, 14.5);
            arrow_v(2.0, 5.0, 14.5);
        }
        Pic::SkewBox => {
            outline(&[(5.5, 2.5), (15.0, 2.5), (10.5, 13.5), (1.0, 13.5)], s);
        }
        Pic::MirrorH => {
            line((8.0, 1.0), (8.0, 15.0), Stroke::new(0.8 * k, color));
            outline(&[(1.5, 3.5), (6.0, 8.0), (1.5, 12.5)], s);
            poly(
                &[(14.5, 3.5), (10.0, 8.0), (14.5, 12.5)],
                color,
                Stroke::NONE,
            );
        }
        Pic::MirrorV => {
            line((1.0, 8.0), (15.0, 8.0), Stroke::new(0.8 * k, color));
            outline(&[(3.5, 1.5), (8.0, 6.0), (12.5, 1.5)], s);
            poly(
                &[(3.5, 14.5), (8.0, 10.0), (12.5, 14.5)],
                color,
                Stroke::NONE,
            );
        }
        Pic::Pen => {
            // A fountain pen nib.
            poly(
                &[
                    (8.0, 1.0),
                    (12.0, 7.0),
                    (10.0, 12.5),
                    (6.0, 12.5),
                    (4.0, 7.0),
                ],
                Color32::TRANSPARENT,
                s,
            );
            line((8.0, 4.0), (8.0, 9.0), thin);
            painter.circle_filled(q(r, 8.0, 9.0), 1.0 * k, color);
            line((5.5, 14.5), (10.5, 14.5), s);
        }
        Pic::ZoomIn | Pic::ZoomOut => {
            magnifier(painter);
            line((4.5, 6.5), (8.5, 6.5), s);
            if pic == Pic::ZoomIn {
                line((6.5, 4.5), (6.5, 8.5), s);
            }
        }
        Pic::ZoomSelected => {
            magnifier(painter);
            outline(&[(4.5, 4.5), (8.5, 4.5), (8.5, 8.5), (4.5, 8.5)], thin);
        }
        Pic::ZoomAll => {
            magnifier(painter);
            painter.circle_stroke(q(r, 5.5, 7.0), 1.6 * k, thin);
            painter.circle_stroke(q(r, 8.0, 6.0), 1.6 * k, thin);
        }
        Pic::ZoomPage => {
            outline(
                &[
                    (2.0, 1.5),
                    (9.0, 1.5),
                    (11.0, 3.5),
                    (11.0, 14.5),
                    (2.0, 14.5),
                ],
                thin,
            );
            painter.circle_stroke(q(r, 10.5, 10.5), 3.0 * k, s);
            line((12.7, 12.7), (15.0, 15.0), Stroke::new(1.6 * k, color));
        }
        Pic::ZoomWidth => {
            arrow_h(2.0, 1.0, 15.0);
            painter.circle_stroke(q(r, 7.5, 9.0), 3.5 * k, s);
            line((10.0, 11.5), (13.5, 15.0), Stroke::new(1.6 * k, color));
        }
        Pic::ZoomHeight => {
            arrow_v(2.0, 1.0, 15.0);
            painter.circle_stroke(q(r, 9.0, 7.5), 3.5 * k, s);
            line((11.5, 10.0), (15.0, 13.5), Stroke::new(1.6 * k, color));
        }
        Pic::Ellipse => {
            painter.circle_stroke(q(r, 8.0, 8.0), 6.5 * k, s);
        }
        Pic::Pie | Pic::Arc => {
            let mut pts = Vec::new();
            for i in 0..=20 {
                let t = std::f32::consts::FRAC_PI_2 + 1.5 * std::f32::consts::PI * i as f32 / 20.0;
                pts.push(q(r, 8.0 + 6.5 * t.cos(), 8.0 - 6.5 * t.sin()));
            }
            if pic == Pic::Pie {
                pts.push(q(r, 8.0, 8.0));
                painter.add(epaint::PathShape::closed_line(pts, s));
            } else {
                painter.add(epaint::PathShape::line(pts, s));
            }
        }
        Pic::ArcDirection => {
            let mut pts = Vec::new();
            for i in 0..=12 {
                let t = std::f32::consts::PI * (1.0 - i as f32 / 12.0);
                pts.push(q(r, 8.0 + 6.0 * t.cos(), 10.0 - 6.0 * t.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
            poly(
                &[(12.0, 10.0), (15.5, 7.5), (15.5, 12.0)],
                color,
                Stroke::NONE,
            );
            poly(&[(4.0, 10.0), (0.5, 7.5), (0.5, 12.0)], color, Stroke::NONE);
        }
        Pic::CornerRound => {
            line((2.0, 15.0), (2.0, 8.0), s);
            line((8.0, 2.0), (15.0, 2.0), s);
            let mut pts = Vec::new();
            for i in 0..=8 {
                let t = std::f32::consts::PI * (1.0 - 0.5 * i as f32 / 8.0);
                pts.push(q(r, 8.0 + 6.0 * t.cos(), 8.0 - 6.0 * t.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
        }
        Pic::CornerScallop => {
            line((2.0, 15.0), (2.0, 8.0), s);
            line((8.0, 2.0), (15.0, 2.0), s);
            // A quarter circle centred on the corner, bending inwards.
            let mut pts = Vec::new();
            for i in 0..=8 {
                let t = std::f32::consts::FRAC_PI_2 * (1.0 - i as f32 / 8.0);
                pts.push(q(r, 2.0 + 6.0 * t.cos(), 2.0 + 6.0 * t.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
        }
        Pic::CornerChamfer => {
            painter.add(epaint::PathShape::line(
                vec![
                    q(r, 2.0, 15.0),
                    q(r, 2.0, 8.0),
                    q(r, 8.0, 2.0),
                    q(r, 15.0, 2.0),
                ],
                s,
            ));
        }
        Pic::CornerScaling => {
            // The corner at two sizes and an arrow from the small to the
            // large one.
            for (o, rad) in [(1.5f32, 6.0f32), (8.0, 3.5)] {
                let c = o + rad;
                let mut pts = vec![q(r, o, 15.0)];
                for i in 0..=8 {
                    let t = std::f32::consts::PI * (1.0 - 0.5 * i as f32 / 8.0);
                    pts.push(q(r, c + rad * t.cos(), c - rad * t.sin()));
                }
                pts.push(q(r, 15.0, o));
                painter.add(epaint::PathShape::line(pts, thin));
            }
            line((8.6, 8.6), (5.4, 5.4), thin);
            poly(&[(4.6, 4.6), (7.6, 5.4), (5.4, 7.6)], color, Stroke::NONE);
        }
        Pic::ToFront | Pic::ToBack => {
            let front = pic == Pic::ToFront;
            let (back, top) = if front {
                ((1.5, 1.5, 9.5, 9.5), (6.5, 6.5, 14.5, 14.5))
            } else {
                ((6.5, 6.5, 14.5, 14.5), (1.5, 1.5, 9.5, 9.5))
            };
            outline(
                &[
                    (back.0, back.1),
                    (back.2, back.1),
                    (back.2, back.3),
                    (back.0, back.3),
                ],
                thin,
            );
            poly(
                &[
                    (top.0, top.1),
                    (top.2, top.1),
                    (top.2, top.3),
                    (top.0, top.3),
                ],
                color,
                Stroke::NONE,
            );
        }
        Pic::ToCurves => {
            let mut pts = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                pts.push(q(
                    r,
                    1.5 + 13.0 * t,
                    12.0 - 9.0 * (t * std::f32::consts::PI).sin(),
                ));
            }
            painter.add(epaint::PathShape::line(pts, s));
            for (x, y) in [(1.5, 12.0), (8.0, 3.0), (14.5, 12.0)] {
                painter.rect_filled(
                    Rect::from_center_size(q(r, x, y), Vec2::splat(2.6 * k)),
                    0.0,
                    color,
                );
            }
        }
        Pic::TextLeft | Pic::TextCenter | Pic::TextRight | Pic::TextJustify => {
            // Lines of text of different lengths, set as the alignment.
            for (i, w) in [12.0, 8.0, 11.0, 7.0].iter().enumerate() {
                let y = 3.0 + i as f32 * 3.3;
                let w = if pic == Pic::TextJustify && i < 3 {
                    12.0
                } else {
                    *w
                };
                let x0 = match pic {
                    Pic::TextLeft | Pic::TextJustify => 2.0,
                    Pic::TextCenter => 8.0 - w / 2.0,
                    _ => 14.0 - w,
                };
                line((x0, y), (x0 + w, y), Stroke::new(1.3 * k, color));
            }
        }
        Pic::TextHorizontal => {
            // Three lines read across, an arrow along the first.
            for (i, w) in [12.0, 9.0, 11.0].iter().enumerate() {
                let y = 4.0 + i as f32 * 4.0;
                line((2.0, y), (2.0 + w, y), Stroke::new(1.3 * k, color));
            }
            arrow_h(14.0, 2.0, 14.0);
        }
        Pic::TextVertical => {
            // Three columns read down, from the right, an arrow along the
            // first.
            for (i, h) in [12.0, 9.0, 11.0].iter().enumerate() {
                let x = 12.0 - i as f32 * 4.0;
                line((x, 2.0), (x, 2.0 + h), Stroke::new(1.3 * k, color));
            }
            arrow_v(2.0, 2.0, 14.0);
        }
        Pic::TextNone | Pic::TextForceJustify => {
            for i in 0..4 {
                let y = 3.0 + i as f32 * 3.3;
                let w = if pic == Pic::TextNone {
                    [12.0, 8.0, 11.0, 7.0][i]
                } else {
                    12.0
                };
                line((2.0, y), (2.0 + w, y), Stroke::new(1.3 * k, color));
            }
            if pic == Pic::TextNone {
                let red = Stroke::new(1.4 * k, egui::Color32::from_rgb(0xD0, 0x30, 0x30));
                line((8.0, 7.5), (15.0, 14.5), red);
                line((15.0, 7.5), (8.0, 14.5), red);
            }
        }
        Pic::Bullets => {
            for i in 0..3 {
                let y = 3.5 + i as f32 * 4.5;
                painter.circle_filled(q(r, 3.0, y), 1.3 * k, color);
                line((6.0, y), (14.5, y), Stroke::new(1.2 * k, color));
            }
        }
        Pic::DropCap => {
            // A large letter over two lines, then the lines below it.
            outline(&[(1.5, 1.5), (7.5, 1.5), (7.5, 8.5), (1.5, 8.5)], thin);
            line((3.0, 7.0), (4.5, 3.0), s);
            line((4.5, 3.0), (6.0, 7.0), s);
            line((3.6, 5.6), (5.4, 5.6), thin);
            for (x0, y) in [(9.5, 2.5), (9.5, 5.5), (9.5, 8.5), (1.5, 11.5), (1.5, 14.5)] {
                line((x0, y), (14.5, y), Stroke::new(1.2 * k, color));
            }
        }
        Pic::TextProperties => {
            // A letter beside a paragraph's lines.
            line((1.5, 13.5), (4.5, 3.0), s);
            line((4.5, 3.0), (7.5, 13.5), s);
            line((2.6, 9.6), (6.4, 9.6), thin);
            for y in [4.0, 7.5, 11.0, 14.0] {
                line((9.5, y), (14.5, y), Stroke::new(1.2 * k, color));
            }
        }
        Pic::EditText => {
            // "ab" and a text cursor.
            painter.text(
                q(r, 6.0, 9.0),
                egui::Align2::CENTER_CENTER,
                "ab",
                egui::FontId::proportional(8.5 * k),
                color,
            );
            line((12.0, 2.5), (12.0, 14.0), s);
            line((10.5, 2.5), (13.5, 2.5), thin);
            line((10.5, 14.0), (13.5, 14.0), thin);
        }
        Pic::WrapText => {
            // Lines of text going round a square.
            poly(
                &[(6.0, 5.5), (10.0, 5.5), (10.0, 10.5), (6.0, 10.5)],
                color,
                Stroke::NONE,
            );
            for y in [2.5, 13.5] {
                line((1.5, y), (14.5, y), Stroke::new(1.2 * k, color));
            }
            for y in [6.0, 10.0] {
                line((1.5, y), (4.5, y), Stroke::new(1.2 * k, color));
                line((11.5, y), (14.5, y), Stroke::new(1.2 * k, color));
            }
        }
        _ => draw_node_pic(painter, r, pic, color),
    }
}

/// The Shape tool's pictures: curves with their nodes (small squares),
/// handles and marks, on the same 16 x 16 grid.
fn draw_node_pic(painter: &Painter, r: Rect, pic: Pic, color: Color32) {
    let k = r.width() / 16.0;
    let s = Stroke::new(1.1 * k, color);
    let thin = Stroke::new(0.8 * k, color);
    let line = |a: (f32, f32), b: (f32, f32), st: Stroke| {
        painter.line_segment([q(r, a.0, a.1), q(r, b.0, b.1)], st);
    };
    let node = |x: f32, y: f32, filled: bool| {
        let rr = Rect::from_center_size(q(r, x, y), Vec2::splat(3.2 * k));
        if filled {
            painter.rect_filled(rr, 0.0, color);
        } else {
            painter.rect_filled(rr, 0.0, crate::theme::Tokens::field());
            painter.rect_stroke(rr, 0.0, thin, egui::StrokeKind::Inside);
        }
    };
    // An arch from (x0, y) to (x1, y) rising by `h`.
    let arch = |x0: f32, x1: f32, y: f32, h: f32, st: Stroke| {
        let mut pts = Vec::new();
        for i in 0..=12 {
            let t = i as f32 / 12.0;
            pts.push(q(
                r,
                x0 + (x1 - x0) * t,
                y - h * (t * std::f32::consts::PI).sin(),
            ));
        }
        painter.add(epaint::PathShape::line(pts, st));
    };
    let arrow = |tip: (f32, f32), dir: (f32, f32)| {
        let (dx, dy) = dir;
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (ux, uy) = (dx / len, dy / len);
        let (px, py) = (-uy, ux);
        let back = (tip.0 - ux * 3.0, tip.1 - uy * 3.0);
        painter.add(epaint::PathShape::convex_polygon(
            vec![
                q(r, tip.0, tip.1),
                q(r, back.0 + px * 2.0, back.1 + py * 2.0),
                q(r, back.0 - px * 2.0, back.1 - py * 2.0),
            ],
            color,
            Stroke::NONE,
        ));
    };
    let dashed = |a: (f32, f32), b: (f32, f32)| {
        let n = 6;
        for i in 0..n {
            if i % 2 == 0 {
                let t0 = i as f32 / n as f32;
                let t1 = (i + 1) as f32 / n as f32;
                line(
                    (a.0 + (b.0 - a.0) * t0, a.1 + (b.1 - a.1) * t0),
                    (a.0 + (b.0 - a.0) * t1, a.1 + (b.1 - a.1) * t1),
                    thin,
                );
            }
        }
    };
    match pic {
        Pic::AddNode | Pic::DeleteNode => {
            arch(1.5, 12.5, 14.0, 8.0, s);
            node(7.0, 6.0, true);
            line((10.5, 3.0), (15.0, 3.0), s);
            if pic == Pic::AddNode {
                line((12.75, 0.8), (12.75, 5.2), s);
            }
        }
        Pic::JoinNodes => {
            arch(1.0, 8.0, 13.0, 7.0, s);
            arch(8.0, 15.0, 13.0, 7.0, s);
            node(8.0, 13.0, true);
            arrow((7.0, 11.0), (1.0, 1.0));
            arrow((9.0, 11.0), (-1.0, 1.0));
        }
        Pic::BreakCurve => {
            arch(1.0, 7.0, 13.0, 7.0, s);
            arch(9.0, 15.0, 13.0, 7.0, s);
            node(6.5, 13.0, true);
            node(9.5, 13.0, true);
        }
        Pic::ToLine => {
            line((2.0, 13.0), (14.0, 3.0), s);
            node(2.0, 13.0, true);
            node(14.0, 3.0, true);
        }
        Pic::ToCurve => {
            arch(2.0, 14.0, 13.0, 9.0, s);
            node(2.0, 13.0, true);
            node(14.0, 13.0, true);
        }
        Pic::NodeCusp => {
            line((1.5, 13.0), (8.0, 6.0), s);
            line((8.0, 6.0), (14.5, 13.0), s);
            line((8.0, 6.0), (3.0, 2.0), thin);
            line((8.0, 6.0), (13.0, 2.0), thin);
            node(8.0, 6.0, true);
        }
        Pic::NodeSmooth => {
            arch(1.5, 14.5, 14.0, 9.0, s);
            line((3.0, 5.0), (12.0, 5.0), thin);
            painter.circle_filled(q(r, 8.0, 5.0), 1.9 * k, color);
        }
        Pic::NodeSymmetrical => {
            arch(1.5, 14.5, 14.0, 9.0, s);
            line((2.5, 5.0), (13.5, 5.0), thin);
            painter.add(epaint::PathShape::convex_polygon(
                vec![
                    q(r, 8.0, 3.0),
                    q(r, 10.0, 5.0),
                    q(r, 8.0, 7.0),
                    q(r, 6.0, 5.0),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Pic::ReverseDirection => {
            arch(2.0, 14.0, 12.0, 6.0, s);
            arrow((2.0, 12.5), (-0.4, 1.0));
            node(14.0, 12.0, true);
        }
        Pic::ExtendToClose => {
            arch(1.0, 7.0, 13.0, 7.0, s);
            arch(9.0, 15.0, 13.0, 7.0, s);
            node(7.0, 13.0, true);
            node(9.0, 13.0, true);
            dashed((7.0, 13.0), (9.0, 13.0));
            line((6.0, 15.0), (10.0, 15.0), thin);
        }
        Pic::ExtractSubpath => {
            let rr = Rect::from_min_max(q(r, 1.5, 4.5), q(r, 10.5, 14.5));
            painter.rect_stroke(rr, 0.0, thin, egui::StrokeKind::Middle);
            let inner = Rect::from_min_max(q(r, 6.5, 1.5), q(r, 14.5, 9.5));
            painter.rect_filled(inner, 0.0, crate::theme::Tokens::field());
            painter.rect_stroke(inner, 0.0, s, egui::StrokeKind::Middle);
            arrow((14.5, 1.5), (1.0, -1.0));
        }
        Pic::CloseCurve => {
            let mut pts = Vec::new();
            for i in 0..=14 {
                let a = std::f32::consts::PI * (0.35 + 1.3 * i as f32 / 14.0);
                pts.push(q(r, 8.0 + 6.0 * a.cos(), 8.0 + 6.0 * a.sin()));
            }
            painter.add(epaint::PathShape::line(pts, s));
            let a0 = std::f32::consts::PI * 0.35;
            let a1 = std::f32::consts::PI * 1.65;
            let p0 = (8.0 + 6.0 * a0.cos(), 8.0 + 6.0 * a0.sin());
            let p1 = (8.0 + 6.0 * a1.cos(), 8.0 + 6.0 * a1.sin());
            dashed(p1, p0);
            node(p0.0, p0.1, true);
            node(p1.0, p1.1, true);
        }
        Pic::PenPreview => {
            arch(1.5, 9.0, 12.0, 6.0, s);
            node(9.0, 12.0, true);
            dashed((9.0, 12.0), (13.0, 4.0));
            // The pointer's cross hair.
            line((13.0, 1.0), (13.0, 7.0), thin);
            line((10.0, 4.0), (16.0, 4.0), thin);
        }
        Pic::AutoAddDelete => {
            arch(1.5, 14.5, 13.0, 7.0, s);
            node(8.0, 8.5, true);
            line((1.0, 3.0), (5.0, 3.0), s);
            line((3.0, 1.0), (3.0, 5.0), s);
            line((11.0, 3.0), (15.0, 3.0), s);
        }
        Pic::StretchNodes | Pic::RotateNodes => {
            let rr = Rect::from_min_max(q(r, 3.5, 3.5), q(r, 12.5, 12.5));
            let pts = [(3.5, 3.5), (12.5, 3.5), (12.5, 12.5), (3.5, 12.5)];
            for i in 0..4 {
                dashed(pts[i], pts[(i + 1) % 4]);
            }
            let _ = rr;
            if pic == Pic::StretchNodes {
                for (x, y) in pts {
                    node(x, y, true);
                }
                line((12.5, 3.5), (15.0, 1.0), thin);
                arrow((15.0, 1.0), (1.0, -1.0));
            } else {
                let mut arc = Vec::new();
                for i in 0..=8 {
                    let a = -std::f32::consts::FRAC_PI_2 * i as f32 / 8.0;
                    arc.push(q(r, 8.0 + 6.5 * a.cos(), 8.0 + 6.5 * a.sin()));
                }
                painter.add(epaint::PathShape::line(arc, s));
                arrow((8.0, 1.5), (-1.0, 0.0));
                node(8.0, 8.0, false);
            }
        }
        Pic::AlignNodes => {
            line((1.0, 9.0), (15.0, 9.0), thin);
            for x in [3.0, 8.0, 13.0] {
                node(x, 9.0, true);
            }
            arrow((3.0, 7.0), (0.0, 1.0));
            arrow((13.0, 11.0), (0.0, -1.0));
        }
        Pic::ReflectH => {
            dashed((8.0, 1.0), (8.0, 15.0));
            node(3.0, 8.0, true);
            node(13.0, 8.0, true);
            arrow((1.0, 4.0), (-1.0, 0.0));
            arrow((15.0, 4.0), (1.0, 0.0));
            line((1.5, 4.0), (6.0, 4.0), thin);
            line((10.0, 4.0), (14.5, 4.0), thin);
        }
        Pic::ReflectV => {
            dashed((1.0, 8.0), (15.0, 8.0));
            node(8.0, 3.0, true);
            node(8.0, 13.0, true);
            arrow((4.0, 1.0), (0.0, -1.0));
            arrow((4.0, 15.0), (0.0, 1.0));
            line((4.0, 1.5), (4.0, 6.0), thin);
            line((4.0, 10.0), (4.0, 14.5), thin);
        }
        Pic::Elastic => {
            let pts: Vec<Pos2> = [
                (1.0, 8.0),
                (3.0, 8.0),
                (4.5, 4.0),
                (6.5, 12.0),
                (8.5, 4.0),
                (10.5, 12.0),
                (12.0, 8.0),
                (15.0, 8.0),
            ]
            .iter()
            .map(|(x, y)| q(r, *x, *y))
            .collect();
            painter.add(epaint::PathShape::line(pts, s));
            node(1.5, 8.0, true);
            node(14.5, 8.0, true);
        }
        Pic::SelectAllNodes => {
            arch(1.5, 14.5, 13.0, 9.0, s);
            for (x, y) in [
                (1.5, 13.0),
                (4.5, 6.5),
                (8.0, 4.0),
                (11.5, 6.5),
                (14.5, 13.0),
            ] {
                node(x, y, true);
            }
        }
        Pic::ReduceNodes => {
            arch(1.5, 14.5, 13.0, 9.0, s);
            node(1.5, 13.0, true);
            node(14.5, 13.0, true);
            for (x, y) in [(4.5, 6.5), (11.5, 6.5)] {
                node(x, y, false);
            }
            line((5.5, 1.5), (10.5, 1.5), s);
        }
        Pic::Star | Pic::ComplexStar => {
            let vertex = |i: usize, rad: f32| {
                let a = std::f32::consts::FRAC_PI_2 - i as f32 * std::f32::consts::TAU / 10.0;
                (8.0 + rad * a.cos(), 8.6 - rad * a.sin())
            };
            let pts: Vec<(f32, f32)> = if pic == Pic::Star {
                (0..10)
                    .map(|i| vertex(i, if i % 2 == 0 { 7.2 } else { 3.0 }))
                    .collect()
            } else {
                // Every second vertex of five: the sides cross.
                [0, 4, 8, 2, 6].iter().map(|i| vertex(*i, 7.2)).collect()
            };
            painter.add(epaint::PathShape::closed_line(
                pts.iter().map(|(x, y)| q(r, *x, *y)).collect(),
                s,
            ));
        }
        _ => draw_arrange_pic(painter, r, pic, color),
    }
}

/// Fill a simple polygon, convex or not (cut into triangles by ear
/// clipping), with an edge stroke so the border is smooth.
fn fill_polygon(painter: &Painter, pts: &[Pos2], color: Color32, edge: f32) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let turn = |a: Pos2, b: Pos2, c: Pos2| (b - a).x * (c - b).y - (b - a).y * (c - b).x;
    let inside = |p: Pos2, a: Pos2, b: Pos2, c: Pos2| {
        let (d1, d2, d3) = (turn(a, b, p), turn(b, c, p), turn(c, a, p));
        (d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0) || (d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0)
    };
    let mut mesh = epaint::Mesh::default();
    for p in pts {
        mesh.colored_vertex(*p, color);
    }
    let mut idx: Vec<usize> = (0..n).collect();
    while idx.len() > 3 {
        let m = idx.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let t = turn(pts[a], pts[b], pts[c]);
            t * area > 0.0
                && !idx
                    .iter()
                    .any(|&j| j != a && j != b && j != c && inside(pts[j], pts[a], pts[b], pts[c]))
        });
        let Some(i) = ear else {
            break;
        };
        let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
        mesh.add_triangle(a as u32, b as u32, c as u32);
        idx.remove(i);
    }
    if idx.len() == 3 {
        mesh.add_triangle(idx[0] as u32, idx[1] as u32, idx[2] as u32);
    }
    painter.add(mesh);
    painter.add(epaint::PathShape::closed_line(
        pts.to_vec(),
        Stroke::new(edge, color),
    ));
}

/// The arranging pictures. The shaping ones show a square (behind) and a
/// circle (in front, centred on the square's lower right corner); the
/// result is filled, a removed object dashed, a kept one outlined.
fn draw_arrange_pic(painter: &Painter, r: Rect, pic: Pic, color: Color32) {
    let k = r.width() / 16.0;
    let thin = Stroke::new(0.8 * k, color);
    let edge = 0.6 * k;
    let p = |x: f32, y: f32| q(r, x, y);
    // The circle's points from angle `a0` to `a1` (degrees, y down).
    let arc = |cx: f32, cy: f32, rad: f32, a0: f32, a1: f32| -> Vec<Pos2> {
        (0..=16)
            .map(|i| {
                let t = (a0 + (a1 - a0) * i as f32 / 16.0).to_radians();
                p(cx + rad * t.cos(), cy + rad * t.sin())
            })
            .collect()
    };
    let square = [p(1.5, 1.5), p(10.0, 1.5), p(10.0, 10.0), p(1.5, 10.0)];
    let circle = || arc(10.0, 10.0, 5.0, 0.0, 360.0);
    let union = || {
        let mut v = vec![p(1.5, 1.5), p(10.0, 1.5)];
        v.extend(arc(10.0, 10.0, 5.0, -90.0, 180.0));
        v.push(p(1.5, 10.0));
        v
    };
    let back_minus_front = || {
        let mut v = vec![p(1.5, 1.5), p(10.0, 1.5)];
        v.extend(arc(10.0, 10.0, 5.0, -90.0, -180.0));
        v.push(p(1.5, 10.0));
        v
    };
    let front_minus_back = || {
        let mut v = arc(10.0, 10.0, 5.0, -90.0, 180.0);
        v.push(p(10.0, 10.0));
        v
    };
    let both = || {
        let mut v = vec![p(10.0, 10.0)];
        v.extend(arc(10.0, 10.0, 5.0, -90.0, -180.0));
        v
    };
    let outline = |pts: Vec<Pos2>, st: Stroke| {
        painter.add(epaint::PathShape::closed_line(pts, st));
    };
    let dashed = |mut pts: Vec<Pos2>| {
        if let Some(first) = pts.first().copied() {
            pts.push(first);
        }
        painter.extend(egui::Shape::dashed_line(&pts, thin, 1.3 * k, 1.7 * k));
    };
    // Corner marks around a box, as a group's selection shows.
    let brackets = |x0: f32, y0: f32, x1: f32, y1: f32| {
        let l = 2.5;
        for (cx, cy, dx, dy) in [
            (x0, y0, 1.0, 1.0),
            (x1, y0, -1.0, 1.0),
            (x1, y1, -1.0, -1.0),
            (x0, y1, 1.0, -1.0),
        ] {
            painter.add(epaint::PathShape::line(
                vec![p(cx + dx * l, cy), p(cx, cy), p(cx, cy + dy * l)],
                thin,
            ));
        }
    };
    match pic {
        Pic::Weld => fill_polygon(painter, &union(), color, edge),
        Pic::Trim => {
            fill_polygon(painter, &back_minus_front(), color, edge);
            outline(circle(), thin);
        }
        Pic::Intersect => {
            outline(square.to_vec(), thin);
            outline(circle(), thin);
            fill_polygon(painter, &both(), color, edge);
        }
        Pic::Simplify => {
            outline(back_minus_front(), thin);
            fill_polygon(painter, &circle(), color, edge);
        }
        Pic::FrontMinusBack => {
            dashed(square.to_vec());
            fill_polygon(painter, &front_minus_back(), color, edge);
        }
        Pic::BackMinusFront => {
            dashed(circle());
            fill_polygon(painter, &back_minus_front(), color, edge);
        }
        Pic::Boundary => {
            outline(square.to_vec(), thin);
            outline(circle(), thin);
            // The outline around both, a little outside them.
            let mut v = vec![p(0.5, 0.5), p(11.0, 0.5)];
            v.extend(arc(10.0, 10.0, 6.0, -80.0, 170.0));
            v.push(p(0.5, 11.0));
            dashed(v);
        }
        Pic::Combine => {
            // One object with both outlines: where they overlap is a hole.
            fill_polygon(painter, &back_minus_front(), color, edge);
            fill_polygon(painter, &front_minus_back(), color, edge);
        }
        Pic::BreakApart => {
            fill_polygon(
                painter,
                &[p(1.0, 1.0), p(7.0, 1.0), p(7.0, 7.0), p(1.0, 7.0)],
                color,
                edge,
            );
            fill_polygon(painter, &arc(11.5, 11.5, 3.5, 0.0, 360.0), color, edge);
            painter.line_segment([p(5.2, 10.8), p(10.8, 5.2)], thin);
            for (a, b, c) in [
                ((4.5, 11.5), (4.5, 8.8), (7.2, 11.5)),
                ((11.5, 4.5), (8.8, 4.5), (11.5, 7.2)),
            ] {
                painter.add(epaint::PathShape::convex_polygon(
                    vec![p(a.0, a.1), p(b.0, b.1), p(c.0, c.1)],
                    color,
                    Stroke::NONE,
                ));
            }
        }
        Pic::Group | Pic::Ungroup | Pic::UngroupAll => {
            fill_polygon(
                painter,
                &[p(3.5, 3.5), p(8.5, 3.5), p(8.5, 8.5), p(3.5, 8.5)],
                color,
                edge,
            );
            fill_polygon(painter, &arc(10.5, 10.5, 2.8, 0.0, 360.0), color, edge);
            match pic {
                Pic::Group => brackets(1.0, 1.0, 15.0, 15.0),
                Pic::Ungroup => {
                    brackets(1.5, 1.5, 9.5, 9.5);
                    brackets(6.8, 6.8, 14.5, 14.5);
                }
                _ => {
                    brackets(0.5, 0.5, 15.5, 15.5);
                    brackets(2.3, 2.3, 9.7, 9.7);
                    brackets(6.9, 6.9, 14.1, 14.1);
                }
            }
        }
        Pic::AlignDistribute => {
            painter.line_segment([p(2.0, 1.0), p(2.0, 15.0)], thin);
            for (y, w) in [(2.5, 10.0), (6.5, 6.5), (10.5, 12.5)] {
                fill_polygon(
                    painter,
                    &[
                        p(3.0, y),
                        p(3.0 + w, y),
                        p(3.0 + w, y + 3.0),
                        p(3.0, y + 3.0),
                    ],
                    color,
                    edge,
                );
            }
        }
        _ => {}
    }
}

pub(crate) fn frame(ui: &Ui, rect: Rect, hovered: bool, pressed: bool) {
    if pressed {
        ui.painter()
            .rect_filled(rect, 0.0, crate::theme::Tokens::field());
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            Stroke::new(1.0, Tokens::control_border()),
            egui::StrokeKind::Middle,
        );
    } else if hovered {
        ui.painter()
            .rect_filled(rect, 0.0, crate::theme::Tokens::hovered_row());
        ui.painter().rect_stroke(
            rect.shrink(0.5),
            0.0,
            Stroke::new(1.0, crate::theme::Tokens::selected_row()),
            egui::StrokeKind::Middle,
        );
    }
}

/// An icon button of the bar; `pressed` draws it pushed in.
pub fn pic_button(ui: &mut Ui, pic: Pic, tip: &str, enabled: bool, pressed: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::splat(btn()),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    frame(ui, rect, enabled && resp.hovered(), pressed);
    draw_pic(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
        pic,
        if enabled {
            Tokens::icon()
        } else {
            Tokens::border()
        },
    );
    resp.rich_tip(tip)
}

/// A thin separator between groups.
pub fn sep(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, BAR_H), Sense::hover());
    let x = rect.center().x.round() + 0.5;
    ui.painter().vline(
        x,
        (rect.top() + 2.0)..=(rect.bottom() - 2.0),
        Stroke::new(1.0, crate::theme::Tokens::border()),
    );
}

/// A small label picture before a field.
pub(crate) fn label_pic(ui: &mut Ui, pic: Pic, tip: &str) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(16.0, ROW), Sense::hover());
    draw_pic(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(13.0)),
        pic,
        Tokens::icon(),
    );
    resp.rich_tip(tip);
}

/// A row with a label picture and what follows it (a field), all of it
/// showing the tip of `key` (its name and the `key_desc` description).
pub(crate) fn tip_row<R>(ui: &mut Ui, pic: Pic, key: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let tip = crate::ui::tips::keyed(key);
    crate::ui::tips::tip_over(ui, &tip, |ui| {
        row(ui, |ui| {
            draw_label_pic(ui, pic);
            add(ui)
        })
    })
}

/// [`tip_row`] with a text label ("X:") in place of the picture.
pub(crate) fn tip_text_row<R>(
    ui: &mut Ui,
    label: &str,
    key: &str,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let tip = crate::ui::tips::keyed(key);
    crate::ui::tips::tip_over(ui, &tip, |ui| {
        row(ui, |ui| {
            label_text(ui, label);
            add(ui)
        })
    })
}

/// A label picture without a tip of its own.
fn draw_label_pic(ui: &mut Ui, pic: Pic) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(16.0, ROW), Sense::hover());
    draw_pic(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(13.0)),
        pic,
        Tokens::icon(),
    );
}

/// A text label before a field ("X:", "Units:").
pub(crate) fn label_text(ui: &mut Ui, text: &str) {
    ui.add_sized(
        [16.0, ROW],
        egui::Label::new(egui::RichText::new(text).size(12.0)),
    );
}

/// A distance field in the ruler unit, through the drawing scale (a
/// position, a size, an offset); with spin arrows when `step` is given (in
/// the unit). Returns the new value in mm once committed.
pub(crate) fn distance(
    ui: &mut Ui,
    app: &App,
    mm: f64,
    width: f32,
    step: Option<f64>,
) -> Option<f64> {
    distance_in(ui, app, mm, width, step, app.units.world_mm())
}

/// A page size field: in the ruler unit as printed, not through the
/// drawing scale.
fn paper_distance(ui: &mut Ui, app: &App, mm: f64, width: f32, step: Option<f64>) -> Option<f64> {
    distance_in(ui, app, mm, width, step, app.units.mm())
}

/// A distance field showing `mm` in units of `unit_mm` millimetres.
fn distance_in(
    ui: &mut Ui,
    app: &App,
    mm: f64,
    width: f32,
    step: Option<f64>,
    unit_mm: f64,
) -> Option<f64> {
    distance_in_mixed(ui, app, mm, width, step, unit_mm, false)
}

/// [`distance`] for a value the selected objects do not share: the field
/// is empty, a typed value goes to all of them.
fn distance_mixed(ui: &mut Ui, app: &App, mm: f64, width: f32, mixed: bool) -> Option<f64> {
    distance_in_mixed(ui, app, mm, width, Some(1.0), app.units.world_mm(), mixed)
}

fn distance_in_mixed(
    ui: &mut Ui,
    app: &App,
    mm: f64,
    width: f32,
    step: Option<f64>,
    unit_mm: f64,
    mixed: bool,
) -> Option<f64> {
    let u = app.units;
    let d = app.settings.precision.min(6) as usize;
    let unit_mm = if unit_mm.is_finite() && unit_mm > 0.0 {
        unit_mm
    } else {
        1.0
    };
    let mut v = mm / unit_mm;
    // The field's own arrows and wheel step by `step` units; the bar had
    // a second pair beside them doing the same (#488).
    let r = ui.add_sized(
        [width, ROW],
        crate::ui::field::NumField::new(&mut v)
            .fixed_decimals(d.min(3))
            .max_decimals(d)
            .suffix(format!(" {}", u.short()))
            .unit_mm(u.mm())
            .step(step.unwrap_or(1.0))
            .mixed(mixed),
    );
    // Typed values apply with Enter or when the field is left.
    if r.changed() && (mixed || (v * unit_mm - mm).abs() > 1e-12) {
        return Some(v * unit_mm);
    }
    None
}

/// Two rows stacked in the bar's height.
pub(crate) fn stacked<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        Vec2::new(0.0, BAR_H),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            // The second field's top border is the first one's bottom.
            ui.spacing_mut().item_spacing.y = -1.0;
            ui.spacing_mut().interact_size.y = ROW;
            ui.spacing_mut().button_padding.y = 0.0;
            add(ui)
        },
    )
    .inner
}

pub(crate) fn row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.spacing_mut().button_padding.y = 0.0;
        ui.set_height(ROW);
        add(ui)
    })
    .inner
}

// ----- the page bar -------------------------------------------------------

/// Pick tool with nothing selected: page size, orientation, the pages it
/// applies to, units, nudge and duplicate distances, "treat as filled".
/// The property bar with guidelines selected and no object: the kind of
/// the one selected guideline and its x, y and angle (counted from the
/// ruler origin, as the Guidelines panel shows them), each moving it as
/// typed; with several selected, how many.
pub fn guide_bar(app: &mut App, ui: &mut Ui) {
    use crate::ui::layout_options::GuideKind;
    use tracedraw_core::document::GuideLine;
    let guides = app.page_guides();
    let single = match app.selected_guides.as_slice() {
        [i] => guides.get(*i).copied(),
        _ => None,
    };
    let Some(g) = single else {
        ui.label(crate::i18n::trf(
            "toolbar.guides_selected",
            &[("n", &app.selected_guides.len().to_string())],
        ));
        return;
    };
    let kind = GuideKind::of(&g.line);
    ui.label(tr(kind.key()));
    sep(ui);
    let (o, _) = g.line.point_and_direction();
    let r = app.to_ruler(o);
    let (mut x, mut y) = (r.x, r.y);
    let mut angle = match g.line {
        GuideLine::Angled { angle, .. } => angle,
        _ => 0.0,
    };
    let mut changed = false;
    let unit = app.units.world_mm();
    if kind != GuideKind::Horizontal {
        ui.label(tr("guides.x"));
        if let Some(v) = distance_in(ui, app, x, 90.0, Some(1.0), unit) {
            x = v;
            changed = true;
        }
    }
    if kind != GuideKind::Vertical {
        ui.label(tr("guides.y"));
        if let Some(v) = distance_in(ui, app, y, 90.0, Some(1.0), unit) {
            y = v;
            changed = true;
        }
    }
    if kind == GuideKind::Angled {
        ui.label(tr("guides.angle"));
        if ui
            .add_sized(
                [70.0, ROW],
                crate::ui::field::NumField::new(&mut angle)
                    .speed(1.0)
                    .max_decimals(2)
                    .suffix("°"),
            )
            .changed()
        {
            changed = true;
        }
    }
    if g.locked {
        sep(ui);
        ui.label(tr("guides.locked"));
    }
    if changed {
        let p = app.from_ruler(Point::new(x, y));
        let line = match kind {
            GuideKind::Horizontal => GuideLine::Horizontal { y: p.y },
            GuideKind::Vertical => GuideLine::Vertical { x: p.x },
            GuideKind::Angled => GuideLine::Angled {
                x: p.x,
                y: p.y,
                angle,
            },
        };
        app.set_selected_guide_line(line);
    }
}

/// The page bar's list of sizes: the presets by group, then Custom
/// (#650), which is the one ticked when the page matches no preset and
/// opens the Page Size dialog with the page's width and height to type
/// any size, and Edit This List, which opens Document Options > Page Size
/// to save or delete sizes.
fn page_size_list(app: &mut App, ui: &mut Ui, current: &str, size: Size) {
    let saved = app.settings.page_sizes.clone();
    if let Some((_, s)) = crate::ui::dialogs::paper_preset_menu(ui, current, &saved) {
        let landscape = size.width > size.height;
        let s = if landscape {
            Size::new(s.height.max(s.width), s.height.min(s.width))
        } else {
            Size::new(s.width.min(s.height), s.width.max(s.height))
        };
        resize_pages(app, s);
        app.fit_pending = true;
    }
    ui.separator();
    let custom = crate::ui::dialogs::paper_name(size, &saved).is_none();
    if ui
        .selectable_label(custom, tr("toolbar.custom_page"))
        .clicked()
    {
        app.dialog = crate::ui::dialogs::Dialog::PageSize {
            width: size.width,
            height: size.height,
            all_pages: app.page_size_all,
        };
    }
    if ui.button(tr("toolbar.edit_page_sizes")).clicked() {
        app.dialog = crate::ui::dialogs::Dialog::Options;
        app.options_page = crate::ui::dialogs::OptionsPage::PageSize;
        ui.close();
    }
}

/// Autofit Page's popup (there is one property bar).
pub(crate) fn autofit_popup_id() -> egui::Id {
    egui::Id::new("autofit_page_popup")
}

pub fn page_bar(app: &mut App, ui: &mut Ui) {
    let size = app.page_size();
    let current = crate::ui::dialogs::paper_name(size, &app.settings.page_sizes)
        .unwrap_or_else(|| tr("toolbar.custom"));
    ui.allocate_ui_with_layout(
        Vec2::new(200.0, BAR_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            egui::ComboBox::from_id_salt("page_size_bar")
                .selected_text(current.clone())
                .width(192.0)
                .height(400.0)
                .show_ui(ui, |ui| page_size_list(app, ui, &current, size))
                .response
                .rich_tip(crate::ui::tips::keyed("toolbar.page_size"));
        },
    );
    let mut new_size = None;
    stacked(ui, |ui| {
        tip_row(ui, Pic::PageWidth, "toolbar.page_width", |ui| {
            if let Some(w) = paper_distance(ui, app, size.width, 92.0, Some(1.0)) {
                new_size = Some(Size::new(w, size.height));
            }
        });
        tip_row(ui, Pic::PageHeight, "toolbar.page_height", |ui| {
            if let Some(h) = paper_distance(ui, app, size.height, 92.0, Some(1.0)) {
                new_size = Some(Size::new(size.width, h));
            }
        });
    });
    if let Some(s) = new_size {
        if s.width > 0.1 && s.height > 0.1 {
            resize_pages(app, s);
        }
    }
    // Fit page to drawing, as Layout has it (#236); pale on an empty
    // page.
    let has_objects = app
        .doc()
        .page(app.page)
        .ok()
        .is_some_and(|p| p.layers.iter().any(|l| !l.shapes.is_empty()));
    // Autofit Page: a popup with the margin to leave around the drawing
    // (kept in the preferences) and the button that fits the page.
    let fit = pic_button(
        ui,
        Pic::FitPage,
        &tr("toolbar.fit_page_tip"),
        has_objects,
        false,
    );
    let popup_id = autofit_popup_id();
    if fit.clicked() {
        egui::Popup::toggle_id(ui.ctx(), popup_id);
    }
    let ctx = ui.ctx().clone();
    let mut fitted = false;
    // Clicks inside (the margin field, the Live preview box) keep it
    // open, so the preview can be switched and watched (#659).
    egui::Popup::new(popup_id, ctx.clone(), fit.rect, ui.layer_id())
        .open_memory(None)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(190.0);
            // The Live preview box on the margin's row, right of the field.
            ui.horizontal(|ui| {
                ui.label(tr("toolbar.fit_margin"));
                ui.add_space(8.0);
                let margin = app.settings.fit_page_margin.max(0.0);
                if let Some(m) = paper_distance(ui, app, margin, 88.0, Some(1.0)) {
                    app.settings.fit_page_margin = m.max(0.0);
                }
                ui.add_space(6.0);
                ui.checkbox(
                    &mut app.settings.fit_page_preview,
                    tr("toolbar.autofit_live_preview"),
                );
            });
            ui.add_space(4.0);
            if ui
                .add_sized(
                    Vec2::new(ui.available_width(), 24.0),
                    egui::Button::new(tr("toolbar.autofit_page")),
                )
                .clicked()
            {
                fitted = true;
            }
        });
    // While the popup is open the page it would make shows on the canvas,
    // following the margin as it is typed (#594), unless Live preview is
    // off (#659).
    let preview = app.settings.fit_page_preview && egui::Popup::is_id_open(&ctx, popup_id);
    app.autofit_preview = preview.then(|| {
        (
            app.settings.fit_page_margin.max(0.0),
            ctx.cumulative_frame_nr(),
        )
    });
    if fitted {
        app.fit_page_to_drawing_with_margin(app.settings.fit_page_margin);
        app.fit_pending = true;
        egui::Popup::close_id(&ctx, popup_id);
    }
    sep(ui);
    let portrait = size.height >= size.width;
    if pic_button(ui, Pic::Portrait, &tr("dialog.portrait"), true, portrait).clicked() && !portrait
    {
        resize_pages(app, Size::new(size.height, size.width));
        app.fit_pending = true;
    }
    if pic_button(ui, Pic::Landscape, &tr("dialog.landscape"), true, !portrait).clicked()
        && portrait
    {
        resize_pages(app, Size::new(size.height, size.width));
        app.fit_pending = true;
    }
    sep(ui);
    if pic_button(
        ui,
        Pic::AllPages,
        &tr("toolbar.all_pages_tip"),
        true,
        app.page_size_all,
    )
    .clicked()
    {
        app.page_size_all = true;
    }
    if pic_button(
        ui,
        Pic::CurrentPage,
        &tr("toolbar.current_page_tip"),
        true,
        !app.page_size_all,
    )
    .clicked()
    {
        app.page_size_all = false;
    }
    sep(ui);
    ui.label(tr("toolbar.units_label"));
    let mut u = app.units;
    egui::ComboBox::from_id_salt("units_bar")
        .selected_text(u.label())
        .width(120.0)
        .height(400.0)
        .show_ui(ui, |ui| {
            for v in Units::ALL {
                ui.selectable_value(&mut u, v, v.label());
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("toolbar.units"));
    app.units = u;
    sep(ui);
    tip_row(ui, Pic::Nudge, "toolbar.nudge_tip", |ui| {
        if let Some(n) = distance(ui, app, app.nudge_mm, 84.0, Some(0.1)) {
            if n > 0.0 {
                app.nudge_mm = n;
            }
        }
    });
    sep(ui);
    let dup = app.duplicate_offset;
    let mut new_dup = None;
    stacked(ui, |ui| {
        tip_row(ui, Pic::DupX, "toolbar.duplicate_x_tip", |ui| {
            if let Some(x) = distance(ui, app, dup.x, 92.0, Some(0.1)) {
                new_dup = Some(tracedraw_core::geometry::Vec2::new(x, dup.y));
            }
        });
        tip_row(ui, Pic::DupY, "toolbar.duplicate_y_tip", |ui| {
            if let Some(y) = distance(ui, app, dup.y, 92.0, Some(0.1)) {
                new_dup = Some(tracedraw_core::geometry::Vec2::new(dup.x, y));
            }
        });
    });
    if let Some(d) = new_dup {
        app.duplicate_offset = d;
    }
    sep(ui);
    let filled = app.settings.treat_all_filled;
    if pic_button(
        ui,
        Pic::TreatFilled,
        &tr("options.treat_all_filled"),
        true,
        filled,
    )
    .clicked()
    {
        app.settings.treat_all_filled = !filled;
    }
}

/// Resize the current page, or every page when "all pages" is on, as one
/// undo step.
pub fn resize_pages(app: &mut App, size: Size) {
    let pages: Vec<_> = if app.page_size_all {
        app.doc().pages.iter().map(|p| p.id).collect()
    } else {
        vec![app.page]
    };
    let cmds: Vec<Command> = pages
        .into_iter()
        .map(|page| Command::ResizePage { page, size })
        .collect();
    if let Err(e) = app.engine.run_batch("Page Size", &cmds) {
        app.status = e.to_string();
    }
}

// ----- the object bar ---------------------------------------------------

/// The reference point the X and Y fields show: (column, row) of the 3 x 3
/// object origin selector, (1, 1) the centre.
pub fn origin_point(b: tracedraw_core::geometry::Rect, origin: (u8, u8)) -> Point {
    let x = match origin.0 {
        0 => b.x0,
        1 => b.center().x,
        _ => b.x1,
    };
    // Row 0 is the top (largest y).
    let y = match origin.1 {
        0 => b.y1,
        1 => b.center().y,
        _ => b.y0,
    };
    Point::new(x, y)
}

/// The 3 x 3 object origin selector.
fn origin_selector(ui: &mut Ui, app: &mut App) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(22.0, BAR_H), Sense::click());
    let resp = resp.rich_tip(tr("toolbar.object_origin"));
    let cell = 5.0;
    let gap = 2.0;
    let total = cell * 3.0 + gap * 2.0;
    let o = Pos2::new(
        (rect.center().x - total / 2.0).round(),
        (rect.center().y - total / 2.0).round(),
    );
    for j in 0..3u8 {
        for i in 0..3u8 {
            let r = Rect::from_min_size(
                o + Vec2::new(i as f32 * (cell + gap), j as f32 * (cell + gap)),
                Vec2::splat(cell),
            );
            if app.object_origin == (i, j) {
                ui.painter().rect_filled(r, 0.0, Tokens::icon());
            } else {
                ui.painter()
                    .rect_filled(r, 0.0, crate::theme::Tokens::field());
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    Stroke::new(1.0, Tokens::icon()),
                    egui::StrokeKind::Inside,
                );
            }
            if resp.clicked() {
                if let Some(p) = resp.interact_pointer_pos() {
                    if r.expand(gap / 2.0).contains(p) {
                        app.object_origin = (i, j);
                    }
                }
            }
        }
    }
}

/// The rotation of the single selected object (degrees), from its
/// transform.
fn selection_angle(app: &App) -> f64 {
    let shapes = app.selected_rc();
    if shapes.len() != 1 {
        return 0.0;
    }
    let c = shapes[0].transform.as_coeffs();
    let a = c[1].atan2(c[0]).to_degrees();
    if a.abs() < 1e-9 {
        0.0
    } else {
        a
    }
}

/// Position, size, scale, rotation and mirroring of the selection.
/// The rotation angle: the selection's own, turned about `refp` by what
/// is typed or stepped, each arrow click or wheel turn on its own.
fn rotation_field(app: &mut App, ui: &mut Ui, refp: Point) -> egui::Response {
    let angle = selection_angle(app);
    let mut a = angle;
    let r = ui.add_sized(
        [64.0, ROW + 4.0],
        crate::ui::field::NumField::new(&mut a)
            .speed(1.0)
            .fixed_decimals(1)
            .suffix(" \u{00B0}"),
    );
    if r.changed() && (a - angle).abs() > 1e-9 {
        app.transform_selection(
            Affine::translate(refp.to_vec2())
                * Affine::rotate((a - angle).to_radians())
                * Affine::translate(-refp.to_vec2()),
        );
    }
    r
}

pub fn object_bar(app: &mut App, ui: &mut Ui) {
    let Some(b) = app.selection_bounds() else {
        return;
    };
    origin_selector(ui, app);
    let o = app.ruler_origin();
    let refp = origin_point(b, app.object_origin);
    let mut moved: Option<Point> = None;
    stacked(ui, |ui| {
        tip_text_row(ui, "X:", "toolbar.object_x", |ui| {
            if let Some(x) = distance(ui, app, refp.x - o.x, 100.0, None) {
                moved = Some(Point::new(x + o.x, refp.y));
            }
        });
        tip_text_row(ui, "Y:", "toolbar.object_y", |ui| {
            if let Some(y) = distance(ui, app, refp.y - o.y, 100.0, None) {
                moved = Some(Point::new(refp.x, y + o.y));
            }
        });
    });
    if let Some(p) = moved {
        app.transform_selection(Affine::translate(p - refp));
    }
    // Size, about the reference point; the lock keeps proportions.
    let (w, h) = (b.width(), b.height());
    let mut size: Option<(f64, f64)> = None;
    let locked = app.scale_locked;
    stacked(ui, |ui| {
        tip_row(ui, Pic::Width, "toolbar.object_width", |ui| {
            if let Some(nw) = distance(ui, app, w, 92.0, None) {
                let nh = if locked && w > 1e-9 { h * nw / w } else { h };
                size = Some((nw, nh));
            }
        });
        tip_row(ui, Pic::Height, "toolbar.object_height", |ui| {
            if let Some(nh) = distance(ui, app, h, 92.0, None) {
                let nw = if locked && h > 1e-9 { w * nh / h } else { w };
                size = Some((nw, nh));
            }
        });
    });
    // Scale factors (relative: the field shows 100% and applies the change).
    let mut scale: Option<(f64, f64)> = None;
    stacked(ui, |ui| {
        for horizontal in [true, false] {
            let key = if horizontal {
                "toolbar.scale_x"
            } else {
                "toolbar.scale_y"
            };
            crate::ui::tips::tip_over(ui, crate::ui::tips::keyed(key), |ui| {
                row(ui, |ui| {
                    let mut pct = 100.0f64;
                    let r = ui.add_sized(
                        [64.0, ROW],
                        crate::ui::field::NumField::new(&mut pct)
                            .speed(0.5)
                            .fixed_decimals(1)
                            .suffix(" %"),
                    );
                    if r.changed() && (pct - 100.0).abs() > 1e-9 && pct > 0.0 {
                        let f = pct / 100.0;
                        scale = Some(if locked {
                            (f, f)
                        } else if horizontal {
                            (f, 1.0)
                        } else {
                            (1.0, f)
                        });
                    }
                })
            });
        }
    });
    if pic_button(
        ui,
        Pic::Lock(locked),
        &tr("toolbar.lock_ratio"),
        true,
        locked,
    )
    .clicked()
    {
        app.scale_locked = !locked;
    }
    if let Some((nw, nh)) = size {
        if nw > 1e-6 && nh > 1e-6 && w > 1e-9 && h > 1e-9 {
            scale = Some((nw / w, nh / h));
        }
    }
    if let Some((sx, sy)) = scale {
        app.transform_selection(
            Affine::translate(refp.to_vec2())
                * Affine::scale_non_uniform(sx, sy)
                * Affine::translate(-refp.to_vec2()),
        );
    }
    sep(ui);
    crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("toolbar.rotation_angle"), |ui| {
        draw_label_pic(ui, Pic::Rotate);
        rotation_field(app, ui, refp);
    });
    sep(ui);
    if pic_button(ui, Pic::MirrorH, &tr("toolbar.mirror_h"), true, false).clicked() {
        app.mirror(true);
    }
    if pic_button(ui, Pic::MirrorV, &tr("toolbar.mirror_v"), true, false).clicked() {
        app.mirror(false);
    }
}

/// Outline widths the list offers in millimetres.
pub(crate) const OUTLINE_WIDTHS_MM: [f64; 10] =
    [0.1, 0.2, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 2.5, 3.0];
/// ... in points.
const OUTLINE_WIDTHS_PT: [f64; 13] = [
    0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 10.0, 12.0, 16.0,
];
/// ... in inches.
const OUTLINE_WIDTHS_IN: [f64; 9] = [0.003, 0.005, 0.01, 0.015, 0.02, 0.03, 0.05, 0.08, 0.1];
/// ... in pixels.
const OUTLINE_WIDTHS_PX: [f64; 10] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 16.0];

/// The units an outline width can be shown and typed in.
pub const OUTLINE_UNITS: [Units; 5] = [
    Units::Points,
    Units::Millimeters,
    Units::Centimeters,
    Units::Inches,
    Units::Pixels,
];

/// The widths the outline width lists offer in unit `u`: round values in
/// that unit, given back in millimetres (the model's unit). The property
/// bar, the callout bar and the Outline Pen dialog all offer this one list.
pub fn outline_width_presets(u: Units) -> Vec<f64> {
    let list: &[f64] = match u {
        Units::Points => &OUTLINE_WIDTHS_PT,
        Units::Inches => &OUTLINE_WIDTHS_IN,
        Units::Pixels => &OUTLINE_WIDTHS_PX,
        _ => return OUTLINE_WIDTHS_MM.to_vec(),
    };
    list.iter().map(|v| u.to_mm(*v)).collect()
}

/// Decimals an outline width shows in unit `u`.
fn outline_decimals(u: Units) -> usize {
    match u {
        Units::Pixels => 1,
        Units::Points => 2,
        Units::Inches => 4,
        _ => 3,
    }
}

/// An outline width (mm) as a number in unit `u`, without the unit
/// ("0.75"): what the Outline Pen dialog's list shows beside its unit box.
pub fn outline_width_number(w: f64, u: Units) -> String {
    crate::ui::field::format_number(u.from_mm(w), false, 0, Some(outline_decimals(u)))
}

/// An outline width (mm) in unit `u`, as the lists show it ("0.75 pt").
pub fn outline_width_label(w: f64, u: Units) -> String {
    format!("{} {}", outline_width_number(w, u), u.short())
}

/// Text for an outline width in the outline unit ("Hairline", "None").
pub fn outline_text(app: &App, width: Option<f64>) -> String {
    match width {
        None => tr("toolbar.none"),
        Some(w) if w <= tracedraw_core::Stroke::HAIRLINE + 1e-9 => tr("toolbar.hairline"),
        Some(w) => outline_width_label(w, app.outline_units()),
    }
}

/// The outline unit picker (points, millimetres, ...), next to a width.
pub(crate) fn outline_unit_combo(ui: &mut Ui, salt: &str, current: Units) -> Option<Units> {
    let mut picked = None;
    egui::ComboBox::from_id_salt(salt)
        .selected_text(current.short())
        .width(44.0)
        .show_ui(ui, |ui| {
            for u in OUTLINE_UNITS {
                if ui.selectable_label(u == current, u.short()).clicked() {
                    picked = Some(u);
                }
            }
        })
        .response
        .rich_tip(tr("toolbar.outline_units"));
    picked
}

/// What an outline width box's text asks for: no outline (`Some(None)`),
/// a width in millimetres, or nothing readable. Words are "none" and
/// "hairline" (in English or the interface language); numbers are in
/// `units` unless a unit is typed after them.
pub(crate) fn parse_outline_width(text: &str, units: Units) -> Option<Option<f64>> {
    let t = text.trim();
    let is = |key: &str, en: &str| t.eq_ignore_ascii_case(en) || t.eq_ignore_ascii_case(&tr(key));
    if t.is_empty() || is("toolbar.none", "none") {
        return Some(None);
    }
    if is("toolbar.hairline", "hairline") {
        return Some(Some(tracedraw_core::Stroke::HAIRLINE));
    }
    let v = crate::ui::field::parse(t, Some(units.mm()))?;
    let mm = units.to_mm(v);
    (mm.is_finite() && mm >= 0.0).then(|| {
        if mm <= tracedraw_core::Stroke::HAIRLINE {
            Some(tracedraw_core::Stroke::HAIRLINE)
        } else {
            Some(mm.min(1000.0))
        }
    })
}

/// The width the wheel steps to from `width` in the outline width list
/// (no outline, hairline, then the widths of unit `u`): the next one up
/// (wheel up) or down. `None` at the end of the list (#663).
pub(crate) fn outline_width_step(width: Option<f64>, up: bool, u: Units) -> Option<Option<f64>> {
    let hair = tracedraw_core::Stroke::HAIRLINE;
    let list: Vec<Option<f64>> = [None, Some(hair)]
        .into_iter()
        .chain(outline_width_presets(u).into_iter().map(Some))
        .collect();
    // Where `width` falls among the list's widths (no outline below all).
    let key = |w: Option<f64>| w.map_or(-1.0, |w| w.max(hair));
    let at = key(width);
    let eps = 1e-9;
    if up {
        list.into_iter().find(|w| key(*w) > at + eps)
    } else {
        list.into_iter().rev().find(|w| key(*w) < at - eps)
    }
}

/// Take the outline off the selection, or off new objects when nothing is
/// selected.
fn set_no_outline(app: &mut App) {
    if app.selection.is_empty() {
        app.request_default_change(crate::defaults::DefaultChange::Outline(None));
    } else {
        let shapes = app.selection.clone();
        app.run(Command::SetStroke {
            shapes,
            stroke: None,
        });
    }
}

/// The outline width: a box typed into (Enter or leaving it applies, Esc
/// puts it back) and an arrow dropping the usual widths; with `wheel`,
/// the mouse wheel over it steps through those widths. Returns the width
/// picked, typed or stepped to.
fn outline_width_box(
    app: &mut App,
    ui: &mut Ui,
    width: Option<f64>,
    mixed: bool,
    wheel: bool,
) -> Option<Option<f64>> {
    let ctx = ui.ctx().clone();
    let popup_id = ui.make_persistent_id("outline_width_list");
    let id = ui.make_persistent_id("outline_width_box");
    // Objects with different widths: the box is empty until one is typed.
    let shown = if mixed {
        String::new()
    } else {
        outline_text(app, width)
    };
    let mut picked: Option<Option<f64>> = None;
    let inner = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let stored = ui.data(|d| d.get_temp::<String>(id));
        let mut buf = stored.clone().unwrap_or_else(|| shown.clone());
        let resp = ui
            .add(
                egui::TextEdit::singleline(&mut buf)
                    .id(id)
                    .margin(egui::Margin::symmetric(4, 0))
                    .vertical_align(egui::Align::Center)
                    .desired_width(72.0),
            )
            .rich_tip(tr("toolbar.outline_width"));
        if resp.has_focus() {
            ui.data_mut(|d| d.insert_temp(id, buf.clone()));
        } else if stored.is_some() || resp.lost_focus() {
            ui.data_mut(|d| d.remove::<String>(id));
            let escaped = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if !escaped && buf != shown {
                picked = parse_outline_width(&buf, app.outline_units());
            }
        }
        let (arrow_rect, arrow) =
            ui.allocate_exact_size(egui::vec2(16.0, resp.rect.height()), egui::Sense::click());
        let arrow = arrow.rich_tip(crate::ui::tips::pair(
            "toolbar.outline_width",
            "toolbar.outline_widths_desc",
        ));
        if arrow.hovered() {
            ui.painter().rect_filled(
                arrow_rect.shrink(1.0),
                1.0,
                crate::theme::Tokens::tool_hover(),
            );
        }
        let c = arrow_rect.center();
        ui.painter().add(egui::epaint::PathShape::convex_polygon(
            vec![
                c + egui::vec2(-4.0, -2.0),
                c + egui::vec2(4.0, -2.0),
                c + egui::vec2(0.0, 3.0),
            ],
            crate::theme::Tokens::icon(),
            egui::Stroke::NONE,
        ));
        if arrow.clicked() {
            egui::Popup::toggle_id(&ctx, popup_id);
        }
        resp.rect.union(arrow_rect)
    });
    let anchor = inner.inner;
    // The wheel over the closed list: the next width up or down, one step
    // per notch (#663). An event of a notch or more is one step whatever
    // its size (browsers send a notch as 100 points or 3 lines); parts of
    // a notch from a touchpad or a fine wheel add up until they make one,
    // and start again when the wheel turns the other way or the pointer
    // leaves the box.
    let acc_id = id.with("wheel");
    if wheel && !egui::Popup::is_id_open(&ctx, popup_id) && ui.rect_contains_pointer(anchor) {
        if let Some(steps) = crate::ui::field::wheel_steps(ui, acc_id) {
            if steps != 0 {
                // A width being typed goes; the step counts from the one held.
                ui.data_mut(|d| d.remove::<String>(id));
                ui.memory_mut(|m| m.surrender_focus(id));
                let mut w = width;
                for _ in 0..steps.unsigned_abs().min(64) {
                    match outline_width_step(w, steps > 0, app.outline_units()) {
                        Some(next) => w = next,
                        None => break,
                    }
                }
                if w != width {
                    picked = Some(w);
                }
            }
        }
    } else {
        ui.data_mut(|d| d.remove::<f32>(acc_id));
    }
    egui::Popup::new(popup_id, ctx.clone(), anchor, ui.layer_id())
        .open_memory(None)
        .width(anchor.width())
        .show(|ui| {
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    if ui
                        .selectable_label(!mixed && width.is_none(), tr("toolbar.none"))
                        .clicked()
                    {
                        picked = Some(None);
                    }
                    if ui.selectable_label(false, tr("toolbar.hairline")).clicked() {
                        picked = Some(Some(tracedraw_core::Stroke::HAIRLINE));
                    }
                    for w in outline_width_presets(app.outline_units()) {
                        if ui
                            .selectable_label(false, outline_text(app, Some(w)))
                            .clicked()
                        {
                            picked = Some(Some(w));
                        }
                    }
                });
        });
    if picked.is_some() {
        egui::Popup::close_id(&ctx, popup_id);
    }
    picked
}

/// Outline width and the line style and arrowheads of the selection's
/// outline.
pub fn outline_part(app: &mut App, ui: &mut Ui) {
    use tracedraw_core::patch::StrokeField;
    // The first object's outline; values that differ across the selection
    // show empty. Each control changes only its own value on each object.
    let first = app.first_paint_target();
    let stroke = first.as_ref().and_then(|s| s.stroke.clone());
    let mix = app.selection_stroke_mix();
    label_pic(ui, Pic::Pen, &tr("toolbar.outline_width"));
    let width = stroke.as_ref().map(|s| s.width);
    // The wheel changes the selection's outline only, never asks about
    // the outline of new objects at each notch.
    let wheel = !app.selection.is_empty();
    match outline_width_box(
        app,
        ui,
        width,
        mix.presence || mix.has(StrokeField::Width),
        wheel,
    ) {
        Some(None) => set_no_outline(app),
        Some(Some(w)) => app.apply_outline_width(w),
        None => {}
    }
    if let Some(u) = outline_unit_combo(ui, "outline_units_bar", app.outline_units()) {
        app.set_outline_units(u);
    }
    // Line style: solid or one of the dash presets.
    let dash = stroke.as_ref().map(|s| s.dash.clone()).unwrap_or_default();
    let dash_shown = !mix.has(StrokeField::Dash);
    egui::ComboBox::from_id_salt("line_style_bar")
        .selected_text(if dash_shown {
            dash_label(&dash)
        } else {
            String::new()
        })
        .width(96.0)
        .show_ui(ui, |ui| {
            let custom = app.settings.custom_dashes.clone();
            let all: Vec<&[f64]> = DASHES
                .iter()
                .copied()
                .chain(custom.iter().map(|d| d.as_slice()))
                .collect();
            for d in all {
                if ui
                    .selectable_label(dash_shown && dash.as_slice() == d, dash_label(d))
                    .clicked()
                {
                    // Each object (each one in a group too) keeps the
                    // rest of its own outline.
                    let cmds: Vec<Command> = app
                        .paint_targets()
                        .iter()
                        .filter_map(|s| {
                            let mut st = s.stroke.clone()?;
                            st.dash = d.to_vec();
                            Some(Command::SetStroke {
                                shapes: vec![s.id],
                                stroke: Some(st),
                            })
                        })
                        .collect();
                    if let Err(e) = app.engine.run_batch("Line Style", &cmds) {
                        app.status = e.to_string();
                    }
                }
            }
            ui.separator();
            if ui.button(tr("docker.edit_line_style")).clicked() {
                crate::ui::line_style::request(ui);
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed("toolbar.line_style"));
    // Arrowheads at the start and the end.
    for start in [true, false] {
        let current = stroke
            .as_ref()
            .map(|s| {
                if start {
                    s.start_arrow.clone()
                } else {
                    s.end_arrow.clone()
                }
            })
            .unwrap_or_default();
        let shown = !mix.has(if start {
            StrokeField::StartArrow
        } else {
            StrokeField::EndArrow
        });
        egui::ComboBox::from_id_salt(if start {
            "arrow_start_bar"
        } else {
            "arrow_end_bar"
        })
        .selected_text(if shown {
            arrow_label(&current)
        } else {
            String::new()
        })
        .width(56.0)
        .show_ui(ui, |ui| {
            for a in tracedraw_core::Arrowhead::ALL {
                if ui
                    .selectable_label(shown && current == a, arrow_label(&a))
                    .clicked()
                {
                    // Each object (each one in a group too) keeps the
                    // rest of its own outline.
                    let cmds: Vec<Command> = app
                        .paint_targets()
                        .iter()
                        .filter_map(|s| {
                            let mut st = s.stroke.clone()?;
                            if start {
                                st.start_arrow = a.clone();
                            } else {
                                st.end_arrow = a.clone();
                            }
                            Some(Command::SetStroke {
                                shapes: vec![s.id],
                                stroke: Some(st),
                            })
                        })
                        .collect();
                    if let Err(e) = app.engine.run_batch("Arrowheads", &cmds) {
                        app.status = e.to_string();
                    }
                }
            }
        })
        .response
        .rich_tip(crate::ui::tips::keyed(if start {
            "toolbar.start_arrow"
        } else {
            "toolbar.end_arrow"
        }));
    }
}

/// Dash presets (multiples of the outline width); the first is solid.
pub(crate) const DASHES: &[&[f64]] = &[
    &[],
    &[4.0, 2.0],
    &[1.0, 2.0],
    &[8.0, 3.0],
    &[6.0, 2.0, 1.0, 2.0],
    &[6.0, 2.0, 1.0, 2.0, 1.0, 2.0],
];

pub(crate) fn dash_label(d: &[f64]) -> String {
    if d.is_empty() {
        return "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".into();
    }
    // A few characters drawing the pattern.
    let mut s = String::new();
    for (i, v) in d.iter().cycle().take(d.len() * 3).enumerate() {
        let n = (*v / 2.0).round().clamp(1.0, 4.0) as usize;
        let c = if i % 2 == 0 { '\u{2500}' } else { ' ' };
        for _ in 0..n {
            s.push(c);
        }
        if s.chars().count() > 12 {
            break;
        }
    }
    s
}

fn arrow_label(a: &tracedraw_core::Arrowhead) -> String {
    use tracedraw_core::Arrowhead as A;
    match a {
        A::None => "\u{2500}".into(),
        A::Arrow => "\u{2500}\u{25B6}".into(),
        A::OpenArrow => "\u{2500}>".into(),
        A::Circle => "\u{2500}\u{25CF}".into(),
        A::Square => "\u{2500}\u{25A0}".into(),
        A::Bar => "\u{2500}|".into(),
        A::Diamond => "\u{2500}\u{25C6}".into(),
        A::Custom { name, .. } => name.clone(),
    }
}

// ----- shape parts ------------------------------------------------------

/// Ellipse, pie or arc, the start and end angles and the direction, for
/// new ellipses and the selected ones.
pub fn ellipse_part(app: &mut App, ui: &mut Ui) {
    // The selection's arc when an ellipse is selected, else the tool's.
    let selected: Vec<(tracedraw_core::ShapeId, tracedraw_core::geometry::Rect)> = app
        .selected_rc()
        .iter()
        .filter_map(|s| match &s.kind {
            ShapeKind::Ellipse { rect, .. } => Some((s.id, *rect)),
            _ => None,
        })
        .collect();
    let current = app
        .selected_rc()
        .iter()
        .find_map(|s| match &s.kind {
            ShapeKind::Ellipse { arc, .. } => Some(*arc),
            _ => None,
        })
        .unwrap_or(app.ellipse_arc);
    let mode = match current {
        None => 0,
        Some(a) if a.pie => 1,
        Some(_) => 2,
    };
    let mut arc = current.unwrap_or(tracedraw_core::EllipseArc {
        start_deg: 90.0,
        end_deg: 90.0,
        pie: true,
    });
    let mut new = current;
    for (i, pic, key) in [
        (0, Pic::Ellipse, "toolbar.ellipse_tip"),
        (1, Pic::Pie, "toolbar.pie_tip"),
        (2, Pic::Arc, "toolbar.arc_tip"),
    ] {
        if pic_button(ui, pic, &tr(key), true, mode == i).clicked() && mode != i {
            new = match i {
                0 => None,
                k => {
                    arc.pie = k == 1;
                    Some(arc)
                }
            };
        }
    }
    let angles_on = mode != 0;
    stacked(ui, |ui| {
        for start in [true, false] {
            let key = if start {
                "toolbar.arc_start"
            } else {
                "toolbar.arc_end"
            };
            crate::ui::tips::tip_over(ui, crate::ui::tips::keyed(key), |ui| {
                row(ui, |ui| {
                    let mut v = if start { arc.start_deg } else { arc.end_deg };
                    let r = ui.add_enabled(
                        angles_on,
                        crate::ui::field::NumField::new(&mut v)
                            .speed(1.0)
                            .fixed_decimals(1)
                            .suffix(" \u{00B0}"),
                    );
                    if angles_on && r.changed() {
                        if start {
                            arc.start_deg = v;
                        } else {
                            arc.end_deg = v;
                        }
                        new = Some(arc);
                    }
                })
            });
        }
    });
    if pic_button(
        ui,
        Pic::ArcDirection,
        &tr("toolbar.arc_direction"),
        angles_on,
        false,
    )
    .clicked()
    {
        std::mem::swap(&mut arc.start_deg, &mut arc.end_deg);
        new = Some(arc);
    }
    if new != current {
        app.ellipse_arc = new;
        let cmds: Vec<Command> = selected
            .iter()
            .map(|(id, rect)| Command::SetShapeKind {
                shape: *id,
                kind: ShapeKind::Ellipse {
                    rect: *rect,
                    arc: new,
                },
            })
            .collect();
        if !cmds.is_empty() {
            if let Err(e) = app.engine.run_batch("Ellipse", &cmds) {
                app.status = e.to_string();
            }
        }
    }
}

/// Points and sharpness of polygons and stars (sharpness 1 to 99);
/// complex stars count their sharpness in
/// steps (1 up to what their points allow). With the Star tool, the Star
/// and Complex Star buttons pick what it draws.
pub fn polygon_part(app: &mut App, ui: &mut Ui, star: bool) {
    use tracedraw_core::geometry::{complex_star_max_sharpness, COMPLEX_STAR_MIN_POINTS};
    type Sel = (
        tracedraw_core::ShapeId,
        tracedraw_core::geometry::Rect,
        u32,
        f64,
        Option<u32>,
        f64,
    );
    let selected: Vec<Sel> = app
        .selected_rc()
        .iter()
        .filter_map(|s| match &s.kind {
            ShapeKind::Polygon {
                rect,
                points,
                sharpness,
                complex,
                twist,
            } => Some((s.id, *rect, *points, *sharpness, *complex, *twist)),
            _ => None,
        })
        .collect();
    if star && app.tool == crate::tools::Tool::Star {
        for (complex, pic, tip) in [
            (false, Pic::Star, "toolbar.star_tool"),
            (true, Pic::ComplexStar, "toolbar.complex_star_tool"),
        ] {
            if pic_button(ui, pic, &tr(tip), true, app.star_complex == complex).clicked() {
                app.star_complex = complex;
            }
        }
        sep(ui);
    }
    let complex = match selected.first() {
        Some(s) => s.4.is_some(),
        None => star && app.star_complex,
    };
    if complex {
        let (mut n, mut k) = selected
            .first()
            .map(|s| (s.2, s.4.unwrap_or(1)))
            .unwrap_or((app.complex_points, app.complex_sharpness));
        let (n0, k0) = (n, k);
        crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("toolbar.points_tip"), |ui| {
            ui.label(tr("toolbar.points_sides"));
            ui.add_sized(
                [56.0, ROW + 4.0],
                crate::ui::field::NumField::new(&mut n).range(COMPLEX_STAR_MIN_POINTS..=500),
            );
        });
        let max = complex_star_max_sharpness(n);
        crate::ui::tips::tip_over(
            ui,
            crate::ui::tips::keyed("toolbar.complex_sharpness_tip"),
            |ui| {
                ui.label(tr("toolbar.sharpness"));
                ui.add_sized(
                    [56.0, ROW + 4.0],
                    crate::ui::field::NumField::new(&mut k).range(1..=max),
                );
            },
        );
        let k = k.clamp(1, complex_star_max_sharpness(n));
        if n != n0 || k != k0 {
            app.complex_points = n;
            app.complex_sharpness = k;
            let cmds: Vec<Command> = selected
                .iter()
                .filter(|s| s.4.is_some())
                .map(|(id, rect, ..)| Command::SetShapeKind {
                    shape: *id,
                    kind: ShapeKind::Polygon {
                        rect: *rect,
                        points: n,
                        sharpness: 0.0,
                        complex: Some(k),
                        twist: 0.0,
                    },
                })
                .collect();
            if !cmds.is_empty() {
                if let Err(e) = app.engine.run_batch("Complex Star", &cmds) {
                    app.status = e.to_string();
                }
            }
        }
        return;
    }
    let (mut n, mut sharp) = selected
        .first()
        .map(|s| (s.2, s.3))
        .unwrap_or((app.polygon_points, app.star_sharpness));
    let star = star || selected.first().is_some_and(|s| s.3 > 0.0);
    let (n0, s0) = (n, sharp);
    crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("toolbar.points_tip"), |ui| {
        ui.label(tr("toolbar.points_sides"));
        ui.add_sized(
            [56.0, ROW + 4.0],
            crate::ui::field::NumField::new(&mut n).range(3..=500),
        );
    });
    if star {
        crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("toolbar.sharpness_tip"), |ui| {
            ui.label(tr("toolbar.sharpness"));
            let mut pct = (sharp * 100.0).round();
            if ui
                .add_sized(
                    [56.0, ROW + 4.0],
                    crate::ui::field::NumField::new(&mut pct).range(1.0..=99.0),
                )
                .changed()
            {
                sharp = (pct / 100.0).clamp(0.01, 0.99);
            }
        });
    }
    if n != n0 || (sharp - s0).abs() > 1e-9 {
        app.polygon_points = n;
        app.star_sharpness = sharp;
        let cmds: Vec<Command> = selected
            .iter()
            .filter(|s| s.4.is_none())
            .map(|(id, rect, _, old, _, twist)| Command::SetShapeKind {
                shape: *id,
                kind: ShapeKind::Polygon {
                    rect: *rect,
                    points: n,
                    sharpness: if *old > 0.0 || star { sharp } else { 0.0 },
                    complex: None,
                    // A twisted star keeps its turn.
                    twist: *twist,
                },
            })
            .collect();
        if !cmds.is_empty() {
            if let Err(e) = app.engine.run_batch("Polygon", &cmds) {
                app.status = e.to_string();
            }
        }
    }
}

/// The horizontal and vertical character offsets (percent of the font
/// size) and the character angle (degrees) of the characters being edited:
/// the Text tool's selection, the Shape tool's chosen character nodes, or
/// every character of the selected texts. Each change is one undo step.
pub fn char_offset_part(app: &mut App, ui: &mut Ui) {
    let Some((mut h, mut v, mut a)) = app.char_attrs() else {
        return;
    };
    let (h0, v0, a0) = (h, v, a);
    let field = |ui: &mut Ui,
                 tip: &str,
                 label: &str,
                 val: &mut f64,
                 range: std::ops::RangeInclusive<f64>,
                 suffix: &str| {
        crate::ui::tips::tip_over(ui, crate::ui::tips::keyed(tip), |ui| {
            ui.label(label);
            ui.add_sized(
                [72.0, ROW + 4.0],
                crate::ui::field::NumField::new(val)
                    .range(range)
                    .max_decimals(1)
                    .suffix(suffix),
            );
        });
    };
    field(
        ui,
        "toolbar.char_h_offset",
        "x:",
        &mut h,
        -500.0..=500.0,
        " %",
    );
    field(
        ui,
        "toolbar.char_v_offset",
        "y:",
        &mut v,
        -500.0..=500.0,
        " %",
    );
    field(
        ui,
        "toolbar.char_angle",
        "\u{2220}",
        &mut a,
        -360.0..=360.0,
        " \u{b0}",
    );
    if (h - h0).abs() > 1e-9 {
        app.set_char_attrs("Character Offset", move |sp| sp.offset_x_pct = h);
    }
    if (v - v0).abs() > 1e-9 {
        app.set_char_attrs("Character Offset", move |sp| {
            sp.baseline_shift_pt = v / 100.0 * sp.size_pt
        });
    }
    if (a - a0).abs() > 1e-9 {
        app.set_char_attrs("Character Angle", move |sp| sp.angle_deg = a);
    }
}

/// A change the rectangle part of the bar makes to corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerEdit {
    Kind(CornerKind),
    /// One corner's size (page mm); with the lock every corner gets it.
    Size(usize, f64),
    /// Relative corner scaling on or off.
    Scaling(bool),
}

impl CornerEdit {
    /// `c` (measured on the page) after the edit.
    pub fn apply(self, mut c: Corners, together: bool) -> Corners {
        match self {
            CornerEdit::Kind(k) => c.kind = k,
            CornerEdit::Size(i, v) => {
                let v = if v.is_finite() { v.max(0.0) } else { 0.0 };
                if together {
                    c.radii = [v; 4];
                } else if let Some(r) = c.radii.get_mut(i) {
                    *r = v;
                }
            }
            CornerEdit::Scaling(relative) => c.fixed = !relative,
        }
        c
    }
}

/// Apply a corner edit to the selected rectangles, or to the defaults for
/// new ones when none is selected.
pub fn edit_corners(app: &mut App, edit: CornerEdit) {
    let together = app.corners_together;
    let cmds: Vec<Command> = app
        .selected_rc()
        .iter()
        .filter_map(|s| {
            let kind = s.with_page_corners(edit.apply(s.page_corners()?, together))?;
            (kind != s.kind).then_some(Command::SetShapeKind { shape: s.id, kind })
        })
        .collect();
    let any_rect = app
        .selected_rc()
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Rect { .. }));
    if !any_rect {
        app.rect_corners = edit.apply(app.rect_corners, together);
        return;
    }
    if cmds.is_empty() {
        return;
    }
    let label = match edit {
        CornerEdit::Kind(_) => "Corner Style",
        CornerEdit::Size(..) => "Corner Radius",
        CornerEdit::Scaling(_) => "Corner Scaling",
    };
    if let Err(e) = app.engine.run_batch(label, &cmds) {
        app.status = e.to_string();
    }
}

/// Rectangle corners: the corner style, the four sizes (top left over
/// bottom left, then top right over bottom right) with the lock that edits
/// them together, and relative corner scaling.
pub fn rectangle_part(app: &mut App, ui: &mut Ui) {
    let current = app
        .selected_rc()
        .iter()
        .find_map(|s| s.page_corners())
        .unwrap_or(app.rect_corners);
    // Values that differ across the selected rectangles show as mixed.
    let mix = app.selection_corner_mix().map_or([false; 6], |(_, m)| m);
    let mut edit = None;
    for (kind, pic, tip) in [
        (CornerKind::Round, Pic::CornerRound, "toolbar.corner_round"),
        (
            CornerKind::Scallop,
            Pic::CornerScallop,
            "toolbar.corner_scallop",
        ),
        (
            CornerKind::Chamfer,
            Pic::CornerChamfer,
            "toolbar.corner_chamfer",
        ),
    ] {
        if pic_button(ui, pic, &tr(tip), true, !mix[0] && current.kind == kind).clicked() {
            edit = Some(CornerEdit::Kind(kind));
        }
    }
    sep(ui);
    for column in [
        [Corners::TOP_LEFT, Corners::BOTTOM_LEFT],
        [Corners::TOP_RIGHT, Corners::BOTTOM_RIGHT],
    ] {
        stacked(ui, |ui| {
            for i in column {
                let key = match i {
                    Corners::TOP_LEFT => "toolbar.corner_top_left",
                    Corners::TOP_RIGHT => "toolbar.corner_top_right",
                    Corners::BOTTOM_LEFT => "toolbar.corner_bottom_left",
                    _ => "toolbar.corner_bottom_right",
                };
                crate::ui::tips::tip_over(ui, crate::ui::tips::keyed(key), |ui| {
                    row(ui, |ui| {
                        let mixed = mix.get(i + 1).copied().unwrap_or(false);
                        if let Some(v) = distance_mixed(ui, app, current.radii[i], 66.0, mixed) {
                            edit = Some(CornerEdit::Size(i, v));
                        }
                    })
                });
            }
        });
    }
    let together = app.corners_together;
    if pic_button(
        ui,
        Pic::Lock(together),
        &tr("toolbar.corners_together"),
        true,
        together,
    )
    .clicked()
    {
        app.corners_together = !together;
    }
    sep(ui);
    let relative = !current.fixed;
    if pic_button(
        ui,
        Pic::CornerScaling,
        &tr("toolbar.relative_corners"),
        true,
        relative && !mix[5],
    )
    .clicked()
    {
        edit = Some(CornerEdit::Scaling(!relative));
    }
    if let Some(e) = edit {
        edit_corners(app, e);
    }
}

/// The part that follows the object bar for the selected kind of object
/// (rectangle corners, ellipse arcs, polygon points); true when one was
/// drawn.
pub fn kind_part(app: &mut App, ui: &mut Ui) -> bool {
    let kinds: Vec<u8> = app
        .selected_rc()
        .iter()
        .map(|s| match s.kind {
            ShapeKind::Rect { .. } => 1,
            ShapeKind::Ellipse { .. } => 2,
            ShapeKind::Polygon { .. } => 3,
            _ => 0,
        })
        .collect();
    let Some(first) = kinds.first().copied() else {
        return false;
    };
    if first == 0 || kinds.iter().any(|k| *k != first) {
        return false;
    }
    match first {
        1 => rectangle_part(app, ui),
        2 => ellipse_part(app, ui),
        _ => polygon_part(app, ui, false),
    }
    true
}

// ----- the Zoom tool's bar ---------------------------------------------

pub fn zoom_bar(app: &mut App, ui: &mut Ui) {
    crate::ui::toolbar::zoom_box_pub(app, ui, "bar_zoom", 107.0);
    for (pic, key, f) in [
        (Pic::ZoomIn, "menu.view.zoom_in", 0),
        (Pic::ZoomOut, "menu.view.zoom_out", 1),
    ] {
        if pic_button(ui, pic, &tr(key), true, false).clicked() {
            app.zoom_step(f == 0);
        }
    }
    sep(ui);
    let has = !app.selection.is_empty();
    if pic_button(
        ui,
        Pic::ZoomSelected,
        &tr("toolbar.zoom_to_selected"),
        has,
        false,
    )
    .clicked()
    {
        app.zoom_to_selection();
    }
    if pic_button(ui, Pic::ZoomAll, &tr("toolbar.zoom_to_fit"), true, false).clicked() {
        app.zoom_to_fit();
    }
    sep(ui);
    if pic_button(ui, Pic::ZoomPage, &tr("toolbar.zoom_to_page"), true, false).clicked() {
        app.zoom_to_page();
    }
    if pic_button(
        ui,
        Pic::ZoomWidth,
        &tr("toolbar.zoom_to_width"),
        true,
        false,
    )
    .clicked()
    {
        app.zoom_to_page_width();
    }
    if pic_button(
        ui,
        Pic::ZoomHeight,
        &tr("toolbar.zoom_to_height"),
        true,
        false,
    )
    .clicked()
    {
        app.zoom_to_page_height();
    }
}

/// Order and conversion buttons after an object's properties.
/// Grouping, combining and shaping the selection (Pick tool, two or more
/// objects, a group, or a curve of several parts), after a separator:
/// shown only when one of them applies. Returns whether it was shown.
/// What the arrange part offers for the selected `shapes`: several
/// objects (group, combine, shape), a group (ungroup), a curve of several
/// parts (break apart); `None` when nothing applies.
pub fn arrange_offers(shapes: &[tracedraw_core::Shape]) -> Option<(bool, bool, bool)> {
    let many = shapes.len() >= 2;
    let groups = shapes
        .iter()
        .any(|s| matches!(s.kind, ShapeKind::Group { .. }));
    let parts = shapes.iter().any(|s| match &s.kind {
        ShapeKind::Path { path, .. } => {
            path.elements()
                .iter()
                .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
                .count()
                > 1
        }
        _ => false,
    });
    (many || groups || parts).then_some((many, groups, parts))
}

pub fn arrange_part(app: &mut App, ui: &mut Ui) -> bool {
    use crate::ops::Shaping;
    let shapes = app.selected_rc();
    let Some((many, groups, parts)) = arrange_offers(&shapes) else {
        return false;
    };
    sep(ui);
    let tip = |key: &str, keys: &str| {
        if keys.is_empty() {
            tr(key)
        } else {
            format!("{} ({keys})", tr(key))
        }
    };
    if pic_button(
        ui,
        Pic::Group,
        &tip("menu.object.group_group", "Ctrl+G"),
        many,
        false,
    )
    .clicked()
    {
        app.group_selection();
    }
    if pic_button(
        ui,
        Pic::Ungroup,
        &tip("menu.object.group_ungroup", "Ctrl+U"),
        groups,
        false,
    )
    .clicked()
    {
        app.ungroup_selection();
    }
    if pic_button(
        ui,
        Pic::UngroupAll,
        &tip("menu.object.group_ungroup_all", ""),
        groups,
        false,
    )
    .clicked()
    {
        app.ungroup_all();
    }
    sep(ui);
    if pic_button(
        ui,
        Pic::Combine,
        &tip("menu.object.combine", "Ctrl+L"),
        many,
        false,
    )
    .clicked()
    {
        app.combine();
    }
    if pic_button(
        ui,
        Pic::BreakApart,
        &tip("menu.object.break_apart", "Ctrl+K"),
        parts,
        false,
    )
    .clicked()
    {
        app.break_apart();
    }
    sep(ui);
    for (pic, key, hint, op, need_many) in [
        (
            Pic::Weld,
            "menu.object.weld",
            "docker.weld_hint",
            Shaping::Weld,
            true,
        ),
        (
            Pic::Trim,
            "menu.object.trim",
            "docker.trim_hint",
            Shaping::Trim,
            true,
        ),
        (
            Pic::Intersect,
            "menu.object.intersect",
            "docker.intersect_hint",
            Shaping::Intersect,
            true,
        ),
        (
            Pic::Simplify,
            "menu.object.simplify",
            "docker.simplify_hint",
            Shaping::Simplify,
            true,
        ),
        (
            Pic::FrontMinusBack,
            "menu.object.front_minus_back",
            "docker.front_minus_back_hint",
            Shaping::FrontMinusBack,
            true,
        ),
        (
            Pic::BackMinusFront,
            "menu.object.back_minus_front",
            "docker.back_minus_front_hint",
            Shaping::BackMinusFront,
            true,
        ),
        (
            Pic::Boundary,
            "menu.object.boundary",
            "docker.boundary_hint",
            Shaping::Boundary,
            false,
        ),
    ] {
        let on = if need_many {
            op.ready(shapes.len())
        } else {
            !shapes.is_empty()
        };
        let tip = format!("{}\n{}", tr(key), tr(hint));
        if pic_button(ui, pic, &tip, on, false).clicked() {
            app.shaping(op);
        }
    }
    sep(ui);
    if pic_button(
        ui,
        Pic::AlignDistribute,
        &tip("menu.object.align_distribute_docker", "Ctrl+Shift+A"),
        true,
        false,
    )
    .clicked()
    {
        app.show_dockers = true;
        app.docker_tab = crate::app::DockerTab::AlignDistribute;
    }
    true
}

/// The key of a wrap style's name.
pub fn wrap_style_key(
    mode: tracedraw_core::WrapMode,
    side: tracedraw_core::WrapSide,
) -> &'static str {
    use tracedraw_core::{WrapMode as M, WrapSide as S};
    match (mode, side) {
        (M::Contour, S::Left) => "context.wrap_contour_left",
        (M::Contour, S::Right) => "context.wrap_contour_right",
        (M::Contour, S::Straddle) => "context.wrap_contour_straddle",
        (_, S::Left) => "context.wrap_square_left",
        (_, S::Right) => "context.wrap_square_right",
        (_, S::Straddle) => "context.wrap_square_straddle",
        (_, S::AboveBelow) => "context.wrap_square_above_below",
    }
}

/// The arrow beside Wrap Paragraph Text: the wrap styles and the text wrap
/// offset of the selected objects; picking one turns wrapping on.
fn wrap_style_list(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let popup_id = ui.make_persistent_id("wrap_style_list");
    let (on, current) = app.wrap_state();
    let button = ui
        .add_enabled(
            !app.selection.is_empty(),
            egui::Button::new("\u{25BE}").small(),
        )
        .rich_tip(tr("context.wrap_style"));
    if button.clicked() {
        egui::Popup::toggle_id(&ctx, popup_id);
    }
    egui::Popup::new(popup_id, ctx.clone(), button.rect, ui.layer_id())
        .open_memory(None)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .width(260.0)
        .show(|ui| {
            // As the reference program lists them (#661): a title, None,
            // the contour and the square styles under their headings, and
            // the offset, greyed out while nothing wraps (typing one
            // would turn wrapping back on).
            ui.label(egui::RichText::new(tr("context.wrap_style")).strong());
            if ui.selectable_label(!on, tr("context.wrap_none")).clicked() && on {
                let shapes = app.selection.clone();
                app.run(tracedraw_core::Command::SetWrapText {
                    shapes,
                    wrap: false,
                });
            }
            use tracedraw_core::WrapMode;
            for (heading, contour) in [
                ("context.wrap_contour", true),
                ("context.wrap_square", false),
            ] {
                ui.separator();
                ui.label(egui::RichText::new(tr(heading)).strong());
                for (mode, side) in tracedraw_core::TextWrap::STYLES {
                    if (mode == WrapMode::Contour) != contour {
                        continue;
                    }
                    let here = on && current.mode == mode && current.side == side;
                    if ui.selectable_label(here, tr(wrap_side_key(side))).clicked() {
                        app.set_wrap_style(mode, side);
                    }
                }
            }
            ui.separator();
            ui.add_enabled_ui(on, |ui| wrap_offset_field(app, ui, current.offset));
        });
}

/// A wrap style's side under its Contour or Square heading.
fn wrap_side_key(side: tracedraw_core::WrapSide) -> &'static str {
    use tracedraw_core::WrapSide as S;
    match side {
        S::Left => "context.wrap_flows_left",
        S::Right => "context.wrap_flows_right",
        S::Straddle => "context.wrap_straddle",
        S::AboveBelow => "context.wrap_above_below",
    }
}

/// The text wrap offset field (property bar list and context menu).
pub fn wrap_offset_field(app: &mut App, ui: &mut Ui, offset: f64) {
    ui.horizontal(|ui| {
        ui.label(tr("context.wrap_offset"));
        let mut v = offset;
        if crate::ui::text_docker::length(ui, app, &mut v) {
            app.set_wrap_offset(v);
        }
    });
}

/// Wrap Paragraph Text, To Front and To Back of the layer and Convert to
/// Curves, for the selected objects.
pub fn order_part(app: &mut App, ui: &mut Ui) {
    let wrap = app.selected_rc().iter().any(|s| s.wrap_text);
    if pic_button(
        ui,
        Pic::WrapText,
        &tr("context.wrap_paragraph_text"),
        !app.selection.is_empty(),
        wrap,
    )
    .clicked()
    {
        let ids = app.selection.clone();
        app.run(tracedraw_core::Command::SetWrapText {
            shapes: ids,
            wrap: !wrap,
        });
    }
    wrap_style_list(app, ui);
    if pic_button(ui, Pic::ToFront, &tr("toolbar.to_front"), true, false).clicked() {
        app.order(0);
    }
    if pic_button(ui, Pic::ToBack, &tr("toolbar.to_back"), true, false).clicked() {
        app.order(3);
    }
    let curves_ok = app.can_convert_to_curves();
    if pic_button(
        ui,
        Pic::ToCurves,
        &format!("{} (Ctrl+Q)", tr("menu.object.convert_to_curves")),
        curves_ok,
        false,
    )
    .clicked()
    {
        app.convert_to_curves();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect as R;

    /// The rotation field's arrows turn the selection a degree at each
    /// click, also when the box was clicked into first (its value
    /// selected): only the first click used to turn it, the one that took
    /// the focus from the box. The wheel over the box turns it too.
    #[test]
    fn every_click_on_the_rotation_arrows_turns_the_selection() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: R::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        let origin = egui::pos2(100.0, 100.0);
        let run = |app: &mut App, events: Vec<egui::Event>| -> egui::Rect {
            let mut rect = egui::Rect::NOTHING;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 400.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::Area::new(egui::Id::new("rotation"))
                        .fixed_pos(origin)
                        .show(ui.ctx(), |ui| {
                            rect = rotation_field(app, ui, Point::new(35.0, 25.0)).rect;
                        });
                },
            );
            out.textures_delta.clear();
            rect
        };
        run(&mut app, Vec::new());
        let field = run(&mut app, Vec::new());
        let click = |app: &mut App, at: egui::Pos2| {
            run(app, vec![egui::Event::PointerMoved(at)]);
            for pressed in [true, false] {
                run(
                    app,
                    vec![egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            run(app, Vec::new());
        };
        let up = egui::pos2(
            field.right() + crate::ui::field::SPIN_W / 2.0,
            field.top() + field.height() * 0.25,
        );
        // Into the box (its value selected), then the up arrow twice.
        click(&mut app, field.center());
        click(&mut app, up);
        click(&mut app, up);
        assert!(
            (selection_angle(&app) - 2.0).abs() < 1e-6,
            "{}",
            selection_angle(&app)
        );
        // Once more, the box no longer holding the focus.
        click(&mut app, up);
        assert!(
            (selection_angle(&app) - 3.0).abs() < 1e-6,
            "{}",
            selection_angle(&app)
        );
        // The wheel over the box: down a degree.
        run(&mut app, vec![egui::Event::PointerMoved(field.center())]);
        run(
            &mut app,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -1.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(
            (selection_angle(&app) - 2.0).abs() < 1e-6,
            "{}",
            selection_angle(&app)
        );
        // One undo step a click.
        app.undo();
        assert!((selection_angle(&app) - 3.0).abs() < 1e-6);
    }

    #[test]
    fn shaping_and_grouping_show_for_several_objects_groups_and_parts() {
        use tracedraw_core::geometry::Shape as _;
        use tracedraw_core::{Shape, ShapeId};
        let rect = |id: u64, x: f64| {
            Shape::new(
                ShapeId(id),
                ShapeKind::Rect {
                    rect: R::new(x, 0.0, x + 10.0, 10.0),
                    radius: 0.0,
                    corners: None,
                },
            )
        };
        // One plain object: nothing to arrange.
        assert_eq!(arrange_offers(&[rect(1, 0.0)]), None);
        assert_eq!(arrange_offers(&[]), None);
        // Two objects: group, combine and the shaping buttons.
        assert_eq!(
            arrange_offers(&[rect(1, 0.0), rect(2, 20.0)]),
            Some((true, false, false))
        );
        // A group alone: ungroup.
        let group = Shape::new(
            ShapeId(3),
            ShapeKind::Group {
                children: vec![rect(4, 0.0), rect(5, 20.0)],
            },
        );
        assert_eq!(arrange_offers(&[group]), Some((false, true, false)));
        // A curve of two parts: break apart.
        let mut path = R::new(0.0, 0.0, 5.0, 5.0).to_path(0.1);
        path.extend(R::new(10.0, 0.0, 15.0, 5.0).to_path(0.1));
        let curve = Shape::new(ShapeId(6), ShapeKind::Path { path, closed: true });
        assert_eq!(arrange_offers(&[curve]), Some((false, false, true)));
    }

    #[test]
    fn the_object_origin_picks_corners_edges_and_centre() {
        let b = R::new(10.0, 20.0, 30.0, 60.0);
        assert_eq!(origin_point(b, (1, 1)), Point::new(20.0, 40.0));
        assert_eq!(origin_point(b, (0, 0)), Point::new(10.0, 60.0));
        assert_eq!(origin_point(b, (2, 2)), Point::new(30.0, 20.0));
        assert_eq!(origin_point(b, (1, 2)), Point::new(20.0, 20.0));
    }

    /// The page bar draws with its Fit page to drawing button on a page
    /// with objects and on an empty one (where the button is pale), and
    /// the icon is on the 16-grid like the others (#236).
    #[test]
    fn the_page_bar_draws_with_its_fit_page_button() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let run = |app: &mut App| {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.horizontal(|ui| page_bar(app, ui));
            });
            out.textures_delta.clear();
            out.shapes.len()
        };
        let empty = run(&mut app);
        app.new_shape(ShapeKind::Rect {
            rect: R::new(10.0, 10.0, 60.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer");
        app.select(Vec::new());
        let full = run(&mut app);
        assert!(empty > 0 && full > 0);
        // The icon itself.
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
            draw_pic(ui.painter(), rect, Pic::FitPage, Tokens::text());
        });
        out.textures_delta.clear();
    }

    /// The page bar's list of sizes ends with Custom..., ticked when the
    /// page matches no preset; clicking it opens the Page Size dialog with
    /// the page's own width and height (#650).
    #[test]
    fn the_page_size_list_offers_a_custom_size() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let size = Size::new(123.0, 45.0);
        let custom = tr("toolbar.custom_page");
        let run = |app: &mut App, events: Vec<egui::Event>| -> Option<egui::Rect> {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| page_size_list(app, ui, "Custom", size),
            );
            out.textures_delta.clear();
            out.shapes.iter().find_map(|c| match &c.shape {
                epaint::Shape::Text(t) if t.galley.text() == custom => {
                    Some(t.galley.rect.translate(t.pos.to_vec2()))
                }
                _ => None,
            })
        };
        let at = run(&mut app, Vec::new()).expect("a Custom entry").center();
        run(&mut app, vec![egui::Event::PointerMoved(at)]);
        for pressed in [true, false] {
            run(
                &mut app,
                vec![egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        match app.dialog {
            crate::ui::dialogs::Dialog::PageSize { width, height, .. } => {
                assert!((width - 123.0).abs() < 1e-9 && (height - 45.0).abs() < 1e-9);
            }
            _ => panic!("the Page Size dialog"),
        }
    }

    /// With Live preview on the open popup shows the page Autofit Page
    /// would make; off, the page stays as it is (#659).
    #[test]
    fn the_autofit_popup_previews_only_with_live_preview_on() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: R::new(10.0, 10.0, 60.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer");
        let run = |app: &mut App| {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.horizontal(|ui| page_bar(app, ui));
            });
            out.textures_delta.clear();
        };
        run(&mut app);
        assert!(app.autofit_preview.is_none(), "closed");
        egui::Popup::open_id(&ctx, autofit_popup_id());
        assert!(app.settings.fit_page_preview, "on at first");
        run(&mut app);
        assert!(app.autofit_preview.is_some());
        assert!(app
            .autofit_preview_rect(ctx.cumulative_frame_nr())
            .is_some());
        app.settings.fit_page_preview = false;
        run(&mut app);
        assert!(egui::Popup::is_id_open(&ctx, autofit_popup_id()));
        assert!(app.autofit_preview.is_none());
        assert!(app
            .autofit_preview_rect(ctx.cumulative_frame_nr())
            .is_none());
    }

    #[test]
    fn page_size_changes_one_page_or_all_in_one_step() {
        let mut app = App::headless();
        app.add_page();
        let depth = app.engine.history_labels().0.len();
        app.page_size_all = true;
        resize_pages(&mut app, Size::new(100.0, 50.0));
        assert!(app
            .doc()
            .pages
            .iter()
            .all(|p| p.size == Size::new(100.0, 50.0)));
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        app.page_size_all = false;
        resize_pages(&mut app, Size::new(70.0, 70.0));
        let sizes: Vec<Size> = app.doc().pages.iter().map(|p| p.size).collect();
        assert_eq!(
            sizes
                .iter()
                .filter(|s| **s == Size::new(70.0, 70.0))
                .count(),
            1
        );
    }

    /// The wheel steps through the outline width list: no outline,
    /// hairline, then the widths of the unit, from wherever the width is
    /// (#663).
    #[test]
    fn the_wheel_steps_through_the_outline_widths() {
        let hair = tracedraw_core::Stroke::HAIRLINE;
        for u in OUTLINE_UNITS {
            let p = outline_width_presets(u);
            assert_eq!(outline_width_step(None, true, u), Some(Some(hair)));
            assert_eq!(outline_width_step(None, false, u), None);
            assert_eq!(outline_width_step(Some(hair), false, u), Some(None));
            // The first width above the hairline (in inches the first is
            // the hairline itself).
            let above = p.iter().copied().find(|w| *w > hair + 1e-9);
            assert_eq!(outline_width_step(Some(hair), true, u), above.map(Some));
            assert_eq!(outline_width_step(Some(p[2]), true, u), Some(Some(p[3])));
            assert_eq!(outline_width_step(Some(p[2]), false, u), Some(Some(p[1])));
            if p[0] > hair {
                assert_eq!(outline_width_step(Some(p[0]), false, u), Some(Some(hair)));
            }
            // Between two widths: the one above or below.
            let mid = (p[1] + p[2]) / 2.0;
            assert_eq!(outline_width_step(Some(mid), true, u), Some(Some(p[2])));
            assert_eq!(outline_width_step(Some(mid), false, u), Some(Some(p[1])));
            // At the widest it stays.
            let last = *p.last().expect("widths");
            assert_eq!(outline_width_step(Some(last), true, u), None);
            assert_eq!(
                outline_width_step(Some(last * 3.0), false, u),
                Some(Some(last))
            );
        }
    }

    /// The outline width box in a property bar longer than the window:
    /// notch after notch the width steps and the bar stays where it is,
    /// on the frames between the notches too, where the smoothed scroll
    /// of a notch slid the bar and took the box away from under the
    /// pointer (#783).
    #[test]
    fn the_wheel_over_the_outline_width_in_a_long_bar_keeps_the_bar_still() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: R::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        let p = outline_width_presets(app.outline_units());
        app.apply_outline_width(p[6]);
        let width = |app: &App| {
            app.doc()
                .find_shape(id)
                .and_then(|s| s.stroke.as_ref().map(|s| s.width))
        };
        let clock = std::cell::Cell::new(0.0f64);
        let run = |app: &mut App, events: Vec<egui::Event>| -> egui::Rect {
            let mut box_id = egui::Id::NULL;
            clock.set(clock.get() + 1.0 / 60.0);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 300.0),
                    )),
                    events,
                    time: Some(clock.get()),
                    ..Default::default()
                },
                |ui| {
                    crate::ui::field::hold_taken_wheel(ui.ctx());
                    crate::ui::overflow::row(ui, "property_test", |ui| {
                        ui.add_space(200.0);
                        box_id = ui.make_persistent_id("outline_width_box");
                        outline_part(app, ui);
                        ui.add_space(900.0);
                    });
                },
            );
            out.textures_delta.clear();
            ctx.read_response(box_id)
                .map(|r| r.rect)
                .unwrap_or(egui::Rect::NOTHING)
        };
        for _ in 0..3 {
            run(&mut app, Vec::new());
        }
        let start = run(&mut app, Vec::new());
        assert!(start.is_positive(), "{start:?}");
        run(&mut app, vec![egui::Event::PointerMoved(start.center())]);
        for k in 1..=5 {
            run(
                &mut app,
                vec![egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    // Down: the bar slides forward with it.
                    delta: egui::vec2(0.0, -1.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            for f in 0..5 {
                let now = run(&mut app, Vec::new());
                assert!(
                    (now.left() - start.left()).abs() < 0.5,
                    "notch {k}, frame {f}: {now:?} from {start:?}"
                );
            }
            assert_eq!(width(&app), Some(p[6 - k]), "notch {k}");
        }
    }

    /// The wheel over the property bar's outline width box changes the
    /// selection's outline to the next width up or down, one undo step a
    /// notch; with nothing selected it does nothing (#663).
    #[test]
    fn the_wheel_over_the_outline_width_changes_it() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: R::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        let p = outline_width_presets(app.outline_units());
        app.apply_outline_width(p[2]);
        let width = |app: &App| {
            app.doc()
                .find_shape(id)
                .and_then(|s| s.stroke.as_ref().map(|s| s.width))
        };
        assert_eq!(width(&app), Some(p[2]));
        let origin = egui::pos2(100.0, 100.0);
        let run = |app: &mut App, events: Vec<egui::Event>| -> egui::Rect {
            let mut box_id = egui::Id::NULL;
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 400.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::Area::new(egui::Id::new("outline_bar"))
                        .fixed_pos(origin)
                        .show(ui.ctx(), |ui| {
                            ui.horizontal(|ui| {
                                box_id = ui.make_persistent_id("outline_width_box");
                                outline_part(app, ui);
                            });
                        });
                },
            );
            out.textures_delta.clear();
            ctx.read_response(box_id)
                .map(|r| r.rect)
                .unwrap_or(egui::Rect::NOTHING)
        };
        // The bar settles over the first frames.
        run(&mut app, Vec::new());
        run(&mut app, Vec::new());
        let field = run(&mut app, Vec::new());
        assert!(field.is_positive() && field.top() >= origin.y, "{field:?}");
        let wheel = |app: &mut App, dy: f32| {
            run(app, vec![egui::Event::PointerMoved(field.center())]);
            run(
                app,
                vec![egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, dy),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        };
        let depth = app.engine.history_labels().0.len();
        wheel(&mut app, 1.0);
        assert_eq!(width(&app), Some(p[3]));
        wheel(&mut app, -1.0);
        wheel(&mut app, -1.0);
        assert_eq!(width(&app), Some(p[1]));
        assert_eq!(app.engine.history_labels().0.len(), depth + 3);
        // Parts of a notch (a touchpad, a fine wheel) add up: a quarter
        // does nothing, four of them make one step.
        for _ in 0..3 {
            wheel(&mut app, 0.25);
            assert_eq!(width(&app), Some(p[1]));
        }
        wheel(&mut app, 0.25);
        assert_eq!(width(&app), Some(p[2]));
        // A notch of three lines (Firefox) or 100 points (Chrome) is one
        // step.
        wheel(&mut app, 3.0);
        assert_eq!(width(&app), Some(p[3]));
        run(&mut app, vec![egui::Event::PointerMoved(field.center())]);
        run(
            &mut app,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 100.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(width(&app), Some(p[4]));
        assert_eq!(app.engine.history_labels().0.len(), depth + 6);
        // Turning back drops the part of a notch gathered so far.
        wheel(&mut app, 0.75);
        wheel(&mut app, -0.5);
        wheel(&mut app, 0.5);
        assert_eq!(width(&app), Some(p[4]));
        // Nothing selected: nothing changes, nothing is asked.
        app.select(Vec::new());
        wheel(&mut app, 1.0);
        assert_eq!(width(&app), Some(p[4]));
        assert_eq!(app.engine.history_labels().0.len(), depth + 6);
        assert!(matches!(app.dialog, crate::ui::dialogs::Dialog::None));
    }

    #[test]
    fn outline_widths_can_be_typed() {
        let mm = Units::Millimeters;
        assert_eq!(parse_outline_width("0.35", mm), Some(Some(0.35)));
        assert_eq!(parse_outline_width("0,8 mm", mm), Some(Some(0.8)));
        assert_eq!(parse_outline_width("1 in", mm), Some(Some(25.4)));
        assert_eq!(parse_outline_width("2", Units::Inches), Some(Some(50.8)));
        assert_eq!(parse_outline_width("None", mm), Some(None));
        assert_eq!(
            parse_outline_width("hairline", mm),
            Some(Some(tracedraw_core::Stroke::HAIRLINE))
        );
        assert_eq!(parse_outline_width("thick", mm), None);
        assert_eq!(parse_outline_width("-1", mm), None);
        // Points: the number is points, stored in millimetres.
        let pt = parse_outline_width("2", Units::Points)
            .flatten()
            .expect("width");
        assert!((pt - 2.0 * 25.4 / 72.0).abs() < 1e-12);
    }

    #[test]
    fn the_width_presets_follow_the_outline_unit() {
        let pts = outline_width_presets(Units::Points);
        // Round point values, given in millimetres.
        assert!(pts.iter().any(|w| (w - 25.4 / 72.0).abs() < 1e-12));
        for w in &pts {
            let v = Units::Points.from_mm(*w);
            assert!((v * 4.0 - (v * 4.0).round()).abs() < 1e-9, "{v}");
        }
        assert_eq!(outline_width_label(pts[2], Units::Points), "0.75 pt");
        assert_eq!(
            outline_width_presets(Units::Millimeters),
            OUTLINE_WIDTHS_MM.to_vec()
        );
        let inches = outline_width_presets(Units::Inches);
        assert!((inches[2] - 0.254).abs() < 1e-12);
        assert_ne!(pts, outline_width_presets(Units::Pixels));
        for u in OUTLINE_UNITS {
            assert!(outline_width_presets(u)
                .iter()
                .all(|w| *w > 0.0 && w.is_finite()));
        }
    }

    #[test]
    fn outline_widths_read_in_the_outline_unit() {
        let mut app = App::headless();
        app.units = Units::Millimeters;
        // Points by default, whatever the ruler unit.
        assert_eq!(app.outline_units(), Units::Points);
        assert_eq!(outline_text(&app, Some(25.4 / 72.0)), "1 pt");
        assert_eq!(outline_text(&app, Some(0.2)), "0.57 pt");
        app.set_outline_units(Units::Millimeters);
        assert_eq!(app.settings.outline_units, "mm");
        assert_eq!(outline_text(&app, Some(0.2)), "0.2 mm");
        app.set_outline_units(Units::Inches);
        assert_eq!(outline_text(&app, Some(2.54)), "0.1 \"");
        assert_eq!(outline_text(&app, None), tr("toolbar.none"));
        assert_eq!(
            outline_text(&app, Some(tracedraw_core::Stroke::HAIRLINE)),
            tr("toolbar.hairline")
        );
    }

    #[test]
    fn corner_edits_change_the_selection_in_one_step_or_the_defaults() {
        let mut app = App::headless();
        // Nothing selected: the defaults for new rectangles.
        edit_corners(&mut app, CornerEdit::Size(Corners::TOP_LEFT, 3.0));
        assert_eq!(app.rect_corners.radii, [3.0; 4]);
        edit_corners(&mut app, CornerEdit::Kind(CornerKind::Chamfer));
        assert_eq!(app.rect_corners.kind, CornerKind::Chamfer);
        app.rect_corners = Corners::default();
        let id = app
            .new_shape(ShapeKind::rect_with_corners(
                R::new(0.0, 0.0, 40.0, 20.0),
                Corners::default(),
            ))
            .expect("a rectangle");
        app.select(vec![id]);
        app.transform_selection(Affine::scale(2.0));
        let depth = app.engine.history_labels().0.len();
        // Together: every corner; the sizes are page millimetres.
        edit_corners(&mut app, CornerEdit::Size(Corners::TOP_RIGHT, 6.0));
        let s = app.selected_rc()[0].clone();
        assert_eq!(s.page_corners().unwrap().radii, [6.0; 4]);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        assert_eq!(app.engine.undo_label(), Some("Corner Radius"));
        // Unlocked: one corner.
        app.corners_together = false;
        edit_corners(&mut app, CornerEdit::Size(Corners::BOTTOM_LEFT, 1.0));
        let s = app.selected_rc()[0].clone();
        assert_eq!(s.page_corners().unwrap().radii, [6.0, 6.0, 1.0, 6.0]);
        // Turning relative scaling off keeps the sizes on the page.
        edit_corners(&mut app, CornerEdit::Scaling(false));
        let s = app.selected_rc()[0].clone();
        let c = s.page_corners().unwrap();
        assert!(c.fixed);
        assert_eq!(c.radii, [6.0, 6.0, 1.0, 6.0]);
        // ... and scaling the rectangle again leaves them alone.
        app.transform_selection(Affine::scale(0.5));
        let s = app.selected_rc()[0].clone();
        assert_eq!(s.page_corners().unwrap().radii, [6.0, 6.0, 1.0, 6.0]);
        assert_eq!(app.rect_corners, Corners::default());
        // Undo goes back one edit at a time.
        app.undo();
        app.undo();
        let s = app.selected_rc()[0].clone();
        assert!(!s.page_corners().unwrap().fixed);
    }

    #[test]
    fn the_star_tool_draws_complex_stars_that_fill_the_drag() {
        let mut app = App::headless();
        app.tool = crate::tools::Tool::Star;
        app.star_complex = true;
        app.create_box_shape(Point::new(10.0, 10.0), Point::new(50.0, 30.0));
        let s = app.selected_rc()[0].clone();
        assert!(matches!(
            s.kind,
            ShapeKind::Polygon {
                points: 9,
                complex: Some(2),
                ..
            }
        ));
        let b = s.bounds();
        assert!(
            (b.x0 - 10.0).abs() < 1e-9 && (b.x1 - 50.0).abs() < 1e-9,
            "{b:?}"
        );
        assert!(
            (b.y0 - 10.0).abs() < 1e-9 && (b.y1 - 30.0).abs() < 1e-9,
            "{b:?}"
        );
        // The preview while dragging is the same star.
        let p = app.creation_preview(Point::new(10.0, 10.0), Point::new(50.0, 30.0), false);
        assert_eq!(
            p.map(|p| p.elements().len()),
            Some(s.local_path().elements().len())
        );
        // Perfect stars stay perfect.
        app.star_complex = false;
        app.create_box_shape(Point::new(10.0, 10.0), Point::new(50.0, 30.0));
        let s = app.selected_rc()[0].clone();
        assert!(matches!(s.kind, ShapeKind::Polygon { complex: None, .. }));
    }

    #[test]
    fn the_rectangle_tool_draws_with_the_default_corners() {
        let mut app = App::headless();
        app.rect_corners = Corners::uniform(2.0, CornerKind::Scallop);
        app.tool = crate::tools::Tool::Rectangle;
        app.create_box_shape(Point::new(10.0, 10.0), Point::new(50.0, 30.0));
        let s = app.selected_rc()[0].clone();
        let (_, c) = s.kind.rect_corners().unwrap();
        assert_eq!(c, Corners::uniform(2.0, CornerKind::Scallop));
    }
}
