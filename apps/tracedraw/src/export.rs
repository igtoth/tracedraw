//! Export (vector and raster formats), print (through a PDF handed to
//! the system viewer), print merge, find and replace.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::ui::dialogs::{ExportState, PrintState, EXPORT_FORMATS};
use std::path::Path;
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, Rect, Size},
    Command, Document, ShapeId, TextSpan,
};

/// Document limited to the selection or the current page, for exports.
pub(crate) fn export_document(app: &App, st: &ExportState) -> (Document, Rect) {
    let doc = app.engine.document();
    if st.selection_only {
        let bounds = app.selection_bounds().unwrap_or(app.page_rect());
        let mut tmp = Document::new(
            doc.title.clone(),
            Size::new(bounds.width().max(0.1), bounds.height().max(0.1)),
        );
        tmp.symbols = doc.symbols.clone();
        let l = tmp.pages[0].layers[0].id;
        for s in app.selected_shapes() {
            let mut c = s.clone();
            c.transform_by(Affine::translate((-bounds.x0, -bounds.y0)));
            if let Ok(layer) = tmp.layer_mut(l) {
                layer.shapes.push(c);
            }
        }
        (tmp, bounds)
    } else if st.all_pages {
        (doc.clone(), app.page_rect())
    } else {
        let mut tmp = doc.clone();
        tmp.pages.retain(|p| p.id == app.page);
        (tmp, app.page_rect())
    }
}

pub fn export(app: &mut App, path: &Path, st: &ExportState) -> Result<String, String> {
    let (_, ext) = EXPORT_FORMATS[st.format];
    let (doc, _) = export_document(app, st);
    match ext {
        "svg" => {
            let svg = tracedraw_io::svg::page_to_svg(&doc, 0);
            crate::files::write(path, svg).map_err(|e| e.to_string())?;
        }
        "pdf" | "ai" => {
            use tracedraw_io::pdf::{PdfOptions, PdfStandard};
            let standard = match (ext, st.pdf_standard) {
                ("pdf", 1) => PdfStandard::X1a,
                ("pdf", 2) => PdfStandard::X3,
                ("pdf", 3) => PdfStandard::X4,
                _ => PdfStandard::None,
            };
            let opts = PdfOptions {
                standard,
                output_profile: pdfx_profile(app, standard),
                output_condition: st.output_condition.clone(),
                bleed_mm: if ext == "pdf" { st.bleed_mm } else { 0.0 },
                title: Some(if st.title.trim().is_empty() {
                    app.document_title().trim_end_matches('*').to_string()
                } else {
                    st.title.trim().to_string()
                }),
                keywords: (!st.tags.trim().is_empty()).then(|| st.tags.trim().to_string()),
                flatten_dpi: st.dpi.max(72.0),
                ..Default::default()
            };
            let pdf = tracedraw_io::pdf::document_to_pdf_with(&doc, &opts);
            crate::files::write(path, pdf).map_err(|e| e.to_string())?;
        }
        "eps" => {
            let eps = tracedraw_io::eps::page_to_eps(&doc, 0);
            crate::files::write(path, eps).map_err(|e| e.to_string())?;
        }
        "ps" | "prn" => {
            // Every exported page; a print file is the same PostScript.
            let ps = tracedraw_io::eps::document_to_ps(&doc);
            crate::files::write(path, ps).map_err(|e| e.to_string())?;
        }
        "dxf" => {
            let dxf = tracedraw_io::dxf::page_to_dxf(&doc, 0);
            crate::files::write(path, dxf).map_err(|e| e.to_string())?;
        }
        "emf" => {
            let emf = tracedraw_io::emf::page_to_emf(&doc, 0);
            crate::files::write(path, emf).map_err(|e| e.to_string())?;
        }
        "wmf" => {
            let wmf = tracedraw_io::emf::page_to_wmf(&doc, 0);
            crate::files::write(path, wmf).map_err(|e| e.to_string())?;
        }
        "plt" => {
            let plt = tracedraw_io::plt::page_to_plt(&doc, 0);
            crate::files::write(path, plt).map_err(|e| e.to_string())?;
        }
        "psd" => {
            let psd =
                tracedraw_io::psd::page_to_psd(&doc, 0, st.render_dpi()).ok_or("render failed")?;
            crate::files::write(path, psd).map_err(|e| e.to_string())?;
        }
        "html" => {
            let html = tracedraw_io::html::document_to_html(&doc);
            crate::files::write(path, html).map_err(|e| e.to_string())?;
        }
        _ => {
            // Raster: every page when all_pages, numbered files.
            let pages: Vec<_> = doc.pages.iter().map(|p| p.id).collect();
            for (i, pid) in pages.iter().enumerate() {
                let mut pm = tracedraw_render::render_page_image(&doc, *pid, st.render_dpi())
                    .ok_or("render failed")?;
                if st.transparent && matches!(ext, "png" | "webp" | "gif") {
                    // Re-render on a transparent background.
                    let p = doc.page(*pid).map_err(|e| e.to_string())?;
                    let zoom = st.render_dpi() / 25.4;
                    let w = (p.size.width * zoom).ceil() as u32;
                    let h = (p.size.height * zoom).ceil() as u32;
                    pm = tracedraw_render::render_page(
                        &doc,
                        *pid,
                        &tracedraw_render::RenderOptions {
                            width: w,
                            height: h,
                            view: tracedraw_render::ViewTransform {
                                zoom,
                                origin_x: 0.0,
                                origin_y: h as f64,
                            },
                            preview: None,
                            wireframe: false,
                            ..tracedraw_render::RenderOptions::default()
                        },
                    )
                    .ok_or("render failed")?;
                }
                let target = if pages.len() > 1 {
                    let stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    path.with_file_name(format!("{stem}-{}.{ext}", i + 1))
                } else {
                    path.to_path_buf()
                };
                let bytes = encode_raster_with(&pm, ext, st.quality, st.webp_lossless)?;
                // The file says its resolution, so it opens at the size
                // chosen in the dialog (#617).
                let bytes = record_dpi(bytes, ext, st.dpi);
                crate::files::write(&target, bytes).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(trf(
        "status.exported",
        &[("path", &path.display().to_string())],
    ))
}

/// The output intent's profile for a PDF/X file: the CMYK profile picked
/// under Colour Management, when there is one and it can be read.
pub(crate) fn pdfx_profile(app: &App, standard: tracedraw_io::pdf::PdfStandard) -> Option<Vec<u8>> {
    if standard == tracedraw_io::pdf::PdfStandard::None {
        return None;
    }
    let p = app.settings.color.cmyk_profile_path.trim().to_string();
    (!p.is_empty())
        .then(|| crate::files::read(&p).ok())
        .flatten()
}

/// `bytes`, a PNG, JPEG or BMP file, saying it was made at `dpi` dots per
/// inch: a PNG gets a `pHYs` chunk after its header, a JPEG's JFIF header
/// and a BMP's header get the density (#617). Other formats, and files not
/// laid out as the encoder writes them, come back unchanged.
pub fn record_dpi(mut bytes: Vec<u8>, ext: &str, dpi: f64) -> Vec<u8> {
    if !(dpi.is_finite() && dpi >= 1.0) {
        return bytes;
    }
    let per_metre = (dpi / 0.0254).round().min(f64::from(i32::MAX)) as u32;
    match ext {
        "png" => {
            // Signature (8) and the IHDR chunk (4 + 4 + 13 + 4).
            if bytes.len() < 33
                || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                || bytes.get(12..16) != Some(b"IHDR")
            {
                return bytes;
            }
            let mut chunk = Vec::with_capacity(21);
            chunk.extend_from_slice(&9u32.to_be_bytes());
            chunk.extend_from_slice(b"pHYs");
            chunk.extend_from_slice(&per_metre.to_be_bytes());
            chunk.extend_from_slice(&per_metre.to_be_bytes());
            chunk.push(1);
            let crc = crc32(&chunk[4..]);
            chunk.extend_from_slice(&crc.to_be_bytes());
            bytes.splice(33..33, chunk);
            bytes
        }
        "jpg" => {
            // SOI, then the JFIF APP0 segment: units at 13, densities at
            // 14 and 16.
            if bytes.len() < 18
                || bytes.get(2..4) != Some(&[0xFF, 0xE0])
                || bytes.get(6..11) != Some(b"JFIF\0")
            {
                return bytes;
            }
            let d = (dpi.round().min(f64::from(u16::MAX)) as u16).to_be_bytes();
            bytes[13] = 1;
            bytes[14..16].copy_from_slice(&d);
            bytes[16..18].copy_from_slice(&d);
            bytes
        }
        "bmp" => {
            // File header (14), then the info header with the pixels per
            // metre at 24 and 28.
            if bytes.len() < 46 || !bytes.starts_with(b"BM") {
                return bytes;
            }
            let m = per_metre.to_le_bytes();
            bytes[38..42].copy_from_slice(&m);
            bytes[42..46].copy_from_slice(&m);
            bytes
        }
        _ => bytes,
    }
}

/// The CRC-32 a PNG chunk ends with.
fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

/// True when the export format `ext` keeps transparency.
pub fn keeps_alpha(ext: &str) -> bool {
    matches!(ext, "png" | "webp" | "gif")
}

/// What the export will look like: the page (or the selection) of `st`
/// rendered to fit `max_px` on its longer side, on a transparent
/// background when the format keeps one, else on white.
pub fn preview(app: &App, st: &ExportState, max_px: u32) -> Option<tiny_skia::Pixmap> {
    let (_, ext) = EXPORT_FORMATS[st.format.min(EXPORT_FORMATS.len() - 1)];
    let (doc, _) = export_document(app, st);
    let page = doc.pages.first()?;
    let longest = page.size.width.max(page.size.height).max(0.1);
    let zoom = max_px.max(16) as f64 / longest;
    let w = (page.size.width * zoom).ceil().max(1.0) as u32;
    let h = (page.size.height * zoom).ceil().max(1.0) as u32;
    let mut pm = tracedraw_render::render_page(
        &doc,
        page.id,
        &tracedraw_render::RenderOptions {
            width: w,
            height: h,
            view: tracedraw_render::ViewTransform {
                zoom,
                origin_x: 0.0,
                origin_y: h as f64,
            },
            preview: None,
            wireframe: false,
            ..tracedraw_render::RenderOptions::default()
        },
    )?;
    if !(st.transparent && keeps_alpha(ext)) {
        let mut white = tiny_skia::Pixmap::new(w, h)?;
        white.fill(tiny_skia::Color::WHITE);
        white.draw_pixmap(
            0,
            0,
            pm.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
        pm = white;
    }
    Some(pm)
}

/// About how many bytes the export of `st` takes: a vector format is
/// written in memory; a bitmap format is estimated from `preview` (the
/// output of [`preview`]) encoded the same way and scaled to the export's
/// pixel count.
pub fn estimated_size(app: &App, st: &ExportState, preview: &tiny_skia::Pixmap) -> Option<usize> {
    let (_, ext) = EXPORT_FORMATS[st.format.min(EXPORT_FORMATS.len() - 1)];
    let (doc, bounds) = export_document(app, st);
    match ext {
        "svg" => Some(tracedraw_io::svg::page_to_svg(&doc, 0).len()),
        "eps" => Some(tracedraw_io::eps::page_to_eps(&doc, 0).len()),
        "ps" | "prn" => Some(tracedraw_io::eps::document_to_ps(&doc).len()),
        "pdf" | "ai" => Some(tracedraw_io::pdf::document_to_pdf(&doc).len()),
        "png" | "jpg" | "bmp" | "tif" | "webp" | "gif" | "psd" => {
            let enc = if ext == "psd" { "tif" } else { ext };
            let bytes = encode_raster_with(preview, enc, st.quality, st.webp_lossless)
                .ok()?
                .len() as f64;
            let dpi = st.render_dpi();
            let full = (bounds.width() / 25.4 * dpi) * (bounds.height() / 25.4 * dpi);
            let shown = (preview.width() as f64 * preview.height() as f64).max(1.0);
            let pages = if st.all_pages && !st.selection_only {
                doc.pages.len().max(1) as f64
            } else {
                1.0
            };
            Some((bytes * full / shown * pages) as usize)
        }
        _ => None,
    }
}

/// A byte count for people: "820 KB", "1.4 MB".
pub fn human_size(bytes: usize) -> String {
    let b = bytes as f64;
    if b < 1000.0 * 1024.0 {
        format!("{:.0} KB", (b / 1024.0).max(1.0))
    } else {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    }
}

/// Round each colour channel (not alpha) to fewer levels for WebP's
/// near-lossless quality: 100 keeps every value, lower qualities drop up
/// to 4 low bits.
fn near_lossless(img: &mut image::RgbaImage, quality: u8) {
    let bits: u32 = match quality {
        100.. => 0,
        90..=99 => 1,
        75..=89 => 2,
        50..=74 => 3,
        _ => 4,
    };
    if bits == 0 {
        return;
    }
    let half = 1u16 << (bits - 1);
    for p in img.pixels_mut() {
        for v in &mut p.0[..3] {
            let q = ((*v as u16 + half) >> bits) << bits;
            *v = q.min(255) as u8;
        }
    }
}

/// Encode a rendered page in the bitmap format `ext` (WebP lossless).
#[cfg(test)]
fn encode_raster(pm: &tiny_skia::Pixmap, ext: &str, quality: u8) -> Result<Vec<u8>, String> {
    encode_raster_with(pm, ext, quality, true)
}

/// A WebP file with lossy compression at `quality` (1 to 100), alpha
/// kept; `None` where no lossy encoder is built in (the browser).
#[cfg(not(target_arch = "wasm32"))]
fn webp_lossy(img: &image::RgbaImage, quality: u8) -> Option<Vec<u8>> {
    let enc = webp::Encoder::from_rgba(img.as_raw(), img.width(), img.height());
    Some(enc.encode(f32::from(quality.clamp(1, 100))).to_vec())
}

#[cfg(target_arch = "wasm32")]
fn webp_lossy(_img: &image::RgbaImage, _quality: u8) -> Option<Vec<u8>> {
    None
}

/// Encode a rendered page in the bitmap format `ext`; WebP lossless
/// (`lossless`, colours rounded below quality 100) or lossy at the
/// quality (#592).
fn encode_raster_with(
    pm: &tiny_skia::Pixmap,
    ext: &str,
    quality: u8,
    lossless: bool,
) -> Result<Vec<u8>, String> {
    let (w, h) = (pm.width(), pm.height());
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for p in pm.pixels() {
        let d = p.demultiply();
        rgba.extend_from_slice(&[d.red(), d.green(), d.blue(), d.alpha()]);
    }
    let img = image::RgbaImage::from_raw(w, h, rgba).ok_or("image buffer")?;
    let dynimg = image::DynamicImage::ImageRgba8(img);
    let format = match ext {
        "png" => image::ImageFormat::Png,
        "jpg" => image::ImageFormat::Jpeg,
        "bmp" => image::ImageFormat::Bmp,
        "tif" => image::ImageFormat::Tiff,
        "webp" => image::ImageFormat::WebP,
        "gif" => image::ImageFormat::Gif,
        _ => image::ImageFormat::Png,
    };
    let mut out = std::io::Cursor::new(Vec::new());
    match format {
        image::ImageFormat::Jpeg => {
            let rgb = dynimg.to_rgb8();
            let mut enc =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100));
            enc.encode_image(&rgb).map_err(|e| e.to_string())?;
        }
        image::ImageFormat::Bmp => {
            dynimg
                .to_rgb8()
                .write_to(&mut out, format)
                .map_err(|e| e.to_string())?;
        }
        image::ImageFormat::WebP => {
            let mut rgba = dynimg.to_rgba8();
            // Lossy: much smaller, the quality as the encoder takes it
            // (#592). Lossless (or no lossy encoder, in the browser): below
            // 100 the colours are first rounded to fewer levels, WebP's
            // near-lossless way, so quality still shrinks the file (#544).
            if let Some(bytes) = (!lossless).then(|| webp_lossy(&rgba, quality)).flatten() {
                return Ok(bytes);
            }
            near_lossless(&mut rgba, quality);
            rgba.write_to(&mut out, format).map_err(|e| e.to_string())?;
        }
        _ => dynimg
            .write_to(&mut out, format)
            .map_err(|e| e.to_string())?,
    }
    Ok(out.into_inner())
}

/// Print: build a PDF with the print options and send it to the chosen
/// system printer, or open it in the system viewer, which owns the
/// printer dialog.
pub fn print(app: &mut App, st: &PrintState) -> Result<String, String> {
    let mut doc = print_sheets(app, st);
    if st.separations {
        doc = separations_doc_without(&doc, &st.plates_off);
    }
    let stem = app.document_title().trim_end_matches('*').to_string();
    // Files get their copies inside; the printer (lp) or the system
    // viewer handles the count.
    if st.destination != 0 && st.copies > 1 {
        repeat_copies(&mut doc, st.copies.min(999) as usize, st.collate);
    }
    match st.destination {
        1 | 2 => {
            let ps = st.destination == 2;
            let (ext, filter) = if ps {
                ("ps", "PostScript")
            } else {
                ("pdf", "PDF")
            };
            let bytes = if ps {
                postscript_level(tracedraw_io::eps::document_to_ps(&doc), st.ps_level).into_bytes()
            } else {
                tracedraw_io::pdf::document_to_pdf(&doc)
            };
            crate::files::Dialog::new()
                .add_filter(filter, &[ext])
                .set_file_name(format!("{stem}.{ext}"))
                .save_file(app, move |app, path| {
                    app.status = match crate::files::write(&path, bytes) {
                        Ok(()) => trf("status.exported", &[("path", &path.display().to_string())]),
                        Err(e) => format!("{}: {e}", tr("dialog.print_failed")),
                    };
                });
            Ok(String::new())
        }
        _ => {
            let name = format!("{stem}.pdf");
            if let (Some(printer), false) = (&st.printer, crate::files::WEB) {
                // The sheets are drawn and spooled off the interface's
                // thread: a slow or stuck printer leaves the window usable
                // (#810).
                let (printer, copies, collate) = (printer.clone(), st.copies, st.collate);
                let msg = trf("status.printing_to", &[("name", &printer)]);
                app.start_print_job(move || {
                    print_on(&printer, &name, &doc, copies, collate, &stem)
                });
                return Ok(msg);
            }
            let pdf = tracedraw_io::pdf::document_to_pdf(&doc);
            if crate::files::WEB {
                // In a browser the PDF is downloaded; its viewer prints it.
                let path = std::path::PathBuf::from("/download").join(&name);
                crate::files::write(&path, pdf).map_err(|e| e.to_string())?;
                return Ok(trf("status.print_sent", &[("path", &name)]));
            }
            print_through_viewer(&name, pdf)
        }
    }
}

/// The print PDF sent to the system printer `printer` (`lp`), with the
/// copies the dialog asked for.
#[cfg(not(target_os = "windows"))]
fn print_on(
    printer: &str,
    name: &str,
    doc: &Document,
    copies: u32,
    collate: bool,
    title: &str,
) -> Result<String, String> {
    let pdf = tracedraw_io::pdf::document_to_pdf(doc);
    let dir = std::env::temp_dir().join("tracedraw-print");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(name);
    crate::files::write(&path, pdf).map_err(|e| e.to_string())?;
    crate::printers::print_pdf(printer, &path, copies, collate, title)?;
    Ok(trf("status.print_sent_printer", &[("name", printer)]))
}

/// The resolution sheets are drawn at for a Windows printer.
#[cfg(target_os = "windows")]
const WINDOWS_PRINT_DPI: f64 = 300.0;

/// The sheets drawn as pictures and sent to the Windows printer
/// `printer`, local or on the network (#614), with the copies the dialog
/// asked for.
#[cfg(target_os = "windows")]
fn print_on(
    printer: &str,
    _name: &str,
    doc: &Document,
    copies: u32,
    collate: bool,
    title: &str,
) -> Result<String, String> {
    let sheets: Vec<(Vec<u8>, f64, f64)> = doc
        .pages
        .iter()
        .filter_map(|p| {
            sheet_png(doc, p.id, WINDOWS_PRINT_DPI).map(|png| (png, p.size.width, p.size.height))
        })
        .collect();
    crate::printers::print_sheets_windows(printer, &sheets, copies, collate, title)?;
    Ok(trf("status.print_sent_printer", &[("name", printer)]))
}

/// Prints sent to the system viewer in this session.
#[cfg(not(target_arch = "wasm32"))]
static VIEWER_PRINTS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Where the `count`th print of `name` to the viewer goes in `dir`: the
/// first takes the name itself, later ones "name (2).pdf" and on, so a
/// viewer still showing (and holding) an earlier print opens the new one
/// in a page of its own (#809). Prints of the name before the last one
/// are removed once no viewer holds them.
#[cfg(not(target_arch = "wasm32"))]
fn viewer_print_path(dir: &Path, name: &str, count: u32) -> std::path::PathBuf {
    let p = Path::new(name);
    let stem = p.file_stem().map_or(name.into(), |s| s.to_string_lossy());
    let ext = p.extension().map_or("pdf".into(), |s| s.to_string_lossy());
    let nth = |n: u32| match n {
        0 | 1 => dir.join(name),
        n => dir.join(format!("{stem} ({n}).{ext}")),
    };
    for n in 1..count.saturating_sub(1) {
        let _ = std::fs::remove_file(nth(n));
    }
    // A file of that name left by another session and still held.
    let mut n = count.max(1);
    while n < count.max(1) + 100 && nth(n).exists() && std::fs::remove_file(nth(n)).is_err() {
        n += 1;
    }
    nth(n)
}

#[cfg(not(target_arch = "wasm32"))]
fn print_through_viewer(name: &str, pdf: Vec<u8>) -> Result<String, String> {
    let dir = std::env::temp_dir().join("tracedraw-print");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let count = VIEWER_PRINTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let path = viewer_print_path(&dir, name, count);
    crate::files::write(&path, pdf).map_err(|e| e.to_string())?;
    open_with_system(&path)?;
    Ok(trf(
        "status.print_sent",
        &[("path", &path.display().to_string())],
    ))
}

#[cfg(target_arch = "wasm32")]
fn print_through_viewer(name: &str, _pdf: Vec<u8>) -> Result<String, String> {
    Ok(trf("status.print_sent", &[("path", name)]))
}

/// The sheets repeated for `copies` copies: whole sets one after the other
/// (collated) or each sheet `copies` times.
fn repeat_copies(doc: &mut Document, copies: usize, collate: bool) {
    let original = std::mem::take(&mut doc.pages);
    let mut ids = doc.ids().clone();
    let mut push = |p: &tracedraw_core::document::Page, out: &mut Vec<_>| {
        let mut q = p.clone();
        q.id = ids.page();
        out.push(q);
    };
    let mut out = Vec::with_capacity(original.len() * copies);
    if collate {
        for _ in 0..copies {
            for p in &original {
                push(p, &mut out);
            }
        }
    } else {
        for p in &original {
            for _ in 0..copies {
                push(p, &mut out);
            }
        }
    }
    doc.set_ids(ids);
    doc.pages = out;
}

/// The PostScript file's language level in its header comments.
fn postscript_level(ps: String, level: u8) -> String {
    let level = level.clamp(2, 3);
    match ps.find('\n') {
        Some(i) => format!("{}\n%%LanguageLevel: {level}{}", &ps[..i], &ps[i..]),
        None => ps,
    }
}

/// The sheets the printer gets for the Print dialog's settings: the pages
/// imposed with their marks, mirrored and inverted when asked. Print and
/// Print Preview both start here, so the preview shows what prints.
/// The objects `page` shows (every one with `all`) scaled down and moved
/// as one to fit inside it, centred, when they run off it; objects wholly
/// off the page stay out of it.
fn fit_contents_to_page(page: &mut tracedraw_core::document::Page, all: bool) {
    const EPS: f64 = 1e-6;
    let area = page.rect();
    let shows = |b: &Rect| {
        all || (b.x0 <= area.x1 && b.x1 >= area.x0 && b.y0 <= area.y1 && b.y1 >= area.y0)
    };
    let Some(b) = page
        .layers
        .iter()
        .flat_map(|l| &l.shapes)
        .map(|s| s.bounds())
        .filter(|b| b.width().is_finite() && b.height().is_finite() && shows(b))
        .reduce(|a, b| a.union(b))
    else {
        return;
    };
    let inside = b.x0 >= area.x0 - EPS
        && b.y0 >= area.y0 - EPS
        && b.x1 <= area.x1 + EPS
        && b.y1 <= area.y1 + EPS;
    if inside {
        return;
    }
    let k = (area.width() / b.width().max(EPS))
        .min(area.height() / b.height().max(EPS))
        .min(1.0);
    let t = Affine::translate(area.center().to_vec2())
        * Affine::scale(k)
        * Affine::translate(-b.center().to_vec2());
    for layer in page.layers.iter_mut() {
        for s in layer.shapes.iter_mut() {
            if shows(&s.bounds()) {
                s.transform_by(t);
            }
        }
    }
}

pub fn print_sheets(app: &App, st: &PrintState) -> Document {
    let mut doc = app.engine.document().clone();
    let current = doc.pages.iter().position(|p| p.id == app.page).unwrap_or(0);
    let keep = st.page_indexes(doc.pages.len(), current);
    let mut i = 0;
    doc.pages.retain(|_| {
        i += 1;
        keep.contains(&(i - 1))
    });
    // Selection: only the selected objects of the current page.
    if st.range == 3 && !app.selection.is_empty() {
        for page in doc.pages.iter_mut() {
            for layer in page.layers.iter_mut() {
                layer.shapes.retain(|s| app.selection.contains(&s.id));
            }
        }
    }
    // Fit to page: objects running off the page (the selection, when
    // that is printed) shrink to fit inside it, centred, rather than
    // being cut at its edge (#534).
    if st.fit_to_page {
        for page in doc.pages.iter_mut() {
            fit_contents_to_page(page, st.range == 3);
        }
    }
    for page in doc.pages.iter_mut() {
        for layer in page.layers.iter_mut() {
            for s in layer.shapes.iter_mut() {
                if st.spot_to_cmyk {
                    map_colors(s, &spot_as_cmyk);
                }
                if st.text_black {
                    text_in_black(s);
                }
                if st.overprint_black {
                    overprint_black(s);
                }
            }
        }
    }
    let options = print_layout(app, st, &doc);
    let mut doc = crate::imposition::impose(&doc, &options);
    for page in doc.pages.iter_mut() {
        let w = page.size.width;
        for layer in page.layers.iter_mut() {
            for s in layer.shapes.iter_mut() {
                // Emulsion down: the whole sheet, marks too, mirrored.
                if st.mirror {
                    s.transform = Affine::translate((w, 0.0))
                        * Affine::new([-1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
                        * s.transform;
                }
                if st.invert {
                    invert_colors(s);
                }
            }
        }
    }
    // A negative turns the paper black too, so that what was black ink
    // (now white) shows as the clear part of the film.
    if st.invert {
        let mut ids = doc.ids().clone();
        for page in doc.pages.iter_mut() {
            let size = page.size;
            let mut ground = tracedraw_core::Shape::new(
                ids.shape(),
                ShapeKind::Rect {
                    rect: tracedraw_core::geometry::Rect::new(0.0, 0.0, size.width, size.height),
                    radius: 0.0,
                    corners: None,
                },
            );
            ground.fill = tracedraw_core::Fill::Solid(tracedraw_core::Color::BLACK);
            ground.stroke = None;
            if page.layers.is_empty() {
                page.layers
                    .push(tracedraw_core::Layer::new(ids.layer(), "Negative"));
            }
            if let Some(layer) = page.layers.first_mut() {
                layer.shapes.insert(0, ground);
            }
        }
        doc.set_ids(ids);
    }
    if st.as_bitmap {
        sheets_as_bitmaps(&mut doc, st.bitmap_dpi.clamp(36.0, 1200.0));
    }
    doc
}

/// A spot ink's colour as its CMYK mix at the tint.
fn spot_as_cmyk(c: tracedraw_core::Color) -> tracedraw_core::Color {
    match c {
        tracedraw_core::Color::Spot {
            tint, c, m, y, k, ..
        } => tracedraw_core::Color::Cmyk {
            c: c * tint,
            m: m * tint,
            y: y * tint,
            k: k * tint,
        },
        other => other,
    }
}

/// Composite > Print text in black: every text, in groups too, filled and
/// outlined in black.
fn text_in_black(s: &mut tracedraw_core::Shape) {
    use tracedraw_core::{Color, Fill};
    match &mut s.kind {
        ShapeKind::Text { spans, .. } => {
            if !matches!(s.fill, Fill::None) {
                s.fill = Fill::Solid(Color::BLACK);
            }
            if let Some(st) = &mut s.stroke {
                st.color = Color::BLACK;
            }
            for sp in spans.iter_mut() {
                if sp.fill.is_some() {
                    sp.fill = Some(Fill::Solid(Color::BLACK));
                }
            }
        }
        ShapeKind::Group { children } => children.iter_mut().for_each(text_in_black),
        ShapeKind::ClipFrame { contents, .. } => contents.iter_mut().for_each(text_in_black),
        _ => {}
    }
}

/// Whether `c` is black ink only (100 % black, no other ink).
fn is_pure_black(c: &tracedraw_core::Color) -> bool {
    match c.convert_to("CMYK") {
        tracedraw_core::Color::Cmyk { c, m, y, k } => {
            k >= 0.999 && c <= 0.001 && m <= 0.001 && y <= 0.001
        }
        _ => false,
    }
}

/// Composite > Always overprint black: black fills and outlines overprint.
fn overprint_black(s: &mut tracedraw_core::Shape) {
    use tracedraw_core::Fill;
    if let Fill::Solid(c) = &s.fill {
        if is_pure_black(c) {
            s.overprint_fill = true;
        }
    }
    if s.stroke.as_ref().is_some_and(|st| is_pure_black(&st.color)) {
        s.overprint_outline = true;
    }
    match &mut s.kind {
        ShapeKind::Group { children } => children.iter_mut().for_each(overprint_black),
        ShapeKind::ClipFrame { frame, contents } => {
            overprint_black(frame);
            contents.iter_mut().for_each(overprint_black);
        }
        _ => {}
    }
}

/// A sheet drawn on white at `dpi`: its PNG and size in pixels.
fn sheet_pixels(
    doc: &Document,
    id: tracedraw_core::PageId,
    dpi: f64,
) -> Option<(Vec<u8>, u32, u32)> {
    let pm = tracedraw_render::render_page_image(doc, id, dpi)?;
    let mut white = tiny_skia::Pixmap::new(pm.width(), pm.height())?;
    white.fill(tiny_skia::Color::WHITE);
    white.draw_pixmap(
        0,
        0,
        pm.as_ref(),
        &Default::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    let png = white.encode_png().ok()?;
    Some((png, white.width(), white.height()))
}

/// [`sheet_pixels`]'s PNG.
#[cfg(target_os = "windows")]
fn sheet_png(doc: &Document, id: tracedraw_core::PageId, dpi: f64) -> Option<Vec<u8>> {
    sheet_pixels(doc, id, dpi).map(|(png, _, _)| png)
}

/// General > Print as bitmap: each sheet becomes one picture of itself.
fn sheets_as_bitmaps(doc: &mut Document, dpi: f64) {
    let pages: Vec<_> = doc.pages.iter().map(|p| (p.id, p.size)).collect();
    for (id, size) in pages {
        let Some((png, w, h)) = sheet_pixels(doc, id, dpi) else {
            continue;
        };
        let shape_id = doc.ids_mut().shape();
        let mut shape = tracedraw_core::Shape::new(
            shape_id,
            ShapeKind::Bitmap {
                rect: tracedraw_core::geometry::Rect::new(0.0, 0.0, size.width, size.height),
                width_px: w,
                height_px: h,
                png: png.into(),
                fx: None,
            },
        );
        shape.fill = tracedraw_core::Fill::None;
        shape.stroke = None;
        if let Ok(page) = doc.page_mut(id) {
            if let Some(first) = page.layers.first_mut() {
                first.shapes = vec![shape];
            }
            page.layers.truncate(1);
        }
    }
}

/// A separation plate: a process ink (0 cyan to 3 black) or a spot ink by
/// its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plate {
    Process(usize),
    Spot(u32),
}

/// The plates of a drawing: the four process inks, then its spot inks,
/// each with its name.
pub fn plates(doc: &Document) -> Vec<(Plate, String)> {
    let mut plates: Vec<(Plate, String)> = vec![
        (Plate::Process(0), "Cyan".into()),
        (Plate::Process(1), "Magenta".into()),
        (Plate::Process(2), "Yellow".into()),
        (Plate::Process(3), "Black".into()),
    ];
    plates.extend(doc.inks.iter().map(|i| (Plate::Spot(i.id), i.name.clone())));
    plates
}

/// One plate of a page: every colour turned into the grey of the ink's
/// coverage (black = full ink); registration colour prints on all plates.
pub fn plate_page(
    page: &tracedraw_core::document::Page,
    plate: Plate,
) -> tracedraw_core::document::Page {
    use tracedraw_core::{Color, Fill};
    let mut p = page.clone();
    let coverage = move |c: Color| -> Color {
        if matches!(c, Color::Registration) {
            return Color::Gray { v: 0.0 };
        }
        let v = match (plate, c) {
            (Plate::Spot(id), Color::Spot { ink, tint, .. }) if ink == id => tint,
            (Plate::Spot(_), _) | (Plate::Process(_), Color::Spot { .. }) => 0.0,
            (Plate::Process(i), _) => match c.convert_to("CMYK") {
                Color::Cmyk { c, m, y, k } => [c, m, y, k].get(i).copied().unwrap_or(0.0),
                _ => 0.0,
            },
        };
        Color::Gray {
            v: 1.0 - v.clamp(0.0, 1.0),
        }
    };
    for layer in p.layers.iter_mut() {
        for s in layer.shapes.iter_mut() {
            // Fills the separations cannot split show as their average
            // colour.
            if matches!(s.fill, Fill::Texture(_) | Fill::Pattern(_)) {
                if let Some(c) = s.fill.preview_color() {
                    s.fill = Fill::Solid(c);
                }
            }
            map_colors(s, &coverage);
        }
    }
    p
}

/// A page as printed with only some inks (#571): process colours keep
/// the channels of the inks on, spot colours show when their ink is on
/// and are paper otherwise; registration colour prints with every ink.
pub fn inks_page(
    page: &tracedraw_core::document::Page,
    process: [bool; 4],
    spots: &[u32],
) -> tracedraw_core::document::Page {
    use tracedraw_core::{Color, Fill};
    let spots = spots.to_vec();
    let keep = move |c: Color| -> Color {
        let on = |i: usize, v: f32| if process[i] { v } else { 0.0 };
        match c {
            Color::Registration => Color::Cmyk {
                c: on(0, 1.0),
                m: on(1, 1.0),
                y: on(2, 1.0),
                k: on(3, 1.0),
            },
            Color::Spot { ink, .. } if spots.contains(&ink) => c,
            Color::Spot { .. } => Color::Gray { v: 1.0 },
            other => match other.convert_to("CMYK") {
                Color::Cmyk { c, m, y, k } => Color::Cmyk {
                    c: on(0, c),
                    m: on(1, m),
                    y: on(2, y),
                    k: on(3, k),
                },
                same => same,
            },
        }
    };
    let mut p = page.clone();
    for layer in p.layers.iter_mut() {
        for s in layer.shapes.iter_mut() {
            if matches!(s.fill, Fill::Texture(_) | Fill::Pattern(_)) {
                if let Some(c) = s.fill.preview_color() {
                    s.fill = Fill::Solid(c);
                }
            }
            map_colors(s, &keep);
        }
    }
    p
}

/// The print layout for the Print dialog's settings: the imposition, the
/// sheet, the bleed and the marks.
pub fn print_layout(app: &App, st: &PrintState, doc: &Document) -> crate::imposition::Options {
    let bleed = if st.bleed {
        st.bleed_limit.clamp(0.0, 50.0)
    } else {
        0.0
    };
    let first = doc
        .pages
        .first()
        .map(|p| p.size)
        .unwrap_or(Size::new(210.0, 297.0));
    let tz = crate::time_tracker::local_zone();
    crate::imposition::Options {
        layout: st.layout(),
        sheet: st.sheet_size_for(first),
        gutter: st.gutter.max(0.0),
        bleed,
        scale: if st.fit_to_page {
            1.0
        } else {
            st.scale / 100.0
        },
        back_side: st.back_side,
        keep_size: st.keep_size,
        info_offset: st.info_offset,
        paper_margins: st.margins,
        // Moved and stretched in Print Preview, the whole page with its
        // clip, clear of the printer's margins (#536, #595).
        page_offset: st.art_offset,
        page_stretch: st.art_stretch,
        stretch_fit: st.stretch_fit,
        marks: crate::imposition::Marks {
            crop: st.crop_marks,
            registration: st.registration_marks,
            color_bars: st.color_bars,
            densitometer: st.densitometer,
            page_info: st.page_info,
            page_numbers: st.page_numbers,
        },
        title: if st.info_text.trim().is_empty() {
            app.document_name()
        } else {
            st.info_text.trim().to_string()
        },
        when: crate::time_tracker::date_time(crate::time_tracker::unix_now(), &tz),
    }
}

/// Map every colour of `s` and what it holds through `f`: fills of every
/// kind, outlines, character fills, groups and clip frames.
pub fn map_colors(
    s: &mut tracedraw_core::Shape,
    f: &dyn Fn(tracedraw_core::Color) -> tracedraw_core::Color,
) {
    fn fill(
        x: &mut tracedraw_core::Fill,
        f: &dyn Fn(tracedraw_core::Color) -> tracedraw_core::Color,
    ) {
        use tracedraw_core::{Fill, Pattern};
        match x {
            Fill::Solid(c) => *c = f(*c),
            Fill::Fountain(g) => g.stops.iter_mut().for_each(|s| s.color = f(s.color)),
            Fill::Mesh(m) => m.nodes.iter_mut().for_each(|n| n.color = f(n.color)),
            Fill::Pattern(Pattern::TwoColor { front, back, .. }) => {
                *front = f(*front);
                *back = f(*back);
            }
            _ => {}
        }
    }
    fill(&mut s.fill, f);
    if let Some(st) = &mut s.stroke {
        st.color = f(st.color);
    }
    match &mut s.kind {
        ShapeKind::Group { children } => children.iter_mut().for_each(|c| map_colors(c, f)),
        ShapeKind::ClipFrame { frame, contents } => {
            map_colors(frame, f);
            contents.iter_mut().for_each(|c| map_colors(c, f));
        }
        ShapeKind::Text { spans, .. } => {
            for sp in spans.iter_mut() {
                if let Some(x) = &mut sp.fill {
                    fill(x, f);
                }
            }
        }
        _ => {}
    }
}

fn invert_colors(s: &mut tracedraw_core::Shape) {
    use tracedraw_core::Color;
    map_colors(s, &|c: Color| {
        let [r, g, b] = c.to_rgb8();
        Color::rgb8(255 - r, 255 - g, 255 - b)
    });
}

/// Colour separations with every plate.
#[cfg(test)]
fn separations_doc(doc: &Document) -> Document {
    separations_doc_without(doc, &[])
}

/// Colour separations: one grey page per sheet and plate (the four
/// process inks, then each spot ink), each the grey of that ink's
/// coverage; a spot colour prints on its own plate only, at its tint.
/// The plates at the indices `off` (in [`plates`] order), those unticked
/// in Print Preview, are left out (#846).
fn separations_doc_without(doc: &Document, off: &[usize]) -> Document {
    let size = doc
        .pages
        .first()
        .map(|p| p.size)
        .unwrap_or(Size::new(210.0, 297.0));
    let mut sep = Document::new(format!("{} (separations)", doc.title), size);
    sep.pages.clear();
    sep.symbols = doc.symbols.clone();
    let plates: Vec<(Plate, String)> = plates(doc)
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !off.contains(i))
        .map(|(_, p)| p)
        .collect();
    for page in &doc.pages {
        for (plate, name) in &plates {
            let mut p = plate_page(page, *plate);
            p.name = format!("{} {}", page.name, name);
            let mut ids = sep.ids().clone();
            p.id = ids.page();
            sep.set_ids(ids);
            sep.pages.push(p);
        }
    }
    sep
}

pub fn open_with_system(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd")
        .args(["/C", "start", "", &path.display().to_string()])
        .spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(path).spawn();
    r.map(|_| ())
        .map_err(|e| format!("{}: {e}", tr("status.could_not_open_viewer")))
}

/// The program and arguments that open `path` in `editor`: the
/// program itself, or on macOS an application bundle through `open -a`.
pub fn editor_command(editor: &str, path: &Path) -> (String, Vec<String>) {
    let editor = editor.trim().trim_matches('"');
    let file = path.display().to_string();
    if cfg!(target_os = "macos") && editor.trim_end_matches('/').ends_with(".app") {
        return ("open".into(), vec!["-a".into(), editor.into(), file]);
    }
    (editor.into(), vec![file])
}

/// Open `path` in the bitmap editor of the preferences, or with the
/// system's program when none is set (#593).
#[cfg(not(target_arch = "wasm32"))]
pub fn open_in_editor(editor: &str, path: &Path) -> Result<(), String> {
    if editor.trim().is_empty() {
        return open_with_system(path);
    }
    let (program, args) = editor_command(editor, path);
    std::process::Command::new(&program)
        .args(&args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{}: {program}: {e}", tr("status.could_not_open_viewer")))
}

impl App {
    /// Open a web page in the system browser (a new tab in the browser
    /// build).
    pub fn open_url(&mut self, url: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::open_url(url);
            return;
        }
        #[allow(unreachable_code)]
        if let Err(e) = open_with_system(Path::new(url)) {
            self.status = e;
        }
    }

    /// File name for a PDF sent somewhere: the document title, cleaned.
    fn send_to_file_name(&self) -> String {
        let title = self.document_title();
        let stem: String = title
            .trim_end_matches('*')
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let stem = stem.trim();
        let stem = if stem.is_empty() { "Graphic1" } else { stem };
        format!("{stem}.pdf")
    }

    /// File > Send To: write the document as PDF into `target` and report
    /// where it went.
    pub fn send_to(&mut self, target: SendTarget) {
        if crate::files::WEB {
            // No folders in a browser: every target is a PDF download.
            let name = self.send_to_file_name();
            let pdf = tracedraw_io::pdf::document_to_pdf(self.engine.document());
            let path = std::path::PathBuf::from("/download").join(&name);
            self.status = match crate::files::write(&path, pdf) {
                Ok(()) => trf("status.sent_to", &[("path", &name)]),
                Err(e) => trf("status.export_failed", &[("e", &e.to_string())]),
            };
            let _ = target;
            return;
        }
        let Some(dir) = target.dir() else {
            self.status = tr("status.send_to_no_folder");
            return;
        };
        let path = dir.join(self.send_to_file_name());
        match tracedraw_io::save_pdf(self.engine.document(), &path) {
            Ok(()) => {
                self.status = trf("status.sent_to", &[("path", &path.display().to_string())]);
                if target == SendTarget::Mail {
                    // The platform's mail client gets the PDF as an
                    // attachment; without a mailto attachment standard,
                    // open the file's folder so it can be dragged in.
                    let _ = open_with_system(&dir);
                }
            }
            Err(e) => self.status = trf("status.export_failed", &[("e", &e.to_string())]),
        }
    }
}

/// Destinations of File > Send To.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendTarget {
    Desktop,
    Documents,
    Mail,
}

impl SendTarget {
    /// The folder the PDF is written to; the mail recipient gets it from
    /// the system's temporary folder.
    pub fn dir(self) -> Option<std::path::PathBuf> {
        match self {
            SendTarget::Desktop => user_dir("XDG_DESKTOP_DIR", "Desktop"),
            SendTarget::Documents => user_dir("XDG_DOCUMENTS_DIR", "Documents"),
            SendTarget::Mail => {
                let d = std::env::temp_dir().join("tracedraw-send");
                std::fs::create_dir_all(&d).ok()?;
                Some(d)
            }
        }
    }
}

/// `$XDG_*_DIR`, then `~/<name>` (created when missing), then the home
/// folder itself.
fn user_dir(env: &str, name: &str) -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os(env) {
        let p = std::path::PathBuf::from(p);
        if p.is_dir() {
            return Some(p);
        }
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)?;
    let p = home.join(name);
    if p.is_dir() || std::fs::create_dir_all(&p).is_ok() {
        Some(p)
    } else {
        Some(home)
    }
}

// ----- print merge ---------------------------------------------------------------

/// The text of a data file as spreadsheets save it: UTF-8 (with or
/// without the byte order mark), UTF-16 with its mark (Excel's Unicode
/// Text), else code page 1252 (Excel's CSV on Windows), which failed to
/// read as UTF-8 (#495).
pub fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    let utf16 = |rest: &[u8], le: bool| {
        let units: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| {
                if le {
                    u16::from_le_bytes([c[0], c[1]])
                } else {
                    u16::from_be_bytes([c[0], c[1]])
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(rest, false);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes
            .iter()
            .map(|&b| crate::ui::glyphs_docker::from_cp1252(b).unwrap_or('\u{FFFD}'))
            .collect(),
    }
}

/// Records of a CSV text: the first row names the fields; the separator
/// is the comma, semicolon or tab the first row holds most of.
pub fn parse_csv(text: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let first = text.lines().next().unwrap_or_default();
    let delim = b",;\t"
        .iter()
        .copied()
        .max_by_key(|d| first.matches(*d as char).count())
        .filter(|d| first.contains(*d as char))
        .unwrap_or(b',');
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delim)
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .map(|h| h.iter().map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = rdr
        .records()
        .filter_map(|r| r.ok())
        .map(|r| r.iter().map(|s| s.to_string()).collect())
        .collect();
    (headers, rows)
}

// ----- find and replace ----------------------------------------------------------

fn matches_text(hay: &str, needle: &str, match_case: bool, whole_word: bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    let (h, n) = if match_case {
        (hay.to_string(), needle.to_string())
    } else {
        (hay.to_lowercase(), needle.to_lowercase())
    };
    if !whole_word {
        return h.contains(&n);
    }
    h.split(|c: char| !c.is_alphanumeric()).any(|w| w == n)
}

/// Find text objects containing `needle` (kind 0) or objects whose kind or
/// name matches it (kind 1).
pub fn find_shapes(
    app: &App,
    needle: &str,
    kind: usize,
    match_case: bool,
    whole_word: bool,
) -> Vec<ShapeId> {
    let Ok(page) = app.doc().page(app.page) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    fn walk(
        s: &tracedraw_core::Shape,
        needle: &str,
        kind: usize,
        mc: bool,
        ww: bool,
        out: &mut Vec<ShapeId>,
    ) {
        let hit = match (kind, &s.kind) {
            (0, ShapeKind::Text { spans, .. }) => {
                let t: String = spans.iter().map(|x| x.text.as_str()).collect();
                matches_text(&t, needle, mc, ww)
            }
            (1, k) => {
                let name = s.name.clone().unwrap_or_default();
                let kind_name = crate::ui::dockers::kind_name(k);
                matches_text(&name, needle, mc, ww) || matches_text(&kind_name, needle, mc, false)
            }
            _ => false,
        };
        if hit {
            out.push(s.id);
        }
        match &s.kind {
            ShapeKind::Group { children } => children
                .iter()
                .for_each(|c| walk(c, needle, kind, mc, ww, out)),
            ShapeKind::ClipFrame { frame, contents } => {
                walk(frame, needle, kind, mc, ww, out);
                contents
                    .iter()
                    .for_each(|c| walk(c, needle, kind, mc, ww, out));
            }
            _ => {}
        }
    }
    for s in page.layers.iter().flat_map(|l| &l.shapes) {
        walk(s, needle, kind, match_case, whole_word, &mut out);
    }
    // Only top-level ids are selectable; keep those.
    out.retain(|id| app.doc().shape(*id).is_ok());
    out
}

pub fn replace_text(
    app: &mut App,
    needle: &str,
    replacement: &str,
    match_case: bool,
    whole_word: bool,
    all: bool,
) {
    let ids = find_shapes(app, needle, 0, match_case, whole_word);
    let ids: Vec<ShapeId> = if all {
        ids
    } else {
        ids.into_iter().take(1).collect()
    };
    let mut cmds = Vec::new();
    for id in ids {
        let Some(s) = app.doc().find_shape(id).cloned() else {
            continue;
        };
        if let ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        } = s.kind
        {
            let new_spans: Vec<TextSpan> = spans
                .into_iter()
                .map(|sp| TextSpan {
                    text: replace_in(&sp.text, needle, replacement, match_case, whole_word),
                    ..sp
                })
                .collect();
            cmds.push(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Text {
                    spans: new_spans,
                    origin,
                    frame,
                    align,
                    para,
                    on_path,
                },
            });
        }
    }
    if !cmds.is_empty() {
        if let Err(e) = app.engine.run_batch("Replace", &cmds) {
            app.status = e.to_string();
        }
    }
}

fn replace_in(
    text: &str,
    needle: &str,
    replacement: &str,
    match_case: bool,
    whole_word: bool,
) -> String {
    if needle.is_empty() {
        return text.to_string();
    }
    if whole_word {
        let mut out = String::new();
        let mut word = String::new();
        let flush = |word: &mut String, out: &mut String| {
            if !word.is_empty() {
                let eq = if match_case {
                    word == needle
                } else {
                    word.to_lowercase() == needle.to_lowercase()
                };
                out.push_str(if eq { replacement } else { word });
                word.clear();
            }
        };
        for c in text.chars() {
            if c.is_alphanumeric() {
                word.push(c);
            } else {
                flush(&mut word, &mut out);
                out.push(c);
            }
        }
        flush(&mut word, &mut out);
        return out;
    }
    if match_case {
        return text.replace(needle, replacement);
    }
    let lower = text.to_lowercase();
    let nl = needle.to_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while let Some(pos) = lower[i..].find(&nl) {
        out.push_str(&text[i..i + pos]);
        out.push_str(replacement);
        i += pos + nl.len();
    }
    out.push_str(&text[i..]);
    out
}

#[cfg(test)]
mod tests {
    /// Each print to the viewer gets a name of its own, so the viewer
    /// opens it in a new page while the last one is still shown; older
    /// ones are cleared away (#809).
    #[test]
    fn each_print_to_the_viewer_gets_its_own_file() {
        let dir = std::env::temp_dir().join(format!("tracedraw-viewer-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut paths = Vec::new();
        for count in 1..=3 {
            let p = super::viewer_print_path(&dir, "Poster.pdf", count);
            std::fs::write(&p, b"%PDF").unwrap();
            paths.push(p);
        }
        let names: Vec<_> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["Poster.pdf", "Poster (2).pdf", "Poster (3).pdf"]);
        assert!(!paths[0].exists(), "the oldest is cleared away");
        assert!(paths[1].exists() && paths[2].exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    use super::*;

    /// A spot colour on a rectangle: printed with separations, the sheets
    /// give the four process plates and one for the spot ink, where the
    /// rectangle is full ink; the print preview offers its box (#779).
    #[test]
    fn separations_print_a_plate_for_each_spot_ink() {
        use tracedraw_core::Fill;
        let mut app = App::headless();
        app.run(tracedraw_core::Command::SetInks {
            inks: vec![tracedraw_core::document::Ink {
                id: 7,
                name: "Spot Red".into(),
                cmyk: [0.0, 1.0, 0.8, 0.0],
            }],
        });
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(20.0, 20.0, 80.0, 60.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.run(tracedraw_core::Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(tracedraw_core::Color::Spot {
                ink: 7,
                tint: 1.0,
                c: 0.0,
                m: 1.0,
                y: 0.8,
                k: 0.0,
            }),
        });
        let st = PrintState {
            separations: true,
            ..PrintState::default()
        };
        let sheets = print_sheets(&app, &st);
        let names: Vec<String> = plates(&sheets).into_iter().map(|(_, n)| n).collect();
        assert_eq!(names, ["Cyan", "Magenta", "Yellow", "Black", "Spot Red"]);
        let seps = separations_doc(&sheets);
        assert_eq!(seps.pages.len(), 5);
        // Plates unticked in Print Preview do not print (#846).
        let some = separations_doc_without(&sheets, &[0, 2]);
        let kept: Vec<&str> = some.pages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(kept.len(), 3, "{kept:?}");
        assert!(
            kept[0].ends_with("Magenta") && kept[2].ends_with("Spot Red"),
            "{kept:?}"
        );
        let spot = &seps.pages[4];
        assert!(spot.name.ends_with("Spot Red"), "{}", spot.name);
        // The rectangle is full ink on its plate and absent on cyan.
        let fills = |page: &tracedraw_core::document::Page| -> Vec<Fill> {
            let mut out = Vec::new();
            fn walk(s: &tracedraw_core::Shape, out: &mut Vec<Fill>) {
                if let ShapeKind::Group { children } = &s.kind {
                    children.iter().for_each(|c| walk(c, out));
                } else if let ShapeKind::ClipFrame { contents, .. } = &s.kind {
                    contents.iter().for_each(|c| walk(c, out));
                } else {
                    out.push(s.fill.clone());
                }
            }
            for l in &page.layers {
                l.shapes.iter().for_each(|s| walk(s, &mut out));
            }
            out
        };
        let full = Fill::Solid(tracedraw_core::Color::Gray { v: 0.0 });
        assert!(fills(spot).contains(&full), "{:?}", fills(spot));
        assert!(!fills(&seps.pages[0]).contains(&full));
    }

    #[test]
    fn spot_inks_get_plates_of_their_own() {
        use tracedraw_core::{document::Ink, Fill};
        let mut doc = Document::new("s", tracedraw_core::Size::new(50.0, 50.0));
        doc.inks.push(Ink {
            id: 7,
            name: "Gold".into(),
            cmyk: [0.0, 0.2, 0.8, 0.1],
        });
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let mut s = tracedraw_core::Shape::new(
            id,
            ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        s.fill = Fill::Solid(doc.inks[0].color(0.6));
        s.stroke = None;
        doc.layer_mut(layer).expect("layer").shapes.push(s);
        let pdf = tracedraw_io::pdf::document_to_pdf(&separations_doc(&doc));
        let text = String::from_utf8_lossy(&pdf);
        // C, M, Y, K and the spot plate.
        assert!(
            text.contains("/Count 5"),
            "{}",
            &text[..text.len().min(400)]
        );
    }

    #[test]
    fn the_export_preview_shows_the_page_and_estimates_the_size() {
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 100.0, 60.0),
            radius: 0.0,
            corners: None,
        });
        // PNG at 300 dpi: an A4 preview keeps the page's proportions.
        let st = ExportState {
            format: 2,
            dpi: 300.0,
            ..ExportState::default()
        };
        let pm = preview(&app, &st, 200).expect("preview");
        assert_eq!(pm.height(), 200);
        assert!((pm.width() as f64 - 200.0 * 210.0 / 297.0).abs() <= 1.0);
        let png = estimated_size(&app, &st, &pm).expect("estimate");
        assert!(png > 0);
        // The same page at 72 dpi is estimated smaller.
        let small = ExportState {
            dpi: 72.0,
            ..st.clone()
        };
        assert!(estimated_size(&app, &small, &pm).unwrap_or(usize::MAX) < png);
        // A vector format is measured, not estimated.
        let svg = ExportState {
            format: 0,
            ..st.clone()
        };
        assert!(estimated_size(&app, &svg, &pm).is_some());
        assert!(keeps_alpha("png") && !keeps_alpha("jpg"));
    }

    /// A bitmap export records the resolution of the dialog, and the
    /// scale changes how many pixels it has but not that resolution, so it
    /// opens at the size chosen (#617).
    #[test]
    fn bitmap_exports_record_their_resolution_and_scale() {
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 100.0, 60.0),
            radius: 0.0,
            corners: None,
        });
        let b = app.page_rect();
        let dir = std::env::temp_dir().join(format!("tracedraw-617-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (format, ext) in [(2, "png"), (3, "jpg"), (4, "bmp")] {
            let st = ExportState {
                format,
                dpi: 50.0,
                scale: 2.0,
                transparent: false,
                ..ExportState::default()
            };
            let path = dir.join(format!("out.{ext}"));
            export(&mut app, &path, &st).expect(ext);
            let bytes = std::fs::read(&path).unwrap();
            let (x, y) = crate::image_dpi::resolution(&bytes).expect(ext);
            assert!(
                (x - 50.0).abs() < 0.1 && (y - 50.0).abs() < 0.1,
                "{ext}: {x} {y}"
            );
            // Still a file every reader opens, twice the pixels of 50 dpi.
            let img = image::load_from_memory(&bytes).expect(ext);
            let want = crate::ui::dialogs::export_pixels(b.width(), b.height(), 100.0);
            assert_eq!((img.width(), img.height()), want, "{ext}");
        }
        let _ = std::fs::remove_dir_all(&dir);
        // Files not laid out as expected come back as they were.
        assert_eq!(record_dpi(vec![1, 2, 3], "png", 300.0), vec![1, 2, 3]);
        assert_eq!(record_dpi(b"BM".to_vec(), "bmp", 300.0), b"BM".to_vec());
        assert_eq!(
            record_dpi(vec![0xFF, 0xD8], "jpg", f64::NAN),
            vec![0xFF, 0xD8]
        );
    }

    /// The bitmap size in the Export dialog: in pixels it sets the
    /// resolution as before, in a length it sets the scale; the other side
    /// follows and nonsense is ignored (#617).
    #[test]
    fn the_export_size_is_typed_in_pixels_or_lengths() {
        use crate::ui::dialogs::{export_size_in_unit, set_export_size};
        let mut st = ExportState {
            dpi: 100.0,
            size_unit: 3,
            ..ExportState::default()
        };
        let near =
            |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6;
        assert!(near(export_size_in_unit(254.0, 127.0, &st), (10.0, 5.0)));
        set_export_size(&mut st, 254.0, 20.0);
        assert!((st.scale - 2.0).abs() < 1e-9 && st.dpi == 100.0);
        assert!(near(export_size_in_unit(254.0, 127.0, &st), (20.0, 10.0)));
        assert!((st.render_dpi() - 200.0).abs() < 1e-9);
        st.size_unit = 1;
        assert!(near(export_size_in_unit(254.0, 127.0, &st), (508.0, 254.0)));
        st.size_unit = 0;
        assert!(near(
            export_size_in_unit(254.0, 127.0, &st),
            (2000.0, 1000.0)
        ));
        set_export_size(&mut st, 254.0, 1000.0);
        assert!((st.dpi - 50.0).abs() < 1e-3 && (st.scale - 2.0).abs() < 1e-9);
        assert!(near(
            export_size_in_unit(254.0, 127.0, &st),
            (1000.0, 500.0)
        ));
        // Never past the highest resolution, and nonsense changes nothing.
        st.size_unit = 3;
        set_export_size(&mut st, 254.0, 1.0e9);
        assert!(st.render_dpi() <= 2400.0 + 1e-6);
        let before = st.clone();
        for typed in [0.0, -5.0, f64::NAN, f64::INFINITY] {
            set_export_size(&mut st, 254.0, typed);
        }
        set_export_size(&mut st, 0.0, 10.0);
        assert_eq!(st, before);
        st.scale = f64::NAN;
        assert_eq!(st.scale_or_one(), 1.0);
    }

    #[test]
    fn colours_are_mapped_inside_groups_frames_and_text() {
        use tracedraw_core::{document::TextSpan, Color, Fill, Rect, Shape, ShapeId};
        let red = Color::Rgb {
            r: 1.0,
            g: 0.0,
            b: 0.0,
        };
        let mut inner = Shape::new(
            ShapeId(2),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                radius: 0.0,
                corners: None,
            },
        );
        inner.fill = Fill::Solid(red);
        let mut span = TextSpan::new("a", "Inter", 10.0);
        span.fill = Some(Fill::Solid(red));
        let text = Shape::new(
            ShapeId(3),
            ShapeKind::Text {
                spans: vec![span],
                origin: tracedraw_core::Point::ZERO,
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        let group = Shape::new(
            ShapeId(4),
            ShapeKind::Group {
                children: vec![inner, text],
            },
        );
        let frame = Shape::new(
            ShapeId(5),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                radius: 0.0,
                corners: None,
            },
        );
        let mut clip = Shape::new(
            ShapeId(1),
            ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: vec![group],
            },
        );
        let green = Color::Gray { v: 0.5 };
        map_colors(&mut clip, &|_| green);
        let ShapeKind::ClipFrame { contents, frame } = &clip.kind else {
            panic!("clip frame");
        };
        assert_eq!(frame.stroke.as_ref().map(|s| s.color), Some(green));
        let ShapeKind::Group { children } = &contents[0].kind else {
            panic!("group");
        };
        assert_eq!(children[0].fill, Fill::Solid(green));
        let ShapeKind::Text { spans, .. } = &children[1].kind else {
            panic!("text");
        };
        assert_eq!(spans[0].fill, Some(Fill::Solid(green)));
    }

    #[test]
    fn page_lists_and_ranges_pick_pages() {
        use crate::ui::dialogs::parse_page_list;
        assert_eq!(parse_page_list("1-3, 5", 6), vec![0, 1, 2, 4]);
        assert_eq!(parse_page_list("4-", 5), vec![3, 4]);
        assert_eq!(parse_page_list("9, x, 2", 3), vec![1]);
        let mut st = PrintState::default();
        assert_eq!(st.page_indexes(3, 1), vec![0, 1, 2]);
        st.range = 1;
        assert_eq!(st.page_indexes(3, 1), vec![1]);
        st.range = 2;
        st.pages = "3".into();
        assert_eq!(st.page_indexes(3, 0), vec![2]);
        // Odd or even pages of the document or the list (#951); the
        // current page stays, and a choice leaving no page is let go.
        st.parity = 1;
        assert_eq!(st.page_indexes(3, 0), vec![2]);
        st.pages = "1-6".into();
        assert_eq!(st.page_indexes(6, 0), vec![0, 2, 4]);
        st.parity = 2;
        assert_eq!(st.page_indexes(6, 0), vec![1, 3, 5]);
        st.pages = "3".into();
        assert_eq!(st.page_indexes(6, 0), vec![2], "nothing left: as listed");
        st.range = 0;
        assert_eq!(st.page_indexes(5, 0), vec![1, 3]);
        st.range = 1;
        assert_eq!(st.page_indexes(5, 0), vec![0]);
    }

    /// Black text prints black, and a negative (Invert) prints it as the
    /// clear part of a black sheet rather than white on white paper.
    #[test]
    fn black_text_prints_black_and_a_negative_blackens_the_sheet() {
        use tracedraw_core::{Color, Fill};
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Card", "Sans", 24.0)],
                origin: tracedraw_core::geometry::Point::new(20.0, 100.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            })
            .expect("text");
        let k100 = Color::cmyk_pct(0.0, 0.0, 0.0, 100.0);
        app.run(tracedraw_core::Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(k100),
        });
        // The sheet may hold the page in groups or clip frames.
        fn texts(s: &tracedraw_core::Shape, out: &mut Vec<Fill>) {
            match &s.kind {
                ShapeKind::Text { .. } => out.push(s.fill.clone()),
                ShapeKind::Group { children } => children.iter().for_each(|c| texts(c, out)),
                ShapeKind::ClipFrame { frame, contents } => {
                    texts(frame, out);
                    contents.iter().for_each(|c| texts(c, out));
                }
                _ => {}
            }
        }
        let text_fill = |doc: &Document| -> Vec<Fill> {
            let mut out = Vec::new();
            for l in doc.all_layers() {
                l.shapes.iter().for_each(|s| texts(s, &mut out));
            }
            out
        };
        let st = PrintState::default();
        let sheets = print_sheets(&app, &st);
        assert_eq!(text_fill(&sheets), vec![Fill::Solid(k100)]);
        // The PDF as text with its zlib streams inflated.
        let bytes = tracedraw_io::pdf::document_to_pdf(&sheets);
        let mut pdf = String::from_utf8_lossy(&bytes).into_owned();
        let mut rest: &[u8] = &bytes;
        while let Some(i) = rest.windows(7).position(|w| w == b"stream\n") {
            let data = &rest[i + 7..];
            use std::io::Read;
            let mut out = Vec::new();
            if flate2::read::ZlibDecoder::new(data)
                .read_to_end(&mut out)
                .is_ok()
            {
                pdf.push_str(&String::from_utf8_lossy(&out));
            }
            rest = data;
        }
        assert!(pdf.contains("0 0 0 1 k"), "black ink in the PDF");
        let st = PrintState {
            invert: true,
            ..PrintState::default()
        };
        let sheets = print_sheets(&app, &st);
        match text_fill(&sheets).as_slice() {
            [Fill::Solid(c)] => assert!(c.to_rgb8().iter().all(|v| *v > 180), "{c:?}"),
            other => panic!("{other:?}"),
        }
        let first = &sheets.pages[0].layers[0].shapes[0];
        assert_eq!(first.fill, Fill::Solid(Color::BLACK));
        let b = first.bounds();
        let size = sheets.pages[0].size;
        assert!(b.x0 <= 0.0 && b.y0 <= 0.0 && b.x1 >= size.width && b.y1 >= size.height);
    }

    #[test]
    fn print_colour_options_change_the_sheets_objects() {
        use tracedraw_core::{Color, Fill};
        let spot = Color::Spot {
            ink: 9,
            tint: 0.5,
            c: 1.0,
            m: 0.0,
            y: 0.0,
            k: 0.0,
        };
        assert_eq!(
            spot_as_cmyk(spot),
            Color::Cmyk {
                c: 0.5,
                m: 0.0,
                y: 0.0,
                k: 0.0
            }
        );
        let mut doc = Document::new("t", Size::new(50.0, 50.0));
        let id = doc.ids_mut().shape();
        let mut text = tracedraw_core::Shape::new(
            id,
            ShapeKind::Text {
                spans: vec![TextSpan::new("a", "Arial", 12.0)],
                origin: tracedraw_core::geometry::Point::ZERO,
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        text.fill = Fill::Solid(Color::cmyk_pct(0.0, 100.0, 0.0, 0.0));
        text_in_black(&mut text);
        assert_eq!(text.fill, Fill::Solid(Color::BLACK));
        let mut black = text.clone();
        black.fill = Fill::Solid(Color::cmyk_pct(0.0, 0.0, 0.0, 100.0));
        overprint_black(&mut black);
        assert!(black.overprint_fill);
        let mut red = text;
        red.fill = Fill::Solid(Color::cmyk_pct(0.0, 100.0, 100.0, 0.0));
        overprint_black(&mut red);
        assert!(!red.overprint_fill);
    }

    #[test]
    fn copies_repeat_sheets_collated_or_not_and_ps_gets_its_level() {
        let mut doc = Document::new("t", Size::new(50.0, 50.0));
        let mut ids = doc.ids().clone();
        let mut second = doc.pages[0].clone();
        second.id = ids.page();
        second.name = "B".into();
        doc.set_ids(ids);
        doc.pages[0].name = "A".into();
        doc.pages.push(second);
        let mut a = doc.clone();
        repeat_copies(&mut a, 2, true);
        let names: Vec<_> = a.pages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["A", "B", "A", "B"]);
        repeat_copies(&mut doc, 2, false);
        let names: Vec<_> = doc.pages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["A", "A", "B", "B"]);
        let ids: std::collections::BTreeSet<_> = doc.pages.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 4);
        let ps = postscript_level("%!PS-Adobe-3.0\n%%EOF\n".into(), 3);
        assert!(ps.starts_with("%!PS-Adobe-3.0\n%%LanguageLevel: 3\n"));
    }

    #[test]
    fn the_print_dialog_settings_make_the_layout() {
        let app = App::headless();
        let mut st = PrintState {
            layout_kind: 3,
            cols: 4,
            rows: 0,
            sheet: 2,
            landscape: true,
            crop_marks: true,
            ..Default::default()
        };
        let o = print_layout(&app, &st, app.doc());
        assert_eq!(
            o.layout,
            crate::imposition::Layout::Repeat { cols: 4, rows: 1 }
        );
        assert_eq!(o.sheet, Some(Size::new(297.0, 210.0)));
        assert!(o.marks.crop && !o.marks.registration);
        // Bleed only when asked for, as far as the bleed limit.
        st.bleed_limit = 3.0;
        assert_eq!(print_layout(&app, &st, app.doc()).bleed, 0.0);
        st.bleed = true;
        assert_eq!(print_layout(&app, &st, app.doc()).bleed, 3.0);
        // Orientation overrides the Landscape box.
        st.orientation = 1;
        assert_eq!(
            print_layout(&app, &st, app.doc()).sheet,
            Some(Size::new(210.0, 297.0))
        );
        st.orientation = 0;
        st.sheet = 0;
        assert_eq!(print_layout(&app, &st, app.doc()).sheet, None);
    }

    #[test]
    fn sizes_read_in_kilobytes_and_megabytes() {
        assert_eq!(human_size(10), "1 KB");
        assert_eq!(human_size(820 * 1024), "820 KB");
        assert_eq!(human_size(3 * 1024 * 1024 + 400 * 1024), "3.4 MB");
    }

    #[test]
    fn csv_parses_headers_and_rows() {
        let (h, r) = parse_csv("Name,City\nAna,Porto\nBob,Lisboa\n");
        assert_eq!(h, vec!["Name", "City"]);
        assert_eq!(r.len(), 2);
        assert_eq!(r[1][1], "Lisboa");
        let (h2, _) = parse_csv("a;b\n1;2\n");
        assert_eq!(h2, vec!["a", "b"]);
        let (h3, r3) = parse_csv("a\tb, c\n1\t2\n");
        assert_eq!(h3, vec!["a", "b, c"]);
        assert_eq!(r3[0], vec!["1", "2"]);
    }

    /// Data files as spreadsheets save them read: UTF-8 with its mark (no
    /// stray mark in the first field), UTF-16 (Unicode Text) and code page
    /// 1252, which Excel writes CSV in on Windows (#495).
    #[test]
    fn data_files_in_every_encoding_read() {
        let mut bom = vec![0xEF, 0xBB, 0xBF];
        bom.extend("Nome,Cidade\nJoão,Évora\n".as_bytes());
        let (h, r) = parse_csv(&decode_text(&bom));
        assert_eq!(h, vec!["Nome", "Cidade"]);
        assert_eq!(r[0], vec!["João", "Évora"]);
        let mut le = vec![0xFF, 0xFE];
        for u in "Name\tCity\nAna\tPorto\n".encode_utf16() {
            le.extend(u.to_le_bytes());
        }
        let (h, r) = parse_csv(&decode_text(&le));
        assert_eq!(h, vec!["Name", "City"]);
        assert_eq!(r[0], vec!["Ana", "Porto"]);
        // "João" and the euro sign in code page 1252 are not UTF-8.
        let ansi = b"Name;Price\nJo\xe3o;5 \x80\n";
        let (h, r) = parse_csv(&decode_text(ansi));
        assert_eq!(h, vec!["Name", "Price"]);
        assert_eq!(r[0], vec!["João", "5 €"]);
    }

    #[test]
    fn replace_variants() {
        assert_eq!(replace_in("Hello hello", "hello", "x", false, false), "x x");
        assert_eq!(
            replace_in("Hello hello", "hello", "x", true, false),
            "Hello x"
        );
        assert_eq!(
            replace_in("cat concat", "cat", "dog", false, true),
            "dog concat"
        );
    }

    /// WebP takes the quality: lower values make smaller files (colours
    /// rounded, near-lossless), 100 keeps the image exact (#544).
    #[test]
    fn webp_quality_shrinks_the_file() {
        let mut pm = tiny_skia::Pixmap::new(256, 128).expect("pixmap");
        for (i, p) in pm.pixels_mut().iter_mut().enumerate() {
            let (x, y) = ((i % 256) as u8, (i / 256) as u8);
            let n = ((x as u32 * 7 + y as u32 * 13) % 5) as u8;
            *p = tiny_skia::PremultipliedColorU8::from_rgba(x, y.wrapping_mul(2), x ^ y ^ n, 255)
                .expect("pixel");
        }
        // Lossy: far smaller at a low quality, and still a WebP (#592).
        let lossy = encode_raster_with(&pm, "webp", 10, false).expect("webp");
        let lossy_high = encode_raster_with(&pm, "webp", 90, false).expect("webp");
        assert!(
            lossy.len() < lossy_high.len(),
            "{} vs {}",
            lossy.len(),
            lossy_high.len()
        );
        assert!(image::load_from_memory(&lossy).is_ok());
        let full = encode_raster(&pm, "webp", 100).expect("webp");
        assert!(
            lossy.len() * 3 < full.len(),
            "{} vs {}",
            lossy.len(),
            full.len()
        );
        let low = encode_raster(&pm, "webp", 40).expect("webp");
        assert!(
            low.len() < full.len() * 3 / 4,
            "{} vs {}",
            low.len(),
            full.len()
        );
        let back = image::load_from_memory(&full).expect("decode").to_rgba8();
        assert_eq!(back.get_pixel(37, 21).0, {
            let d = pm.pixel(37, 21).expect("pixel").demultiply();
            [d.red(), d.green(), d.blue(), d.alpha()]
        });
    }

    /// The artwork moved in Print Preview prints moved by that much on
    /// the sheet; at first it stays (#536).
    #[test]
    fn the_artwork_moved_in_print_preview_prints_moved() {
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: tracedraw_core::geometry::Rect::new(20.0, 20.0, 60.0, 50.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
        let st = PrintState {
            fit_to_page: false,
            ..PrintState::default()
        };
        // The page goes on the sheet in a frame that clips it: the
        // rectangle is inside, with the frame's transform.
        let bounds = |doc: &Document| {
            doc.pages[0]
                .layers
                .iter()
                .flat_map(|l| &l.shapes)
                .find_map(|s| match &s.kind {
                    ShapeKind::ClipFrame { contents, .. } => contents
                        .iter()
                        .find(|c| matches!(c.kind, ShapeKind::Rect { .. }))
                        .map(|c| {
                            let mut c = c.clone();
                            c.transform = s.transform * c.transform;
                            c.bounds()
                        }),
                    _ => None,
                })
                .expect("the rectangle")
        };
        let at_first = bounds(&print_sheets(&app, &st));
        let moved = PrintState {
            art_offset: (5.0, -3.0),
            ..st.clone()
        };
        let b = bounds(&print_sheets(&app, &moved));
        assert!(
            (b.x0 - at_first.x0 - 5.0).abs() < 1e-6,
            "{b:?} {at_first:?}"
        );
        assert!((b.y0 - at_first.y0 + 3.0).abs() < 1e-6);
    }

    /// The bitmap editor of the preferences gets the file as its argument;
    /// a macOS application bundle opens through `open -a` (#593).
    #[test]
    fn the_bitmap_editor_gets_the_file() {
        let p = Path::new("/tmp/bitmap-7.png");
        let (prog, args) = editor_command(" \"/usr/bin/gimp\" ", p);
        assert_eq!(prog, "/usr/bin/gimp");
        assert_eq!(args, vec!["/tmp/bitmap-7.png".to_string()]);
        let (prog, args) = editor_command("/Applications/GIMP.app", p);
        if cfg!(target_os = "macos") {
            assert_eq!(prog, "open");
            assert_eq!(args[0], "-a");
        } else {
            assert_eq!(prog, "/Applications/GIMP.app");
            assert_eq!(args.len(), 1);
        }
    }

    /// Fit to page shrinks objects running off the page to fit inside it,
    /// centred; objects wholly off the page stay out (#534).
    #[test]
    fn fit_to_page_shrinks_what_runs_off_the_page() {
        let mut app = App::headless();
        let size = app.page_size();
        let rect = |app: &mut App, r: Rect| {
            app.new_shape(ShapeKind::Rect {
                rect: r,
                radius: 0.0,
                corners: None,
            })
            .expect("rect")
        };
        // Twice the page's width, off its right edge; another wholly off.
        let wide = rect(
            &mut app,
            Rect::new(10.0, 10.0, 10.0 + size.width * 2.0, 60.0),
        );
        let away = rect(&mut app, Rect::new(-300.0, 0.0, -250.0, 50.0));
        let mut page = app.doc().pages[0].clone();
        fit_contents_to_page(&mut page, false);
        let bounds = |page: &tracedraw_core::document::Page, id| {
            page.layers
                .iter()
                .flat_map(|l| &l.shapes)
                .find(|s| s.id == id)
                .map(|s| s.bounds())
                .expect("shape")
        };
        let b = bounds(&page, wide);
        assert!(b.x0 >= -1e-6 && b.x1 <= size.width + 1e-6, "{b:?}");
        assert!((b.width() - size.width).abs() < 1e-6, "as wide as the page");
        assert!((b.center().y - size.height / 2.0).abs() < 1e-6, "centred");
        assert_eq!(bounds(&page, away), Rect::new(-300.0, 0.0, -250.0, 50.0));
        // Printing the selection fits it whatever it covers.
        let mut page = app.doc().pages[0].clone();
        fit_contents_to_page(&mut page, true);
        assert!(bounds(&page, away).x0 >= -1e-6);
        // Objects inside the page stay as they are.
        let mut page = app.doc().pages[0].clone();
        page.layers
            .iter_mut()
            .for_each(|l| l.shapes.retain(|s| s.id != wide && s.id != away));
        let before = page.clone();
        fit_contents_to_page(&mut page, false);
        assert_eq!(page, before);
    }
}

#[cfg(test)]
mod scratch_191 {
    use super::*;
    use tracedraw_core::{Color, Fill, Stroke, TextSpan};

    #[test]
    #[ignore]
    fn scratch_black_text_outputs() {
        tracedraw_text::install();
        let dir = std::path::PathBuf::from(
            "/tmp/claude-0/-home-claude/ecd2ad2c-4fd8-5343-ab00-e2a436bfbe5e/scratchpad/p191",
        );
        std::fs::create_dir_all(&dir).unwrap();
        let mut report = String::new();
        let colors: Vec<(&str, Color)> = vec![
            ("k100", Color::cmyk_pct(0.0, 0.0, 0.0, 100.0)),
            ("rich", Color::cmyk_pct(60.0, 40.0, 40.0, 100.0)),
            ("reg", Color::Registration),
            ("gray0", Color::Gray { v: 0.0 }),
            ("rgb0", Color::BLACK),
            (
                "lab0",
                Color::Lab {
                    l: 0.0,
                    a: 0.0,
                    b: 0.0,
                },
            ),
            (
                "spot",
                Color::Spot {
                    ink: 1,
                    tint: 1.0,
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 1.0,
                },
            ),
        ];
        for (name, color) in &colors {
            for mode in ["fill", "outline", "paragraph", "group"] {
                let mut app = App::headless();
                app.run(tracedraw_core::Command::SetInks {
                    inks: vec![tracedraw_core::document::Ink {
                        id: 1,
                        name: "Black".into(),
                        cmyk: [0.0, 0.0, 0.0, 1.0],
                    }],
                });
                let id = app
                    .new_shape(ShapeKind::Text {
                        spans: vec![TextSpan::new("WWWW", "Sans", 72.0)],
                        origin: tracedraw_core::geometry::Point::new(20.0, 100.0),
                        frame: (mode == "paragraph")
                            .then(|| tracedraw_core::geometry::Size::new(150.0, 60.0)),
                        align: Default::default(),
                        para: Default::default(),
                        on_path: None,
                    })
                    .expect("text");
                let outline = mode == "outline";
                app.run(tracedraw_core::Command::SetFill {
                    shapes: vec![id],
                    fill: if outline {
                        Fill::None
                    } else {
                        Fill::Solid(*color)
                    },
                });
                app.run(tracedraw_core::Command::SetStroke {
                    shapes: vec![id],
                    stroke: outline.then(|| Stroke::new(*color, 1.0)),
                });
                if mode == "group" {
                    let r = app
                        .new_shape(ShapeKind::Rect {
                            rect: tracedraw_core::geometry::Rect::new(5.0, 5.0, 10.0, 10.0),
                            radius: 0.0,
                            corners: None,
                        })
                        .unwrap();
                    app.select(vec![id, r]);
                    app.group_selection();
                }
                let tag = format!("{name}_{mode}");
                let variants: Vec<(&str, Box<dyn Fn(&mut PrintState)>)> = vec![
                    ("plain", Box::new(|_| {})),
                    ("opb", Box::new(|s| s.overprint_black = true)),
                    ("tb", Box::new(|s| s.text_black = true)),
                    ("s2c", Box::new(|s| s.spot_to_cmyk = true)),
                    ("bmp", Box::new(|s| s.as_bitmap = true)),
                    ("mirror", Box::new(|s| s.mirror = true)),
                    ("invert", Box::new(|s| s.invert = true)),
                    (
                        "marks",
                        Box::new(|s| {
                            s.crop_marks = true;
                            s.registration_marks = true;
                            s.color_bars = true;
                        }),
                    ),
                ];
                for (v, set) in &variants {
                    let mut st = PrintState::default();
                    set(&mut st);
                    let doc = print_sheets(&app, &st);
                    let pdf = tracedraw_io::pdf::document_to_pdf(&doc);
                    std::fs::write(dir.join(format!("print_{tag}_{v}.pdf")), pdf).unwrap();
                }
                let st = PrintState {
                    separations: true,
                    ..PrintState::default()
                };
                let doc = separations_doc(&print_sheets(&app, &st));
                std::fs::write(
                    dir.join(format!("print_{tag}_seps.pdf")),
                    tracedraw_io::pdf::document_to_pdf(&doc),
                )
                .unwrap();
                for (k, std_) in [
                    tracedraw_io::pdf::PdfStandard::None,
                    tracedraw_io::pdf::PdfStandard::X1a,
                    tracedraw_io::pdf::PdfStandard::X3,
                    tracedraw_io::pdf::PdfStandard::X4,
                ]
                .into_iter()
                .enumerate()
                {
                    let opts = tracedraw_io::pdf::PdfOptions {
                        standard: std_,
                        output_condition: "FOGRA39".into(),
                        flatten_dpi: 150.0,
                        ..Default::default()
                    };
                    let pdf = tracedraw_io::pdf::document_to_pdf_with(app.doc(), &opts);
                    std::fs::write(dir.join(format!("publish_{tag}_x{k}.pdf")), pdf).unwrap();
                }
                // Through `.cdr` and back.
                for ver in [14u16, 17, 20, 24, 27] {
                    let bytes = tracedraw_cdr::write::document_to_cdr_version(app.doc(), ver);
                    let Ok((doc, _)) = tracedraw_cdr::open_bytes(&bytes, "t") else {
                        report.push_str(&format!("{tag} v{ver}: unreadable\n"));
                        continue;
                    };
                    let mut texts = Vec::new();
                    fn walk(s: &tracedraw_core::Shape, out: &mut Vec<String>) {
                        match &s.kind {
                            ShapeKind::Text { spans, .. } => out.push(format!(
                                "fill {:?} stroke {:?} span_fills {:?}",
                                s.fill,
                                s.stroke.as_ref().map(|k| k.color),
                                spans.iter().map(|sp| sp.fill.clone()).collect::<Vec<_>>()
                            )),
                            ShapeKind::Group { children } => {
                                children.iter().for_each(|c| walk(c, out))
                            }
                            _ => {}
                        }
                    }
                    for s in doc
                        .pages
                        .iter()
                        .flat_map(|p| &p.layers)
                        .flat_map(|l| &l.shapes)
                    {
                        walk(s, &mut texts);
                    }
                    report.push_str(&format!("{tag} v{ver}: {texts:?}\n"));
                    let mut back = App::headless();
                    back.open_document(doc, None);
                    let pdf = tracedraw_io::pdf::document_to_pdf(&print_sheets(
                        &back,
                        &PrintState::default(),
                    ));
                    std::fs::write(dir.join(format!("cdr{ver}_{tag}.pdf")), pdf).unwrap();
                }
            }
        }
        std::fs::write(dir.join("report.txt"), report).unwrap();
    }
}
