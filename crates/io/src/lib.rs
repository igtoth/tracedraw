//! tracedraw-io: export the document model to interchange formats and read the
//! native `.tdraw` file.

pub mod pdf;
pub mod svg;

use std::path::Path;
use tracedraw_core::Document;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Core(#[from] tracedraw_core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Native format: pretty JSON with a `.tdraw` extension.
pub fn save_native(doc: &Document, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, doc.to_json()?)?;
    Ok(())
}

pub fn load_native(path: impl AsRef<Path>) -> Result<Document> {
    let s = std::fs::read_to_string(path)?;
    Ok(Document::from_json(&s)?)
}

/// Write every page to a PDF file.
pub fn save_pdf(doc: &Document, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, pdf::document_to_pdf(doc))?;
    Ok(())
}

/// Write the first page (or `page_index`) as an SVG file.
pub fn save_svg(doc: &Document, page_index: usize, path: impl AsRef<Path>) -> Result<()> {
    std::fs::write(path, svg::page_to_svg(doc, page_index))?;
    Ok(())
}
