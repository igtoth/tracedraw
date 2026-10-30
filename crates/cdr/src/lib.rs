//! tracedraw-cdr: a clean-room reader for `.cdr` files.
//!
//! What we know about the format comes from public reverse-engineering notes
//! and from observing real files, never from any vendor's code.
//!
//! Two containers exist:
//! - **RIFF** (CDR versions 7 through version 13, and the inner stream of newer files):
//!   `RIFF <size> CDR<v>` followed by nested `LIST` chunks.
//! - **ZIP** (version 14 and later): a ZIP archive whose `content/riffData.cdr`
//!   holds the RIFF stream; large payloads live in `content/data/*.dat`
//!   and are referenced from the stream.
//!
//! The reader has three layers:
//! 1. [`container`]: detect the container, hand back the RIFF bytes.
//! 2. [`riff`]: walk the chunk tree without interpreting it.
//! 3. [`parse`]: turn chunks into a [`tracedraw_core::Document`] (best effort).
//!
//! The first two, with the embedded font and compound file readers, live
//! in the `tracedraw-cdr-format` crate, which knows no document model;
//! they are re-exported here under their old paths.
//!
//! Anything we do not understand is skipped with a logged warning, never a
//! failure: opening a file must always yield a document.
#![forbid(unsafe_code)]

pub mod check;
pub mod cmx;
pub mod container;
mod kept_blends;
mod kept_effects;
pub mod parse;
mod shadow_image;
pub mod spot;
mod text;
pub mod wl;
pub mod write;

use std::path::Path;
use std::sync::Arc;

pub use container::{Container, Version};
pub use parse::{parse_document, parse_document_with, ParseReport};
pub use tracedraw_cdr_format::raw::{self, RawTable};
pub use tracedraw_cdr_format::{eot, loda, ole, riff, Error, Result};

/// Files list a radial or square gradient's colours from its edge to its
/// centre (a sun drawn red in the middle stores yellow first); ours run
/// from the centre out. The same mirror turns either order into the
/// other.
pub(crate) fn mirror_centred_stops(f: &mut tracedraw_core::Fountain) {
    use tracedraw_core::FountainKind;
    if matches!(f.kind, FountainKind::Radial | FountainKind::Square) {
        for s in &mut f.stops {
            s.pos = 1.0 - s.pos;
        }
        f.stops.reverse();
    }
}

/// Read a `.cdr` file into a document. The report lists what was skipped.
pub fn open(path: impl AsRef<Path>) -> Result<(tracedraw_core::Document, ParseReport)> {
    let bytes = std::fs::read(path.as_ref())?;
    let title = path
        .as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();
    open_bytes(&bytes, &title)
}

/// Same as [`open`], from memory. CMX exchange files (RIFF form `CMX1`)
/// are read as well, by [`cmx::open_bytes`].
pub fn open_bytes(bytes: &[u8], title: &str) -> Result<(tracedraw_core::Document, ParseReport)> {
    open_bytes_keeping(bytes, title).map(|(doc, report, _)| (doc, report))
}

/// [`open_bytes`] that also hands back the records the reader left
/// uninterpreted (the chunks and `loda` arguments it does not read),
/// keyed by object, as bytes shared with the file's streams. Empty for
/// CMX and `WL` files. The writer does not use it yet; it is what a
/// later save will put back untouched.
pub fn open_bytes_keeping(
    bytes: &[u8],
    title: &str,
) -> Result<(tracedraw_core::Document, ParseReport, RawTable)> {
    if cmx::is_cmx(bytes) {
        return cmx::open_bytes(bytes, title).map(|(d, r)| (d, r, RawTable::default()));
    }
    if wl::is_wl(bytes) {
        return wl::open_bytes(bytes, title).map(|(d, r)| (d, r, RawTable::default()));
    }
    let container = container::detect(bytes)?;
    let riff = Arc::new(container::riff_stream(bytes, &container)?);
    let externals = container::external_streams(bytes, &container);
    let root = riff::parse_with_externals(&riff, externals, container.version.0)?;
    let cmyk_profile = container::cmyk_profile(bytes, &container);
    // Blends TraceDraw saved as groups open live again (#986).
    let kept = container::own_member(
        bytes,
        &container,
        kept_blends::MEMBER,
        kept_blends::MAX_MEMBER,
    )
    .map(|b| kept_blends::decode(&b))
    .unwrap_or_default();
    // Objects TraceDraw saved as what their effects and shadows draw open
    // live again too (#1031).
    let kept_effects = container::own_member(
        bytes,
        &container,
        kept_effects::MEMBER,
        kept_effects::MAX_MEMBER,
    )
    .map(|b| kept_effects::decode(&b))
    .unwrap_or_default();
    let mut doc_report = parse::parse_document_kept(
        &root,
        &riff,
        container.version,
        cmyk_profile.as_ref().map(|p| p.data.as_slice()),
        &kept,
        &kept_effects,
    );
    doc_report.0.title = title.to_string();
    doc_report.0.palette = container::document_palette(bytes, &container)
        .into_iter()
        .map(|(_, c)| c)
        .collect();
    // Spot library colours take what the file's page preview shows; a
    // version 3 file's thumbnail (`imhd`, a `.bmp` of the drawing) serves
    // the same way.
    if !doc_report.0.inks.is_empty() {
        let thumbnail = || {
            let mut found = None;
            root.root.walk(&mut |c, _| {
                if c.is(b"imhd") && found.is_none() {
                    found = Some(c.clone());
                }
            });
            let c = found?;
            let mut p = spot::Preview::decode(root.data(&riff, &c))?;
            p.covers_drawing = true;
            Some(p)
        };
        let preview = if container.version.0 < 4 {
            thumbnail()
        } else {
            container::page_preview(bytes, &container)
        };
        if let Some(preview) = preview {
            let n = spot::colours_from_preview(&mut doc_report.0, &preview);
            if n > 0 {
                doc_report.1.warn(format!(
                    "{n} spot colour(s) take their look from the file's preview"
                ));
            }
        }
    }
    let (fonts, notes) = container::embedded_fonts(bytes, &container);
    doc_report.0.fonts = fonts;
    doc_report.0.cmyk_profile = cmyk_profile;
    for n in notes {
        doc_report.1.warn(n);
    }
    // What the reader left, by object: the records a later save can put
    // back. The version the report settled on reads the argument tables.
    let version = doc_report.1.version.unwrap_or(container.version).0;
    let kept = raw::collect(&root, riff, version, &parse::known());
    doc_report.1.unknown = kept.summary();
    Ok((doc_report.0, doc_report.1, kept))
}
