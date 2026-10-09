//! tracedraw-core: the pure-data heart of TraceDraw.
//!
//! Everything in this crate is independent of any UI or file format.
//! The UI, the CLI and the importers all talk to the engine through
//! [`Command`]s applied to a [`Document`] via an [`Engine`].
//!
//! Units: all geometry is stored in millimetres, with the origin at the
//! bottom-left corner of the page and the Y axis pointing up, matching the
//! the target design's convention. Renderers flip to screen space as needed.

pub mod color;
pub mod command;
pub mod document;
pub mod effects;
pub mod engine;
pub mod geometry;
pub mod icc;
pub mod id;
pub mod live;
pub mod nodes;
pub mod shaping;
pub mod style;

pub use color::Color;
pub use command::Command;
pub use document::{
    ColorStyle, Document, EllipseArc, Layer, MasterScope, Metadata, ObjectStyle, Page,
    ParagraphStyle, Shadow, Shape, ShapeKind, Symbol, Table, TableCell, TextAlign, TextOnPath,
    TextSpan,
};
pub use engine::Engine;
pub use geometry::{Affine, BezPath, Point, Rect, Size, Vec2};
pub use icc::{Intent as RenderingIntent, Profile as IccProfile, Transform as IccTransform};
pub use id::{LayerId, PageId, ShapeId};
pub use style::{
    arrowhead_paths, Arrowhead, Fill, Fountain, FountainKind, LineCap, LineJoin, Mesh, MeshNode,
    Pattern, PatternTile, Stop, Stroke, Texture, TextureKind,
};

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
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
