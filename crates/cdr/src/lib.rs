//! traco-cdr: a clean-room reader for the editor `.cdr` files.
//!
//! What we know about the format comes from public reverse-engineering notes
//! and from observing real files, never from the vendor's code.
//!
//! Two containers exist:
//! - **RIFF** (the editor 7 through X3, and the inner stream of newer files):
//!   `RIFF <size> CDR<v>` followed by nested `LIST` chunks.
//! - **ZIP** (X4 and later): a ZIP archive whose `content/riffData.cdr`
//!   holds the RIFF stream; large payloads live in `content/data/*.dat`
//!   and are referenced from the stream.
//!
//! The reader has three layers:
//! 1. [`container`]: detect the container, hand back the RIFF bytes.
//! 2. [`riff`]: walk the chunk tree without interpreting it.
//! 3. [`parse`]: turn chunks into a [`traco_core::Document`] (best effort).
//!
//! Anything we do not understand is skipped with a logged warning, never a
//! failure: opening a file must always yield a document.

pub mod container;
pub mod parse;
pub mod riff;

use std::path::Path;

pub use container::{Container, Version};
pub use parse::{parse_document, ParseReport};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a the editor file")]
    NotCdr,
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

/// Read a `.cdr` file into a document. The report lists what was skipped.
pub fn open(path: impl AsRef<Path>) -> Result<(traco_core::Document, ParseReport)> {
    let bytes = std::fs::read(path.as_ref())?;
    let title = path.as_ref().file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
    open_bytes(&bytes, &title)
}

/// Same as [`open`], from memory.
pub fn open_bytes(bytes: &[u8], title: &str) -> Result<(traco_core::Document, ParseReport)> {
    let container = container::detect(bytes)?;
    let riff = container::riff_stream(bytes, &container)?;
    let root = riff::parse(&riff)?;
    let mut doc_report = parse::parse_document(&root, &riff, container.version);
    doc_report.0.title = title.to_string();
    Ok(doc_report)
}
