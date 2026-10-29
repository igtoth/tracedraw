//! Packaging a drawing (#852): File > Package and Save As's Package
//! choice put the drawing, the pictures it links to and the font files
//! its text uses into one folder, with a short report, and optionally a
//! PDF. Linked pictures go into `Pictures/` and the copied drawing's links
//! point to the copies (as paths relative to the drawing, so the folder
//! can move); fonts go into `Fonts/` when their licence allows them to go
//! with a document (the OS/2 embedding bits), the others are listed in the
//! report. A drawing opened from a folder with a `Fonts` folder beside it
//! gets those fonts for the session. Desktop only: a browser has no
//! folders. See docs/behavior/package.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use std::path::{Path, PathBuf};
use tracedraw_core::document::ShapeKind;
use tracedraw_core::{Command, Document, Shape, ShapeId};

/// The folder of the copied pictures, inside the package.
pub const PICTURES: &str = "Pictures";
/// The folder of the copied font files, inside the package.
pub const FONTS: &str = "Fonts";
/// The key of a bitmap's linked file in its object data.
pub const LINK_KEY: &str = "bitmap.link";

/// What a package holds, and what could not go in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub folder: PathBuf,
    pub drawing: PathBuf,
    pub pdf: Option<PathBuf>,
    /// The report's own file.
    pub report: PathBuf,
    /// Linked pictures copied: the link as the drawing had it, and the
    /// copy's path relative to the package.
    pub pictures: Vec<(String, String)>,
    /// Links whose file was not found (the link is kept).
    pub missing_pictures: Vec<String>,
    /// Fonts copied: family, file name in `Fonts/`.
    pub fonts: Vec<(String, String)>,
    /// Fonts whose licence does not allow them to go along: family, the
    /// installed file.
    pub refused_fonts: Vec<(String, String)>,
    /// Families not installed and not carried by the drawing, with the
    /// installed font drawing their text.
    pub missing_fonts: Vec<(String, Option<String>)>,
}

/// Every object of `doc` (on every layer, in groups, clip frames and
/// symbols) passed to `f`.
fn each_shape(doc: &Document, f: &mut impl FnMut(&Shape)) {
    fn walk(s: &Shape, f: &mut impl FnMut(&Shape)) {
        f(s);
        match &s.kind {
            ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, f)),
            ShapeKind::ClipFrame { frame, contents } => {
                walk(frame, f);
                contents.iter().for_each(|c| walk(c, f));
            }
            _ => {}
        }
    }
    for l in doc.all_layers() {
        l.shapes.iter().for_each(|s| walk(s, f));
    }
    doc.symbols.iter().for_each(|s| walk(&s.shape, f));
}

/// [`each_shape`], changing them.
fn each_shape_mut(doc: &mut Document, f: &mut impl FnMut(&mut Shape)) {
    fn walk(s: &mut Shape, f: &mut impl FnMut(&mut Shape)) {
        f(s);
        match &mut s.kind {
            ShapeKind::Group { children } => children.iter_mut().for_each(|c| walk(c, f)),
            ShapeKind::ClipFrame { frame, contents } => {
                walk(frame, f);
                contents.iter_mut().for_each(|c| walk(c, f));
            }
            _ => {}
        }
    }
    for l in doc.all_layers_mut() {
        l.shapes.iter_mut().for_each(|s| walk(s, f));
    }
    doc.symbols.iter_mut().for_each(|s| walk(&mut s.shape, f));
}

fn link_of(s: &Shape) -> Option<&str> {
    s.data
        .iter()
        .find(|(k, _)| k == LINK_KEY)
        .map(|(_, v)| v.as_str())
}

/// The files the drawing's bitmaps link to, each once, in drawing order.
pub fn picture_links(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    each_shape(doc, &mut |s| {
        if let Some(l) = link_of(s) {
            if !l.trim().is_empty() && !out.iter().any(|o| o == l) {
                out.push(l.to_string());
            }
        }
    });
    out
}

/// A link's file: a relative link (a package's) is relative to the
/// folder of the drawing's file, `base`.
pub fn resolve_link(link: &str, base: Option<&Path>) -> PathBuf {
    let p = PathBuf::from(link);
    match base {
        Some(dir) if p.is_relative() => dir.join(p),
        _ => p,
    }
}

/// A name in `taken` for `name`: itself, else "stem-2.ext", "stem-3.ext".
fn unique_name(name: &str, taken: &[String]) -> String {
    let free = |n: &str| !taken.iter().any(|t| t.eq_ignore_ascii_case(n));
    if free(name) {
        return name.to_string();
    }
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|i| format!("{stem}-{i}{ext}"))
        .find(|n| free(n))
        .unwrap_or_else(|| name.to_string())
}

/// Copy the linked pictures of `doc` into `folder/Pictures`: the links
/// copied (link, copy relative to the folder) and those whose file was
/// not found. `base` is the folder relative links are read from.
pub fn copy_pictures(
    doc: &Document,
    base: Option<&Path>,
    folder: &Path,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut copied = Vec::new();
    let mut missing = Vec::new();
    let mut taken: Vec<String> = Vec::new();
    for link in picture_links(doc) {
        let src = resolve_link(&link, base);
        let Some(name) = src.file_name().map(|n| n.to_string_lossy().to_string()) else {
            missing.push(link);
            continue;
        };
        if !crate::files::exists(&src) {
            missing.push(link);
            continue;
        }
        let name = unique_name(&name, &taken);
        let target = folder.join(PICTURES).join(&name);
        // A picture already where the package puts it (packaging the
        // package again) is not copied onto itself.
        let same = crate::files::same_path(
            &crate::files::canonical(&src),
            &crate::files::canonical(&target),
        );
        if !same && crate::files::copy(&src, &target).is_err() {
            missing.push(link);
            continue;
        }
        taken.push(name.clone());
        copied.push((link, format!("{PICTURES}/{name}")));
    }
    (copied, missing)
}

/// The object data of the objects whose link `copied` moves, with the
/// link pointing to the copy.
pub fn relinked_data(
    doc: &Document,
    copied: &[(String, String)],
) -> Vec<(ShapeId, Vec<(String, String)>)> {
    let mut out = Vec::new();
    each_shape(doc, &mut |s| {
        let Some(link) = link_of(s) else { return };
        let Some((_, to)) = copied.iter().find(|(from, _)| from == link) else {
            return;
        };
        let data = s
            .data
            .iter()
            .map(|(k, v)| {
                if k == LINK_KEY {
                    (k.clone(), to.clone())
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect();
        out.push((s.id, data));
    });
    out
}

/// Point the links of `doc` to the copies.
pub fn relink(doc: &mut Document, copied: &[(String, String)]) {
    each_shape_mut(doc, &mut |s| {
        for (k, v) in s.data.iter_mut() {
            if k == LINK_KEY {
                if let Some((_, to)) = copied.iter().find(|(from, _)| from == v) {
                    *v = to.clone();
                }
            }
        }
    });
}

/// The extension a font file's data calls for.
fn font_extension(data: &[u8]) -> &'static str {
    match data.get(0..4) {
        Some(b"OTTO") => "otf",
        Some(b"ttcf") => "ttc",
        _ => "ttf",
    }
}

/// A file name made of a family name (letters, digits, spaces, dashes).
fn family_file_stem(family: &str) -> String {
    let s: String = family
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() {
        "font".into()
    } else {
        s
    }
}

/// Copy the font files the text of `doc` uses into `folder/Fonts`: the
/// fonts the drawing carries, else the installed file of each face, when
/// its licence allows it to go with a document. Returns (copied,
/// refused, missing) as the [`Report`] has them.
#[allow(clippy::type_complexity)]
pub fn copy_fonts(
    doc: &Document,
    folder: &Path,
) -> (
    Vec<(String, String)>,
    Vec<(String, String)>,
    Vec<(String, Option<String>)>,
) {
    let fonts = tracedraw_text::fonts();
    let mut copied: Vec<(String, String)> = Vec::new();
    let mut refused: Vec<(String, String)> = Vec::new();
    let mut missing: Vec<(String, Option<String>)> = Vec::new();
    let mut sources: Vec<PathBuf> = Vec::new();
    let mut taken: Vec<String> = Vec::new();
    let dir = folder.join(FONTS);
    for (family, bold, italic) in doc.font_faces_used() {
        // A font the drawing carries: its data.
        let carried = doc
            .fonts
            .iter()
            .find(|f| f.family.eq_ignore_ascii_case(&family));
        let (data, name, source) = if let Some(f) = carried {
            let name = format!("{}.{}", family_file_stem(&family), font_extension(&f.data));
            (f.data.as_ref().clone(), name, None)
        } else if let Some(path) = fonts.face_path(&family, bold, italic) {
            if sources.contains(&path) {
                continue;
            }
            sources.push(path.clone());
            let Ok(data) = crate::files::read(&path) else {
                continue;
            };
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    format!("{}.{}", family_file_stem(&family), font_extension(&data))
                });
            (data, name, Some(path))
        } else {
            if !fonts.has_family(&family) && !missing.iter().any(|(f, _)| *f == family) {
                missing.push((family.clone(), fonts.substitute(&family)));
            }
            continue;
        };
        if copied.iter().any(|(f, n)| *f == family && *n == name) {
            continue;
        }
        if !tracedraw_text::usable_in_documents(&data) {
            let file = source
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| name.clone());
            if !refused.iter().any(|(f, _)| *f == family) {
                refused.push((family.clone(), file));
            }
            continue;
        }
        if source.is_none() && copied.iter().any(|(f, _)| *f == family) {
            continue;
        }
        let name = unique_name(&name, &taken);
        if crate::files::write_creating(dir.join(&name), &data).is_ok() {
            taken.push(name.clone());
            copied.push((family.clone(), name));
        }
    }
    (copied, refused, missing)
}

/// The report's text, in the interface language.
pub fn report_text(r: &Report, name: &str, when: &str) -> String {
    let mut out = String::new();
    let line = |out: &mut String, s: String| {
        out.push_str(&s);
        out.push('\n');
    };
    let file_name = |p: &Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    };
    line(&mut out, trf("package.report_title", &[("name", name)]));
    line(
        &mut out,
        trf(
            "package.report_made",
            &[("version", env!("CARGO_PKG_VERSION")), ("when", when)],
        ),
    );
    line(&mut out, String::new());
    line(
        &mut out,
        trf(
            "package.report_drawing",
            &[("file", &file_name(&r.drawing))],
        ),
    );
    if let Some(pdf) = &r.pdf {
        line(
            &mut out,
            trf("package.report_pdf", &[("file", &file_name(pdf))]),
        );
    }
    line(&mut out, String::new());
    line(
        &mut out,
        trf(
            "package.report_pictures",
            &[("n", &r.pictures.len().to_string())],
        ),
    );
    for (from, to) in &r.pictures {
        line(
            &mut out,
            format!(
                "  {}",
                trf("package.report_picture", &[("from", from), ("to", to)])
            ),
        );
    }
    for from in &r.missing_pictures {
        line(
            &mut out,
            format!(
                "  {}",
                trf("package.report_picture_missing", &[("from", from)])
            ),
        );
    }
    if r.pictures.is_empty() && r.missing_pictures.is_empty() {
        line(&mut out, format!("  {}", tr("package.report_none")));
    }
    line(&mut out, String::new());
    line(
        &mut out,
        trf("package.report_fonts", &[("n", &r.fonts.len().to_string())]),
    );
    for (family, file) in &r.fonts {
        line(
            &mut out,
            format!(
                "  {}",
                trf(
                    "package.report_font",
                    &[("family", family), ("file", &format!("{FONTS}/{file}"))]
                )
            ),
        );
    }
    for (family, file) in &r.refused_fonts {
        line(
            &mut out,
            format!(
                "  {}",
                trf(
                    "package.report_font_refused",
                    &[("family", family), ("file", file)]
                )
            ),
        );
    }
    for (family, sub) in &r.missing_fonts {
        let text = match sub {
            Some(s) => trf(
                "package.report_font_missing",
                &[("family", family), ("sub", s)],
            ),
            None => trf("package.report_font_missing_none", &[("family", family)]),
        };
        line(&mut out, format!("  {text}"));
    }
    if r.fonts.is_empty() && r.refused_fonts.is_empty() && r.missing_fonts.is_empty() {
        line(&mut out, format!("  {}", tr("package.report_none")));
    }
    out
}

/// Make `folder` and its parents (desktop).
fn make_folder(folder: &Path) -> Result<(), String> {
    if crate::files::is_dir(folder) {
        return Ok(());
    }
    crate::files::create_dir_all(folder).map_err(|e| e.to_string())
}

/// The PDF of `doc` with the Publish to PDF choices made last.
fn write_pdf(app: &App, doc: &Document, path: &Path, title: &str) -> Result<(), String> {
    let prefs = &app.settings.publish_pdf;
    let opts = tracedraw_io::pdf::PdfOptions {
        title: Some(title.to_string()),
        bleed_mm: doc.metadata.bleed.max(0.0),
        text: prefs.text(),
        images: prefs.bitmaps.options(),
        bookmarks: prefs.bookmarks,
        ..Default::default()
    };
    let (pdf, _) = tracedraw_io::pdf::document_to_pdf_report(doc, &opts);
    crate::files::write(path, pdf).map_err(|e| e.to_string())
}

/// The fonts, PDF and report of a package whose drawing (`doc`, written
/// to `drawing`) and pictures are in place.
fn finish(
    app: &App,
    doc: &Document,
    drawing: &Path,
    pictures: (Vec<(String, String)>, Vec<String>),
    pdf: bool,
) -> Result<Report, String> {
    let folder = drawing.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = drawing
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "drawing".into());
    let (fonts, refused_fonts, missing_fonts) = copy_fonts(doc, &folder);
    let pdf_path = if pdf {
        let p = folder.join(format!("{stem}.pdf"));
        write_pdf(app, doc, &p, &stem)?;
        Some(p)
    } else {
        None
    };
    let report_path = folder.join(trf("package.report_file", &[("name", &stem)]));
    let r = Report {
        folder,
        drawing: drawing.to_path_buf(),
        pdf: pdf_path,
        report: report_path,
        pictures: pictures.0,
        missing_pictures: pictures.1,
        fonts,
        refused_fonts,
        missing_fonts,
    };
    let tz = crate::time_tracker::local_zone();
    let when = crate::time_tracker::date_time(crate::time_tracker::unix_now(), &tz);
    crate::files::write(&r.report, report_text(&r, &stem, &when)).map_err(|e| e.to_string())?;
    Ok(r)
}

/// The status line after packaging.
pub fn status(r: &Report) -> String {
    let mut msg = trf(
        "status.packaged",
        &[
            ("p", &r.folder.display().to_string()),
            ("pictures", &r.pictures.len().to_string()),
            ("fonts", &r.fonts.len().to_string()),
        ],
    );
    let left = r.refused_fonts.len() + r.missing_fonts.len() + r.missing_pictures.len();
    if left > 0 {
        msg.push(' ');
        msg.push_str(&trf("status.packaged_left", &[("n", &left.to_string())]));
    }
    msg
}

impl App {
    /// The folder relative links are read from: the drawing file's.
    pub fn link_base(&self) -> Option<PathBuf> {
        self.file
            .as_ref()
            .and_then(|f| f.parent())
            .map(Path::to_path_buf)
    }

    /// File > Package: a copy of the drawing named `name` (a `.tdraw`
    /// file) in `folder`, its pictures and fonts beside it, the report and
    /// optionally a PDF. The open drawing is not changed.
    pub fn package_copy(&self, folder: &Path, name: &str, pdf: bool) -> Result<Report, String> {
        if crate::files::WEB {
            return Err(tr("package.desktop_only"));
        }
        make_folder(folder)?;
        let fonts = tracedraw_text::fonts();
        let mut doc = fonts.with_carried_fonts(self.doc()).into_owned();
        let base = self.link_base();
        let pictures = copy_pictures(&doc, base.as_deref(), folder);
        relink(&mut doc, &pictures.0);
        let drawing = folder.join(format!("{name}.tdraw"));
        crate::files::write_with(&drawing, |w| {
            doc.write_json_version(w, tracedraw_core::document::TDRAW_FORMAT_VERSION)
                .map_err(std::io::Error::other)
        })
        .map_err(|e| e.to_string())?;
        finish(self, &doc, &drawing, pictures, pdf)
    }

    /// Save As with Package: the drawing saved as `path` in its package
    /// folder (made when needed), its linked pictures copied beside it
    /// and its links pointing to the copies (one undo step each), then
    /// its fonts, the report and optionally a PDF. The status line says
    /// what went in.
    pub fn save_package(&mut self, path: PathBuf, pdf: bool) {
        let Some(folder) = path.parent().map(Path::to_path_buf) else {
            return;
        };
        if let Err(e) = make_folder(&folder) {
            self.status = trf("status.package_failed", &[("e", &e)]);
            return;
        }
        let base = self.link_base();
        let pictures = copy_pictures(self.doc(), base.as_deref(), &folder);
        for (shape, data) in relinked_data(self.doc(), &pictures.0) {
            self.run(Command::SetObjectData { shape, data });
        }
        self.save_to(path.clone());
        if self.file.as_ref() != Some(&path) {
            // The save failed; its message stays.
            return;
        }
        let doc = self.doc().clone();
        self.status = match finish(self, &doc, &path, pictures, pdf) {
            Ok(r) => status(&r),
            Err(e) => trf("status.package_failed", &[("e", &e)]),
        };
    }

    /// The fonts in a `Fonts` folder beside a drawing being opened (a
    /// package's) are usable for the session, before its text is laid
    /// out. The families added.
    pub fn load_package_fonts(&mut self, drawing: &Path) -> Vec<String> {
        if crate::files::WEB {
            return Vec::new();
        }
        let Some(dir) = drawing.parent().map(|d| d.join(FONTS)) else {
            return Vec::new();
        };
        if !crate::files::is_dir(&dir) {
            return Vec::new();
        }
        let files = tracedraw_text::catalog::font_files(&dir);
        if files.is_empty() {
            return Vec::new();
        }
        let fonts = tracedraw_text::fonts();
        let added = fonts.install_paths(&files);
        if !added.is_empty() {
            log::info!(
                "fonts of the package {}: {}",
                dir.display(),
                added.join(", ")
            );
            self.font_families = fonts.available_families();
        }
        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Point, Rect};

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tracedraw-pkg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("temp folder");
        d
    }

    fn png(path: &Path, rgb: [u8; 3]) {
        image::RgbaImage::from_pixel(2, 2, image::Rgba([rgb[0], rgb[1], rgb[2], 255]))
            .save(path)
            .expect("png written");
    }

    /// A drawing with two pictures linked to files (one in a group, two
    /// objects on the same file), one linked to a file that is gone, and
    /// a text in an installed font when there is one.
    fn drawing(dir: &Path) -> (App, Option<&'static str>) {
        tracedraw_text::install();
        let mut app = App::headless();
        let src = dir.join("src");
        std::fs::create_dir_all(src.join("b")).expect("folder");
        let a = src.join("logo.png");
        let b = src.join("b").join("logo.png");
        png(&a, [255, 0, 0]);
        png(&b, [0, 0, 255]);
        let bitmap = |app: &mut App, link: &Path| {
            let id = app
                .new_shape(ShapeKind::Bitmap {
                    rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                    width_px: 2,
                    height_px: 2,
                    png: Vec::new().into(),
                    fx: None,
                })
                .expect("bitmap");
            app.run(Command::SetObjectData {
                shape: id,
                data: vec![(LINK_KEY.into(), link.display().to_string())],
            });
            id
        };
        let one = bitmap(&mut app, &a);
        let two = bitmap(&mut app, &b);
        bitmap(&mut app, &a);
        bitmap(&mut app, &src.join("gone.png"));
        app.select(vec![one, two]);
        app.group_selection();
        let fonts = tracedraw_text::fonts();
        let family = ["DejaVu Sans", "Liberation Sans", "Noto Sans"]
            .into_iter()
            .find(|f| fonts.has_family(f) && fonts.face_path(f, false, false).is_some());
        app.new_shape(ShapeKind::Text {
            spans: vec![
                tracedraw_core::TextSpan::new("Hello", family.unwrap_or("DejaVu Sans"), 12.0),
                tracedraw_core::TextSpan::new(" there", "Nowhere Grotesk 9", 12.0),
            ],
            origin: Point::new(10.0, 20.0),
            frame: None,
            align: tracedraw_core::TextAlign::Left,
            para: Default::default(),
            on_path: None,
        })
        .expect("text");
        (app, family)
    }

    fn links(doc: &Document) -> Vec<String> {
        let mut out = Vec::new();
        each_shape(doc, &mut |s| {
            if let Some(l) = link_of(s) {
                out.push(l.to_string());
            }
        });
        out
    }

    /// File > Package: the drawing, its pictures (two files of one name
    /// kept apart), its installed fonts and a report go into the folder;
    /// the copy's links point to the copies; the open drawing is not
    /// changed; a missing picture and a missing font are reported.
    #[test]
    fn a_package_holds_the_drawing_its_pictures_fonts_and_a_report() {
        let dir = temp("copy");
        let (app, family) = drawing(&dir);
        let before = links(app.doc());
        let folder = dir.join("out").join("Poster");
        let r = app.package_copy(&folder, "Poster", true).expect("packaged");
        assert_eq!(links(app.doc()), before, "the open drawing is unchanged");
        assert_eq!(
            r.pictures
                .iter()
                .map(|(_, to)| to.as_str())
                .collect::<Vec<_>>(),
            vec!["Pictures/logo.png", "Pictures/logo-2.png"]
        );
        assert_eq!(r.missing_pictures.len(), 1);
        assert!(folder.join("Pictures/logo.png").is_file());
        assert!(folder.join("Pictures/logo-2.png").is_file());
        // The copy opens with its links on the copies, relative to it.
        let text = std::fs::read_to_string(folder.join("Poster.tdraw")).expect("drawing");
        let copy = Document::from_json(&text).expect("reads");
        let mut got = links(&copy);
        got.sort();
        assert_eq!(got.len(), 4);
        assert_eq!(got.iter().filter(|l| *l == "Pictures/logo.png").count(), 2);
        assert!(got.contains(&"Pictures/logo-2.png".to_string()));
        // Read back from the package, relative links find the copies.
        assert_eq!(
            resolve_link("Pictures/logo.png", Some(&folder)),
            folder.join("Pictures/logo.png")
        );
        // The PDF and the report.
        assert!(std::fs::read(folder.join("Poster.pdf"))
            .expect("pdf")
            .starts_with(b"%PDF"));
        let report = std::fs::read_to_string(&r.report).expect("report");
        assert!(report.contains("Poster.tdraw") && report.contains("gone.png"));
        assert!(report.contains("Nowhere Grotesk 9"), "{report}");
        assert!(r
            .missing_fonts
            .iter()
            .any(|(f, _)| f == "Nowhere Grotesk 9"));
        if let Some(family) = family {
            assert!(r.fonts.iter().any(|(f, _)| f == family), "{:?}", r.fonts);
            let (_, file) = r.fonts.iter().find(|(f, _)| f == family).expect("copied");
            assert!(folder.join(FONTS).join(file).is_file());
            assert!(report.contains(family));
        }
        // Opened again, the fonts of its Fonts folder are loaded first.
        let font = folder.join(FONTS).join("Selawik-Bold-copy.ttf");
        std::fs::write(&font, include_bytes!("../assets/fonts/Selawik-Bold.ttf"))
            .expect("font written");
        // Packaging the package again copies nothing onto itself.
        let mut again = App::headless();
        again.open_path(folder.join("Poster.tdraw"));
        let r2 = again
            .package_copy(&folder, "Poster", false)
            .expect("packaged again");
        assert_eq!(r2.pictures.len(), 2);
        assert!(tracedraw_text::fonts().installed_paths().contains(&font));
        assert!(folder.join("Pictures/logo.png").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A font whose licence forbids it to go with a document is not
    /// copied but named in the report.
    #[test]
    fn fonts_that_may_not_go_along_are_reported_not_copied() {
        let dir = temp("restricted");
        // Selawik with its embedding bits set to "restricted".
        let mut data = include_bytes!("../assets/fonts/Selawik-Bold.ttf").to_vec();
        let n = u16::from_be_bytes([data[4], data[5]]) as usize;
        for i in 0..n {
            let rec = 12 + i * 16;
            if &data[rec..rec + 4] == b"OS/2" {
                let off = u32::from_be_bytes([
                    data[rec + 8],
                    data[rec + 9],
                    data[rec + 10],
                    data[rec + 11],
                ]) as usize;
                data[off + 8..off + 10].copy_from_slice(&2u16.to_be_bytes());
            }
        }
        assert!(!tracedraw_text::usable_in_documents(&data));
        let mut doc = Document::new(
            "Restricted",
            tracedraw_core::geometry::Size::new(50.0, 50.0),
        );
        doc.fonts.push(tracedraw_core::document::DocumentFont {
            family: "Selawik Locked".into(),
            data: std::sync::Arc::new(data),
        });
        let mut ids = doc.ids().clone();
        let mut s = Shape::new(
            ids.shape(),
            ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new("x", "Selawik Locked", 12.0)],
                origin: Point::new(1.0, 1.0),
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            },
        );
        s.stroke = None;
        doc.set_ids(ids);
        doc.pages[0].layers[0].shapes.push(s);
        let (copied, refused, _) = copy_fonts(&doc, &dir);
        assert!(copied.is_empty(), "{copied:?}");
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].0, "Selawik Locked");
        assert!(!dir.join(FONTS).exists());
        let r = Report {
            refused_fonts: refused,
            ..Default::default()
        };
        assert!(report_text(&r, "Restricted", "now").contains("Selawik Locked"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_of_copies_do_not_clash() {
        let taken = vec!["a.png".to_string(), "a-2.png".to_string()];
        assert_eq!(unique_name("b.png", &taken), "b.png");
        assert_eq!(unique_name("A.PNG", &taken), "A-3.PNG");
        assert_eq!(family_file_stem("Ac/me: Sans"), "Ac_me_ Sans");
        assert_eq!(font_extension(b"OTTO...."), "otf");
        assert_eq!(font_extension(b"\0\x01\0\0"), "ttf");
    }
}
