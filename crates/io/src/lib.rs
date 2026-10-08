//! traco-io: export the document model to interchange formats and read the
//! native `.traco` file.

pub mod svg;

use std::path::Path;
use traco_core::Document;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Core(#[from] traco_core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Native format: pretty JSON with a `.traco` extension.
pub fn save_native(doc: &Document, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, doc.to_json()?)?;
    Ok(())
}

pub fn load_native(path: impl AsRef<Path>) -> Result<Document> {
    let s = std::fs::read_to_string(path)?;
    Ok(Document::from_json(&s)?)
}

/// Write the first page (or `page_index`) as an SVG file.
pub fn save_svg(doc: &Document, page_index: usize, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, svg::page_to_svg(doc, page_index))?;
    Ok(())
}
