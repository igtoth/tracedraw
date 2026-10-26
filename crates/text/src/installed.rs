//! The fonts Windows has installed are the ones its font registry lists.
//! Its font folders can still hold the file of a font uninstalled while
//! it was in use (deleted at the next start), which a scan of the
//! folders would find and offer as installed (#504); and the registry can
//! list a font by its full path in a folder of its own (font managers
//! install them so), which a scan of the font folders never finds (#860).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The key a font file is known by: its file name in lower case.
pub(crate) fn file_key(path: &Path) -> Option<String> {
    Some(path.file_name()?.to_string_lossy().to_lowercase())
}

/// Fewer registered files than this is not a real list (a registry read
/// that failed part way); nothing is dropped then.
const MIN_REGISTERED: usize = 20;

/// The faces of `db` read from files `registered` does not list.
pub(crate) fn unregistered_faces(
    db: &fontdb::Database,
    registered: &HashSet<String>,
) -> Vec<fontdb::ID> {
    if registered.len() < MIN_REGISTERED {
        return Vec::new();
    }
    db.faces()
        .filter(|f| {
            let path = match &f.source {
                fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => p,
                _ => return false,
            };
            file_key(path).is_some_and(|k| !registered.contains(&k))
        })
        .map(|f| f.id)
        .collect()
}

/// The files the registry lists by a full path that no face of `db` was
/// read from (by file name, any case): fonts installed outside the font
/// folders.
pub(crate) fn registered_elsewhere(db: &fontdb::Database, registered: &[PathBuf]) -> Vec<PathBuf> {
    let loaded: HashSet<String> = db
        .faces()
        .filter_map(|f| match &f.source {
            fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => file_key(p),
            _ => None,
        })
        .collect();
    let mut out: Vec<PathBuf> = Vec::new();
    for p in registered {
        if p.is_absolute() && file_key(p).is_some_and(|k| !loaded.contains(&k)) && !out.contains(p)
        {
            out.push(p.clone());
        }
    }
    out
}

/// Leave out the system fonts Windows no longer lists as installed, and
/// load those it lists in folders of their own.
#[cfg(windows)]
pub(crate) fn sync_with_registry(db: &mut fontdb::Database) {
    let Some(paths) = registered_files() else {
        return;
    };
    let registered: HashSet<String> = paths.iter().filter_map(|p| file_key(p)).collect();
    let gone = unregistered_faces(db, &registered);
    if !gone.is_empty() {
        log::info!("{} font faces left out: not installed", gone.len());
    }
    for id in gone {
        db.remove_face(id);
    }
    let elsewhere = registered_elsewhere(db, &paths);
    for p in &elsewhere {
        if let Err(e) = db.load_font_file(p) {
            log::warn!("font {}: {e}", p.display());
        }
    }
    if !elsewhere.is_empty() {
        log::info!(
            "{} fonts installed outside the font folders",
            elsewhere.len()
        );
    }
}

/// The files of the fonts Windows lists as installed, for the machine and
/// for the user (a file name, for a file in the fonts folder, or a full
/// path); `None` when neither list reads.
#[cfg(windows)]
fn registered_files() -> Option<Vec<PathBuf>> {
    use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegEnumValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
        KEY_READ, REG_EXPAND_SZ, REG_SZ,
    };
    let key: Vec<u16> = "Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut files = Vec::new();
    let mut read = false;
    let mut name = vec![0u16; 16384];
    let mut data = vec![0u8; 8192];
    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let mut h: HKEY = std::ptr::null_mut();
        // SAFETY: `key` is a NUL-terminated wide string and `h` a valid
        // place for the handle.
        if unsafe { RegOpenKeyExW(root, key.as_ptr(), 0, KEY_READ, &mut h) } != ERROR_SUCCESS {
            continue;
        }
        read = true;
        for index in 0.. {
            let mut name_len = name.len() as u32;
            let mut data_len = data.len() as u32;
            let mut kind = 0u32;
            // SAFETY: the buffers are as long as the lengths passed say.
            let r = unsafe {
                RegEnumValueW(
                    h,
                    index,
                    name.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null(),
                    &mut kind,
                    data.as_mut_ptr(),
                    &mut data_len,
                )
            };
            if r == ERROR_NO_MORE_ITEMS {
                break;
            }
            if r != ERROR_SUCCESS || (kind != REG_SZ && kind != REG_EXPAND_SZ) {
                continue;
            }
            let wide: Vec<u16> = data[..data_len as usize]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            let value = String::from_utf16_lossy(&wide);
            let value = value.trim();
            if !value.is_empty() {
                files.push(PathBuf::from(value));
            }
        }
        // SAFETY: `h` was opened above.
        unsafe { RegCloseKey(h) };
    }
    read.then_some(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A face whose file the registry does not list is left out; one it
    /// lists (by file name, any case, whatever folder) stays; a short
    /// list drops nothing (#504).
    #[test]
    fn faces_of_files_not_registered_are_left_out() {
        let font: &[u8] =
            include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
        let dir = std::env::temp_dir().join(format!("td_installed_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let kept = dir.join("Kept.ttf");
        let gone = dir.join("gone.ttf");
        std::fs::write(&kept, font).expect("write");
        std::fs::write(&gone, font).expect("write");
        let mut db = fontdb::Database::new();
        db.load_font_file(&kept).expect("kept");
        db.load_font_file(&gone).expect("gone");
        let mut registered: HashSet<String> =
            (0..MIN_REGISTERED).map(|i| format!("f{i}.ttf")).collect();
        registered.insert("kept.ttf".into());
        let out = unregistered_faces(&db, &registered);
        let paths: Vec<_> = out
            .iter()
            .filter_map(|id| db.face(*id))
            .filter_map(|f| match &f.source {
                fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => Some(p.clone()),
                _ => None,
            })
            .collect();
        assert!(!paths.is_empty());
        assert!(paths.iter().all(|p| p == &gone), "{paths:?}");
        let short: HashSet<String> = ["kept.ttf".to_string()].into();
        assert!(unregistered_faces(&db, &short).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A font the registry lists by a full path outside the font folders
    /// is loaded; one already read (any folder, any case) or listed by its
    /// file name only is not (#860).
    #[test]
    fn fonts_registered_elsewhere_are_found() {
        let font: &[u8] =
            include_bytes!("../../../apps/tracedraw/assets/fonts/Selawik-Regular.ttf");
        let dir = std::env::temp_dir().join(format!("td_elsewhere_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let loaded = dir.join("Loaded.ttf");
        std::fs::write(&loaded, font).expect("write");
        let mut db = fontdb::Database::new();
        db.load_font_file(&loaded).expect("loaded");
        let other = dir.join("fonts").join("FZHTK.TTF");
        let registered = vec![
            PathBuf::from("arial.ttf"),
            dir.join("elsewhere").join("LOADED.TTF"),
            other.clone(),
            other.clone(),
        ];
        assert_eq!(registered_elsewhere(&db, &registered), vec![other]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
