//! tracedraw-cdr-format: the structure of `.cdr` files, independent of any
//! document model.
//!
//! What is known about the format comes from public format notes and from
//! inspecting files we own, never from any vendor's code or from another
//! implementation (see `CLEANROOM.md` at the root of the repository).
//!
//! The crate holds the layers below the document:
//! - [`container`]: detect a bare RIFF file or a ZIP package, hand back the
//!   RIFF stream and the external data streams.
//! - [`riff`]: walk the chunk tree without interpreting it, inflate the
//!   compressed lists, follow the redirect records of version 16 and later.
//! - [`loda`]: the argument table of the record describing an object.
//! - [`raw`]: the records a reader leaves uninterpreted, kept as bytes
//!   shared with the tree, by the object they belong to.
//! - [`eot`]: the Embedded OpenType structure around an embedded font.
//! - [`ole`]: compound files holding embedded objects and their pictures.
//!
//! Turning chunks into a drawing is the job of the crate above this one
//! (`tracedraw-cdr`). Nothing here depends on the rest of the workspace.
#![forbid(unsafe_code)]

pub mod container;
pub mod eot;
pub mod loda;
pub mod ole;
pub mod raw;
pub mod riff;

pub use container::{Container, ContainerKind, Version};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a CDR file")]
    NotCdr,
    #[error("not a CMX file")]
    NotCmx,
    #[error("unsupported container: {0}")]
    UnsupportedContainer(String),
    #[error("truncated data at offset {0}")]
    Truncated(usize),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

pub type Result<T> = std::result::Result<T, Error>;
