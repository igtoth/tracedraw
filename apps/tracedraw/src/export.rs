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
fn export_document(app: &App, st: &ExportState) -> (Document, Rect) {
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
            c.transform = Affine::translate((-bounds.x0, -bounds.y0)) * c.transform;
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
            std::fs::write(path, svg).map_err(|e| e.to_string())?;
        }
        "pdf" | "ai" => {
            let pdf = tracedraw_io::pdf::document_to_pdf(&doc);
            std::fs::write(path, pdf).map_err(|e| e.to_string())?;
        }
        "eps" => {
            let eps = tracedraw_io::eps::page_to_eps(&doc, 0);
            std::fs::write(path, eps).map_err(|e| e.to_string())?;
        }
        _ => {
            // Raster: every page when all_pages, numbered files.
            let pages: Vec<_> = doc.pages.iter().map(|p| p.id).collect();
            for (i, pid) in pages.iter().enumerate() {
                let mut pm = tracedraw_render::render_page_image(&doc, *pid, st.dpi)
                    .ok_or("render failed")?;
                if st.transparent && matches!(ext, "png" | "webp" | "gif") {
                    // Re-render on a transparent background.
                    let p = doc.page(*pid).map_err(|e| e.to_string())?;
                    let zoom = st.dpi / 25.4;
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
                write_raster(&pm, &target, ext, st.quality)?;
            }
        }
    }
    Ok(trf(
        "status.exported",
        &[("path", &path.display().to_string())],
    ))
}

fn write_raster(pm: &tiny_skia::Pixmap, path: &Path, ext: &str, quality: u8) -> Result<(), String> {
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
        _ => dynimg
            .write_to(&mut out, format)
            .map_err(|e| e.to_string())?,
    }
    std::fs::write(path, out.into_inner()).map_err(|e| e.to_string())
}

/// Print: build a PDF with the print options and open it in the system
/// viewer, which owns the printer dialog.
pub fn print(app: &mut App, st: &PrintState) -> Result<String, String> {
    let mut doc = app.engine.document().clone();
    if !st.all_pages {
        doc.pages.retain(|p| p.id == app.page);
    }
    let bleed = if st.bleed {
        doc.metadata.bleed.max(0.0)
    } else {
        0.0
    };
    let marks = st.crop_marks || st.registration_marks;
    let margin = if marks { 8.0 } else { 0.0 } + bleed;
    // Scale and place every page's content into a (possibly larger) sheet.
    for page in doc.pages.iter_mut() {
        let scale = if st.fit_to_page {
            1.0
        } else {
            st.scale / 100.0
        };
        let base = page.size;
        let sheet = Size::new(
            base.width * scale + 2.0 * margin,
            base.height * scale + 2.0 * margin,
        );
        let mut t = Affine::translate((margin, margin)) * Affine::scale(scale);
        if st.mirror {
            t = Affine::translate((sheet.width, 0.0))
                * Affine::new([-1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
                * t;
        }
        for layer in page.layers.iter_mut() {
            for s in layer.shapes.iter_mut() {
                s.transform = t * s.transform;
                if st.invert {
                    invert_colors(s);
                }
            }
        }
        if marks {
            let id = tracedraw_core::ShapeId(0);
            let mut marks_shape = tracedraw_core::Shape::new(
                id,
                ShapeKind::Path {
                    path: crop_marks(sheet, margin, bleed, st.registration_marks),
                    closed: false,
                },
            );
            marks_shape.fill = tracedraw_core::Fill::None;
            marks_shape.stroke = Some(tracedraw_core::Stroke::new(
                tracedraw_core::Color::Registration,
                0.1,
            ));
            if let Some(l) = page.layers.last_mut() {
                l.shapes.push(marks_shape);
            }
        }
        page.size = sheet;
    }
    let pdf = if st.separations {
        separations_pdf(&doc)
    } else {
        tracedraw_io::pdf::document_to_pdf(&doc)
    };
    let dir = std::env::temp_dir().join("tracedraw-print");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!(
        "{}.pdf",
        app.document_title().trim_end_matches('*')
    ));
    std::fs::write(&path, pdf).map_err(|e| e.to_string())?;
    // The system viewer handles the copy count; open it once.
    open_with_system(&path)?;
    Ok(trf(
        "status.print_sent",
        &[("path", &path.display().to_string())],
    ))
}

fn invert_colors(s: &mut tracedraw_core::Shape) {
    use tracedraw_core::{Color, Fill};
    let inv = |c: Color| {
        let [r, g, b] = c.to_rgb8();
        Color::rgb8(255 - r, 255 - g, 255 - b)
    };
    if let Fill::Solid(c) = s.fill {
        s.fill = Fill::Solid(inv(c));
    }
    if let Some(st) = &mut s.stroke {
        st.color = inv(st.color);
    }
    if let ShapeKind::Group { children } = &mut s.kind {
        for c in children {
            invert_colors(c);
        }
    }
}

fn crop_marks(sheet: Size, margin: f64, bleed: f64, registration: bool) -> tracedraw_core::BezPath {
    let mut p = tracedraw_core::BezPath::new();
    let len = 5.0;
    let gap = bleed + 1.0;
    let (x0, y0, x1, y1) = (margin, margin, sheet.width - margin, sheet.height - margin);
    for (x, y, dx, dy) in [
        (x0, y0, -1.0, -1.0),
        (x1, y0, 1.0, -1.0),
        (x1, y1, 1.0, 1.0),
        (x0, y1, -1.0, 1.0),
    ] {
        p.move_to((x + dx * gap, y));
        p.line_to((x + dx * (gap + len), y));
        p.move_to((x, y + dy * gap));
        p.line_to((x, y + dy * (gap + len)));
    }
    if registration {
        let r = 2.0;
        for (cx, cy) in [
            (sheet.width / 2.0, margin / 2.0),
            (sheet.width / 2.0, sheet.height - margin / 2.0),
            (margin / 2.0, sheet.height / 2.0),
            (sheet.width - margin / 2.0, sheet.height / 2.0),
        ] {
            p.extend(tracedraw_core::geometry::ellipse_path(Rect::new(
                cx - r,
                cy - r,
                cx + r,
                cy + r,
            )));
            p.move_to((cx - r * 1.5, cy));
            p.line_to((cx + r * 1.5, cy));
            p.move_to((cx, cy - r * 1.5));
            p.line_to((cx, cy + r * 1.5));
        }
    }
    p
}

/// Colour separations: one grayscale page per ink (C, M, Y, K) per page,
/// where the ink coverage of every colour is drawn as a gray value.
fn separations_pdf(doc: &Document) -> Vec<u8> {
    use tracedraw_core::{Color, Fill};
    let mut sep = Document::new(format!("{} (separations)", doc.title), doc.pages[0].size);
    sep.pages.clear();
    sep.symbols = doc.symbols.clone();
    for page in &doc.pages {
        for (ink, name) in [
            (0usize, "Cyan"),
            (1, "Magenta"),
            (2, "Yellow"),
            (3, "Black"),
        ] {
            let mut p = page.clone();
            p.name = format!("{} {}", page.name, name);
            let mut ids = sep.ids().clone();
            p.id = ids.page();
            sep.set_ids(ids);
            for layer in p.layers.iter_mut() {
                for s in layer.shapes.iter_mut() {
                    let coverage = |c: Color| -> Color {
                        let cm = c.convert_to("CMYK");
                        let v = match cm {
                            Color::Cmyk { c, m, y, k } => [c, m, y, k][ink],
                            _ => 0.0,
                        };
                        Color::Gray { v: 1.0 - v }
                    };
                    if let Some(c) = s.fill.preview_color() {
                        if !matches!(s.fill, Fill::None) {
                            s.fill = Fill::Solid(coverage(c));
                        }
                    }
                    if let Some(st) = &mut s.stroke {
                        st.color = coverage(st.color);
                    }
                }
            }
            sep.pages.push(p);
        }
    }
    tracedraw_io::pdf::document_to_pdf(&sep)
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

impl App {
    /// Open a web page in the system browser.
    pub fn open_url(&mut self, url: &str) {
        if let Err(e) = open_with_system(Path::new(url)) {
            self.status = e;
        }
    }
}

// ----- print merge ---------------------------------------------------------------

pub fn parse_csv(text: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let delim = if text
        .lines()
        .next()
        .map(|l| l.matches(';').count() > l.matches(',').count())
        .unwrap_or(false)
    {
        b';'
    } else {
        b','
    };
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

/// Duplicate the current page once per data row, substituting `<Field>`
/// placeholders in every text object.
pub fn perform_merge(app: &mut App, headers: &[String], rows: &[Vec<String>]) {
    let page = app.page;
    let template = match app.doc().page(page) {
        Ok(p) => p.clone(),
        Err(_) => return,
    };
    let mut cmds = Vec::new();
    for _ in rows {
        cmds.push(Command::DuplicatePage { page });
    }
    if let Err(e) = app.engine.run_batch("Print Merge", &cmds) {
        app.status = e.to_string();
        return;
    }
    // The duplicates were inserted right after the template page, in order:
    // DuplicatePage inserts at idx + 1 each time, so the last row is nearest.
    let idx = app
        .doc()
        .pages
        .iter()
        .position(|p| p.id == page)
        .unwrap_or(0);
    let new_pages: Vec<tracedraw_core::PageId> = app.doc().pages[idx + 1..idx + 1 + rows.len()]
        .iter()
        .map(|p| p.id)
        .collect();
    let mut cmds = Vec::new();
    for (k, pid) in new_pages.iter().rev().enumerate() {
        let Some(row) = rows.get(k) else { continue };
        let Ok(p) = app.doc().page(*pid) else {
            continue;
        };
        let shapes: Vec<tracedraw_core::Shape> =
            p.layers.iter().flat_map(|l| l.shapes.clone()).collect();
        for s in shapes {
            if let ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } = &s.kind
            {
                let mut changed = false;
                let new_spans: Vec<TextSpan> = spans
                    .iter()
                    .map(|sp| {
                        let mut t = sp.text.clone();
                        for (h, v) in headers.iter().zip(row) {
                            let ph = format!("<{h}>");
                            if t.contains(&ph) {
                                t = t.replace(&ph, v);
                                changed = true;
                            }
                        }
                        TextSpan {
                            text: t,
                            ..sp.clone()
                        }
                    })
                    .collect();
                if changed {
                    cmds.push(Command::SetShapeKind {
                        shape: s.id,
                        kind: ShapeKind::Text {
                            spans: new_spans,
                            origin: *origin,
                            frame: *frame,
                            align: *align,
                            para: para.clone(),
                            on_path: on_path.clone(),
                        },
                    });
                }
            }
        }
        cmds.push(Command::RenamePage {
            page: *pid,
            name: format!("{} {}", template.name, k + 1),
        });
    }
    if let Err(e) = app.engine.run_batch("Print Merge", &cmds) {
        app.status = e.to_string();
    }
    app.status = trf("status.merge_done", &[("n", &rows.len().to_string())]);
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
    use super::*;

    #[test]
    fn csv_parses_headers_and_rows() {
        let (h, r) = parse_csv("Name,City\nAna,Porto\nBob,Lisboa\n");
        assert_eq!(h, vec!["Name", "City"]);
        assert_eq!(r.len(), 2);
        assert_eq!(r[1][1], "Lisboa");
        let (h2, _) = parse_csv("a;b\n1;2\n");
        assert_eq!(h2, vec!["a", "b"]);
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
}
