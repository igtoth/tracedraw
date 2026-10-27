//! The font catalog behind the Font Manager: every face the font database
//! holds and the faces of font files in folders the user adds, with what
//! the manager shows and filters by (technology, embedding rights, weight,
//! width, slant, scripts, OpenType features), plus installing font files
//! for TraceDraw and drawing a face's sample. See
//! docs/behavior/font-manager.md.

use crate::{FontKind, FontSystem};
use kurbo::BezPath;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What a face's licence allows (its OS/2 `fsType` bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Embedding {
    /// No restriction (0).
    Installable,
    /// Editable embedding (8).
    Editable,
    /// Preview and print embedding (4).
    PreviewPrint,
    /// Restricted licence (2): not embedded.
    Restricted,
    /// Only bitmaps may be embedded (0x200).
    BitmapOnly,
}

impl Embedding {
    pub const ALL: [Embedding; 5] = [
        Embedding::Installable,
        Embedding::Editable,
        Embedding::PreviewPrint,
        Embedding::Restricted,
        Embedding::BitmapOnly,
    ];

    /// The rights `fs_type` gives.
    pub fn from_fs_type(fs_type: u16) -> Embedding {
        if fs_type & 0x0200 != 0 {
            Embedding::BitmapOnly
        } else if fs_type & 0x000f == 0x0002 {
            Embedding::Restricted
        } else if fs_type & 0x0004 != 0 {
            Embedding::PreviewPrint
        } else if fs_type & 0x0008 != 0 {
            Embedding::Editable
        } else {
            Embedding::Installable
        }
    }
}

/// A writing system a face covers (from its OS/2 Unicode ranges).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Script {
    Latin,
    Greek,
    Cyrillic,
    Hebrew,
    Arabic,
    Devanagari,
    Bengali,
    Thai,
    Korean,
    Japanese,
    Chinese,
    Symbols,
}

impl Script {
    pub const ALL: [Script; 12] = [
        Script::Latin,
        Script::Greek,
        Script::Cyrillic,
        Script::Hebrew,
        Script::Arabic,
        Script::Devanagari,
        Script::Bengali,
        Script::Thai,
        Script::Korean,
        Script::Japanese,
        Script::Chinese,
        Script::Symbols,
    ];
}

/// The OpenType features the manager filters by, in its order.
pub const FEATURES: [&str; 10] = [
    "liga", "kern", "smcp", "onum", "lnum", "tnum", "frac", "swsh", "salt", "ss01",
];

/// A face in the font database, to read its data with
/// [`FontSystem::with_face`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceHandle(fontdb::ID);

/// One face as the Font Manager lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceRecord {
    /// The typographic family (name ID 16), else the family (name ID 1).
    pub family: String,
    /// The style name ("Bold Italic", "Light").
    pub style: String,
    pub weight: u16,
    /// OS/2 width class: 1 ultra-condensed to 9 ultra-expanded, 5 normal.
    pub width: u16,
    pub italic: bool,
    pub kind: FontKind,
    pub embedding: Embedding,
    pub glyphs: u16,
    pub scripts: Vec<Script>,
    /// The features of [`FEATURES`] the face has.
    pub features: Vec<String>,
    pub version: String,
    pub postscript: String,
    /// The file it comes from (`None` for fonts loaded from memory).
    pub path: Option<PathBuf>,
    /// Its index in a collection file.
    pub index: u32,
    /// In the font database: the system's or one installed for TraceDraw.
    pub installed: bool,
    /// A font of the operating system's own folders: never uninstalled.
    pub system: bool,
    /// The database face, for installed fonts.
    pub handle: Option<FaceHandle>,
}

impl FaceRecord {
    /// What collections remember the face by: its PostScript name, else
    /// its family and style.
    pub fn key(&self) -> String {
        if self.postscript.is_empty() {
            format!("{}|{}", self.family, self.style)
        } else {
            self.postscript.clone()
        }
    }

    /// The OS/2 width class as a word ("Normal", "Condensed").
    pub fn width_name(&self) -> &'static str {
        match self.width {
            1 => "Ultra-condensed",
            2 => "Extra-condensed",
            3 => "Condensed",
            4 => "Semi-condensed",
            6 => "Semi-expanded",
            7 => "Expanded",
            8 => "Extra-expanded",
            9 => "Ultra-expanded",
            _ => "Normal",
        }
    }
}

fn be16(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2)
        .map_or(0, |s| u16::from_be_bytes([s[0], s[1]]))
}

fn be32(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// The English name `id` of a face, else any.
fn name(f: &ttf_parser::Face, id: u16) -> Option<String> {
    let mut best: Option<(bool, String)> = None;
    for n in f.names() {
        if n.name_id != id {
            continue;
        }
        let Some(text) = n.to_string() else {
            continue;
        };
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        let english = n.language_id == 0x409 || n.platform_id == ttf_parser::PlatformId::Macintosh;
        if best.as_ref().is_none_or(|(e, _)| english && !e) {
            best = Some((english, text));
        }
    }
    best.map(|(_, t)| t)
}

/// The OpenType feature tags of a GSUB or GPOS table.
pub(crate) fn feature_tags(table: &[u8]) -> Vec<[u8; 4]> {
    let list = be16(table, 6) as usize;
    if list == 0 {
        return Vec::new();
    }
    let count = be16(table, list) as usize;
    (0..count.min(512))
        .filter_map(|i| {
            let at = list + 2 + i * 6;
            table.get(at..at + 4).map(|t| [t[0], t[1], t[2], t[3]])
        })
        .collect()
}

/// What the manager knows of face `index` in font `data` (not yet marked
/// installed or system).
pub fn describe(data: &[u8], index: u32) -> Option<FaceRecord> {
    let f = ttf_parser::Face::parse(data, index).ok()?;
    let raw = f.raw_face();
    let os2 = raw
        .table(ttf_parser::Tag::from_bytes(b"OS/2"))
        .unwrap_or(&[]);
    let tables = f.tables();
    let kind = if tables.fvar.is_some() {
        FontKind::Variable
    } else if tables.cff.is_some() || tables.cff2.is_some() {
        FontKind::OpenType
    } else {
        FontKind::TrueType
    };
    let symbol = tables.cmap.is_some_and(|c| {
        c.subtables
            .into_iter()
            .any(|s| s.platform_id == ttf_parser::PlatformId::Windows && s.encoding_id == 0)
    });
    let ranges = [be32(os2, 42), be32(os2, 46), be32(os2, 50), be32(os2, 54)];
    let bit = |n: usize| ranges[n / 32] & (1 << (n % 32)) != 0;
    let mut scripts = Vec::new();
    for (script, bits) in [
        (Script::Latin, &[0usize, 1, 2, 3][..]),
        (Script::Greek, &[7]),
        (Script::Cyrillic, &[9]),
        (Script::Hebrew, &[11]),
        (Script::Arabic, &[13]),
        (Script::Devanagari, &[15]),
        (Script::Bengali, &[16]),
        (Script::Thai, &[24]),
        (Script::Korean, &[28, 56]),
        (Script::Japanese, &[49, 50]),
        (Script::Chinese, &[59]),
    ] {
        if bits.iter().any(|b| bit(*b)) {
            scripts.push(script);
        }
    }
    // Fonts without ranges: judged by the letters they map.
    if os2.len() < 58 && f.glyph_index('a').is_some() {
        scripts.push(Script::Latin);
    }
    if symbol {
        scripts.push(Script::Symbols);
    }
    let mut tags = Vec::new();
    for t in [b"GSUB", b"GPOS"] {
        if let Some(table) = raw.table(ttf_parser::Tag::from_bytes(t)) {
            tags.extend(feature_tags(table));
        }
    }
    if tables.kern.is_some() {
        tags.push(*b"kern");
    }
    let features = FEATURES
        .iter()
        .filter(|feat| tags.iter().any(|t| t == feat.as_bytes()))
        .map(|feat| feat.to_string())
        .collect();
    let italic = f.is_italic() || f.is_oblique();
    let weight = f.weight().to_number();
    let width = match be16(os2, 6) {
        w @ 1..=9 => w,
        _ => 5,
    };
    let family = name(&f, ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
        .or_else(|| name(&f, ttf_parser::name_id::FAMILY))?;
    let style = name(&f, ttf_parser::name_id::TYPOGRAPHIC_SUBFAMILY)
        .or_else(|| name(&f, ttf_parser::name_id::SUBFAMILY))
        .unwrap_or_else(|| {
            match (weight >= 600, italic) {
                (false, false) => "Regular",
                (false, true) => "Italic",
                (true, false) => "Bold",
                (true, true) => "Bold Italic",
            }
            .to_string()
        });
    Some(FaceRecord {
        family,
        style,
        weight,
        width,
        italic,
        kind,
        embedding: Embedding::from_fs_type(be16(os2, 8)),
        glyphs: f.number_of_glyphs(),
        scripts,
        features,
        version: name(&f, ttf_parser::name_id::VERSION).unwrap_or_default(),
        postscript: name(&f, ttf_parser::name_id::POST_SCRIPT_NAME).unwrap_or_default(),
        path: None,
        index,
        installed: false,
        system: false,
        handle: None,
    })
}

/// Every face of the font file `data` (one, or each of a collection's).
pub fn describe_file(data: &[u8]) -> Vec<FaceRecord> {
    let n = ttf_parser::fonts_in_collection(data).unwrap_or(1).min(256);
    (0..n).filter_map(|i| describe(data, i)).collect()
}

/// The operating system's own font folders: fonts there are protected.
pub fn is_system_font(path: &Path) -> bool {
    let p = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    [
        "/windows/fonts/",
        "/system/library/fonts/",
        "/library/fonts/",
        "/usr/share/fonts/",
        "/usr/local/share/fonts/",
        "/usr/x11r6/lib/x11/fonts/",
    ]
    .iter()
    .any(|d| p.contains(d))
        && !p.contains("/users/")
}

/// Font files by their extension.
pub fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| matches!(e.as_str(), "ttf" | "otf" | "ttc" | "otc"))
}

/// The font files in `dir` and its subfolders (three levels down at
/// most), sorted.
pub fn font_files(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if depth < 3 {
                    walk(&p, depth + 1, out);
                }
            } else if is_font_file(&p) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, 0, &mut out);
    out.sort();
    out
}

/// The faces of the font files in `dir` (see [`font_files`]), not
/// installed.
pub fn scan_folder(dir: &Path) -> Vec<FaceRecord> {
    let mut out = Vec::new();
    for path in font_files(dir) {
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        for mut r in describe_file(&data) {
            r.system = is_system_font(&path);
            r.path = Some(path.clone());
            out.push(r);
        }
    }
    out
}

/// A face's sample: `text` laid out letter by letter (no shaping) with
/// the em as 1.0, y up, and its advance, ascent and descent (positive).
pub fn sample_outline(data: &[u8], index: u32, text: &str) -> Option<(BezPath, f64, f64, f64)> {
    let f = ttf_parser::Face::parse(data, index).ok()?;
    let upem = f64::from(f.units_per_em().max(1));
    struct Pen<'a> {
        path: &'a mut BezPath,
        x: f64,
        k: f64,
    }
    impl ttf_parser::OutlineBuilder for Pen<'_> {
        fn move_to(&mut self, x: f32, y: f32) {
            self.path
                .move_to((self.x + f64::from(x) * self.k, f64::from(y) * self.k));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.path
                .line_to((self.x + f64::from(x) * self.k, f64::from(y) * self.k));
        }
        fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
            self.path.quad_to(
                (self.x + f64::from(x1) * self.k, f64::from(y1) * self.k),
                (self.x + f64::from(x) * self.k, f64::from(y) * self.k),
            );
        }
        fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
            self.path.curve_to(
                (self.x + f64::from(x1) * self.k, f64::from(y1) * self.k),
                (self.x + f64::from(x2) * self.k, f64::from(y2) * self.k),
                (self.x + f64::from(x) * self.k, f64::from(y) * self.k),
            );
        }
        fn close(&mut self) {
            self.path.close_path();
        }
    }
    let k = 1.0 / upem;
    let mut path = BezPath::new();
    let mut x = 0.0;
    let mut any = false;
    for c in text.chars() {
        let Some(g) = crate::face_glyph(&f, c).or_else(|| f.glyph_index('?')) else {
            continue;
        };
        let mut pen = Pen {
            path: &mut path,
            x,
            k,
        };
        any |= f.outline_glyph(g, &mut pen).is_some();
        x += f64::from(f.glyph_hor_advance(g).unwrap_or(0)) * k;
    }
    if !any {
        return None;
    }
    let ascent = f64::from(f.ascender()) * k;
    let descent = -f64::from(f.descender()) * k;
    Some((path, x, ascent.max(0.0), descent.max(0.0)))
}

impl FontSystem {
    /// Every face in the font database as the manager lists it, installed;
    /// fonts documents brought are left out (they are not installed).
    pub fn catalog(&self) -> Vec<FaceRecord> {
        let document: Vec<String> = self
            .document_families
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let db = self.db();
        let mut out = Vec::new();
        for face in db.faces() {
            if face.families.iter().any(|(n, _)| document.contains(n)) {
                continue;
            }
            let path = match &face.source {
                #[cfg(not(target_arch = "wasm32"))]
                fontdb::Source::File(p) => Some(p.clone()),
                #[cfg(not(target_arch = "wasm32"))]
                fontdb::Source::SharedFile(p, _) => Some(p.clone()),
                _ => None,
            };
            let read = db.with_face_data(face.id, describe);
            let Some(Some(mut r)) = read else {
                continue;
            };
            r.installed = true;
            r.system = path.as_deref().is_some_and(is_system_font);
            r.path = path;
            r.handle = Some(FaceHandle(face.id));
            out.push(r);
        }
        out
    }

    /// Run `f` with the data and index of an installed face.
    pub fn with_face<T>(&self, handle: FaceHandle, f: impl FnOnce(&[u8], u32) -> T) -> Option<T> {
        self.db().with_face_data(handle.0, f)
    }

    /// The font files loaded into the database, for telling installed
    /// files of a folder from the others.
    pub fn installed_paths(&self) -> Vec<PathBuf> {
        let db = self.db();
        let mut out: Vec<PathBuf> = db
            .faces()
            .filter_map(|face| match &face.source {
                #[cfg(not(target_arch = "wasm32"))]
                fontdb::Source::File(p) => Some(p.clone()),
                #[cfg(not(target_arch = "wasm32"))]
                fontdb::Source::SharedFile(p, _) => Some(p.clone()),
                _ => None,
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Install font files for TraceDraw (this session; the application
    /// installs them again at the next start): files already loaded and
    /// files that do not read as fonts are skipped. Returns the families
    /// added.
    pub fn install_paths(&self, paths: &[PathBuf]) -> Vec<String> {
        let loaded = self.installed_paths();
        let mut ids = Vec::new();
        {
            let mut db = self.db.write().unwrap_or_else(|e| e.into_inner());
            for path in paths {
                if loaded.contains(path) {
                    continue;
                }
                let Ok(data) = std::fs::read(path) else {
                    continue;
                };
                #[cfg(not(target_arch = "wasm32"))]
                let source = {
                    let _ = data;
                    fontdb::Source::File(path.clone())
                };
                #[cfg(target_arch = "wasm32")]
                let source = fontdb::Source::Binary(Arc::new(data));
                ids.extend(db.load_font_source(source));
            }
        }
        self.after_loading(&ids)
    }

    /// Install font data loaded from memory for TraceDraw (the browser
    /// build's uploads). Returns the families added.
    pub fn install_data(&self, files: Vec<Vec<u8>>) -> Vec<String> {
        let mut ids = Vec::new();
        {
            let mut db = self.db.write().unwrap_or_else(|e| e.into_inner());
            for data in files {
                ids.extend(db.load_font_source(fontdb::Source::Binary(Arc::new(data))));
            }
        }
        self.after_loading(&ids)
    }

    /// Note the families the faces `ids` just loaded bring.
    fn after_loading(&self, ids: &[fontdb::ID]) -> Vec<String> {
        let mut added: Vec<String> = {
            let db = self.db();
            ids.iter()
                .filter_map(|id| db.face(*id))
                .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
                .collect()
        };
        added.extend(self.register_legacy(ids));
        added.sort();
        added.dedup();
        if added.is_empty() {
            return added;
        }
        let mut families = self.families.write().unwrap_or_else(|e| e.into_inner());
        families.extend(added.iter().cloned());
        families.sort();
        families.dedup();
        drop(families);
        self.fonts_changed();
        added
    }

    /// Take the faces of the font file `path` out of the database (fonts
    /// installed for TraceDraw; the system's own stay). True when some
    /// were removed.
    pub fn uninstall_path(&self, path: &Path) -> bool {
        if is_system_font(path) {
            return false;
        }
        let removed = {
            let mut db = self.db.write().unwrap_or_else(|e| e.into_inner());
            let ids: Vec<fontdb::ID> = db
                .faces()
                .filter(|face| match &face.source {
                    #[cfg(not(target_arch = "wasm32"))]
                    fontdb::Source::File(p) => p == path,
                    #[cfg(not(target_arch = "wasm32"))]
                    fontdb::Source::SharedFile(p, _) => p == path,
                    _ => false,
                })
                .map(|f| f.id)
                .collect();
            for id in &ids {
                db.remove_face(*id);
            }
            ids
        };
        if removed.is_empty() {
            return false;
        }
        {
            let mut legacy = self.legacy.write().unwrap_or_else(|e| e.into_inner());
            for faces in legacy.values_mut() {
                faces.retain(|id| !removed.contains(id));
            }
            legacy.retain(|_, faces| !faces.is_empty());
        }
        let mut families: Vec<String> = self
            .db()
            .faces()
            .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
            .collect();
        families.extend(
            self.legacy
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .keys()
                .cloned(),
        );
        families.sort();
        families.dedup();
        *self.families.write().unwrap_or_else(|e| e.into_inner()) = families;
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.faces.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.fonts_changed();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SELAWIK: &[u8] =
        include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
    const SELAWIK_BOLD: &[u8] =
        include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Bold.ttf");

    fn temp_dir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("tracedraw-catalog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("temp dir");
        d
    }

    #[test]
    fn a_face_is_described_by_what_its_file_says() {
        let r = describe(SELAWIK, 0).expect("reads");
        assert_eq!(r.family, "Selawik");
        assert!(!r.italic);
        assert_eq!(r.width, 5);
        assert_eq!(r.width_name(), "Normal");
        assert!(r.glyphs > 100, "{}", r.glyphs);
        assert!(r.scripts.contains(&Script::Latin), "{:?}", r.scripts);
        assert!(!r.postscript.is_empty());
        assert!(!r.version.is_empty());
        assert_eq!(r.key(), r.postscript);
        let b = describe(SELAWIK_BOLD, 0).expect("reads");
        assert!(b.weight >= 600, "{}", b.weight);
        assert_ne!(b.key(), r.key());
        assert!(describe(b"not a font", 0).is_none());
    }

    #[test]
    fn embedding_rights_follow_the_licence_bits() {
        assert_eq!(Embedding::from_fs_type(0), Embedding::Installable);
        assert_eq!(Embedding::from_fs_type(2), Embedding::Restricted);
        assert_eq!(Embedding::from_fs_type(4), Embedding::PreviewPrint);
        assert_eq!(Embedding::from_fs_type(8), Embedding::Editable);
        assert_eq!(Embedding::from_fs_type(0x208), Embedding::BitmapOnly);
    }

    #[test]
    fn folders_are_scanned_for_font_files() {
        let dir = temp_dir("scan");
        std::fs::create_dir_all(dir.join("sub")).expect("sub");
        std::fs::write(dir.join("a.ttf"), SELAWIK).expect("write");
        std::fs::write(dir.join("sub").join("b.TTF"), SELAWIK_BOLD).expect("write");
        std::fs::write(dir.join("notes.txt"), b"x").expect("write");
        std::fs::write(dir.join("broken.otf"), b"x").expect("write");
        let faces = scan_folder(&dir);
        assert_eq!(faces.len(), 2, "{faces:?}");
        assert!(faces.iter().all(|f| !f.installed && f.path.is_some()));
        assert!(!is_system_font(&dir.join("a.ttf")));
        assert!(is_system_font(Path::new("C:\\Windows\\Fonts\\arial.ttf")));
        assert!(is_system_font(Path::new("/usr/share/fonts/truetype/x.ttf")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn font_files_are_installed_and_uninstalled() {
        let dir = temp_dir("install");
        let path = dir.join("selawik.ttf");
        std::fs::write(&path, SELAWIK).expect("write");
        let fs = FontSystem::from_db(fontdb::Database::new());
        assert!(fs.catalog().is_empty());
        let added = fs.install_paths(std::slice::from_ref(&path));
        assert_eq!(added, vec!["Selawik".to_string()]);
        assert!(fs.has_family("Selawik"));
        // Installing again changes nothing.
        assert!(fs.install_paths(std::slice::from_ref(&path)).is_empty());
        let cat = fs.catalog();
        assert_eq!(cat.len(), 1);
        assert!(cat[0].installed && cat[0].handle.is_some());
        let glyphs = fs
            .with_face(cat[0].handle.expect("handle"), |d, i| {
                describe(d, i).map(|r| r.glyphs)
            })
            .flatten();
        assert_eq!(glyphs, Some(cat[0].glyphs));
        assert!(fs.uninstall_path(&path));
        assert!(!fs.has_family("Selawik"));
        assert!(fs.catalog().is_empty());
        assert!(!fs.uninstall_path(&path));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_sample_is_drawn_letter_by_letter() {
        let (path, advance, ascent, descent) = sample_outline(SELAWIK, 0, "Ab").expect("drawn");
        assert!(path.elements().len() > 4);
        assert!(advance > 0.5 && advance < 3.0, "{advance}");
        assert!(ascent > 0.5 && descent > 0.0);
        assert!(sample_outline(SELAWIK, 0, "").is_none());
    }
}
