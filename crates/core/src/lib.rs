//! tracedraw-core: the pure-data heart of TraceDraw.
//!
//! Everything in this crate is independent of any UI or file format.
//! The UI, the CLI and the importers all talk to the engine through
//! [`Command`]s applied to a [`Document`] via an [`Engine`].
//!
//! Units: all geometry is stored in millimetres, with the origin at the
//! bottom-left corner of the page and the Y axis pointing up. Renderers flip to screen space as needed.

#![allow(clippy::should_implement_trait)]
pub mod bytes;
pub mod color;
pub mod color_convert;
pub mod command;
pub mod corner_cut;
pub mod distort;
pub mod document;
pub mod effects;
pub mod engine;
pub mod envelope;
pub mod extrude;
pub mod extrude3d;
pub mod find;
pub mod fit;
pub mod geometry;
pub mod icc;
pub mod id;
pub mod join;
pub mod live;
pub mod nodes;
pub mod painterly;
pub mod paragraphs;
pub mod patch;
mod press;
pub mod shadow;
pub mod shaping;
pub mod style;
pub mod table;
pub mod text_flow;

pub use bytes::SharedBytes;
pub use color::Color;
pub use command::Command;
pub use document::{
    merge_equal_spans, spans_apply, spans_char_count, spans_delete, spans_insert, spans_text,
    split_spans_at, Baseline, BitmapEffect, BitmapFxStack, ColorStyle, CornerKind, Corners,
    DefaultKind, DefaultProperties, Document, DocumentFont, DocumentProfile, EllipseArc,
    FontVariation, Layer, LeadingUnit, MasterScope, Metadata, ObjectDefaults, ObjectStyle, Page,
    ParagraphStyle, PathTextOrientation, PathTextVertical, QrCode, QrKind, Shadow, Shape,
    ShapeKind, StyleParts, Symbol, Table, TableCell, TextAlign, TextOnPath, TextSpan, TimeTracker,
    TrackedTask, VerticalAlign,
};
pub use engine::Engine;
pub use geometry::{Affine, BezPath, Point, Rect, Size, Vec2};
pub use icc::{Intent as RenderingIntent, Profile as IccProfile, Transform as IccTransform};
pub use id::{LayerId, PageId, ShapeId};
pub use paragraphs::ParagraphFormat;
pub use style::{
    arrowhead_paths, Arrowhead, Fill, Fountain, FountainKind, FountainRepeat, LineCap, LineJoin,
    Mesh, MeshNode, OutlinePosition, Pattern, PatternTile, Stop, Stroke, Texture, TextureKind,
    WidthNode,
};
pub use table::{BorderSet, CellBorders, CellMargins, CellSide, EdgeBorder, TableHit};
pub use text_flow::{TextWrap, WrapMode, WrapSide};

/// Convenience result type for the core crate.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("page not found: {0:?}")]
    PageNotFound(PageId),
    #[error("layer not found: {0:?}")]
    LayerNotFound(LayerId),
    #[error("shape not found: {0:?}")]
    ShapeNotFound(ShapeId),
    #[error("cannot delete the last page")]
    LastPage,
    #[error("cannot delete the last layer of a page")]
    LastLayer,
    #[error("nothing to undo")]
    NothingToUndo,
    #[error("nothing to redo")]
    NothingToRedo,
    #[error("object style not found: {0}")]
    StyleNotFound(u32),
    #[error("selection group {0} is not 0 to 9")]
    SelectionGroup(u8),
    #[error("object {0:?} is not a clip frame")]
    NotClipFrame(ShapeId),
    #[error("a B-spline needs two finite control points or more")]
    BadSpline,
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
