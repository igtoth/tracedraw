//! Session recovery: drawings auto-backed up by a session that ended
//! without closing (a crash, a power cut) are offered again at the next
//! start.
//!
//! Each running session keeps a marker, `session-<id>.json`, in the
//! auto-backup folder: the time it last checked in (every minute) and the
//! auto-backups it wrote. A clean exit deletes the marker and those
//! backups. A marker that has not checked in for [`STALE_SECS`] belongs to
//! a session that ended without closing; its backups that still exist are
//! what can be recovered.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// How often a session checks in.
pub const HEARTBEAT_SECS: u64 = 60;
/// A marker this old belongs to a session that is gone.
pub const STALE_SECS: u64 = 180;

const PREFIX: &str = "session-";
const SUFFIX: &str = ".json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Marker {
    /// Seconds since 1970 of the last check-in.
    heartbeat: u64,
    /// File names of the auto-backups written, in the marker's folder.
    #[serde(default)]
    backups: Vec<String>,
}

/// This session's marker.
#[derive(Debug, Clone)]
pub struct Session {
    dir: PathBuf,
    id: String,
    marker: Marker,
    last_write: Option<u64>,
}

/// Drawings a session that ended without closing left behind.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub marker: PathBuf,
    pub backups: Vec<PathBuf>,
}

/// Seconds since 1970 (0 if the clock is before it).
pub fn now_secs() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A session id unlikely to repeat: the start time in nanoseconds.
pub fn new_id() -> String {
    let nanos = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

impl Session {
    pub fn new(dir: PathBuf, id: String) -> Self {
        Session {
            dir,
            id,
            marker: Marker::default(),
            last_write: None,
        }
    }

    fn path(&self) -> PathBuf {
        self.dir.join(format!("{PREFIX}{}{SUFFIX}", self.id))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// This session's id (the browser keeps each session's backups in a
    /// folder of that name, so two tabs never share one).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The marker's path (the browser's timer and page-leave handler
    /// update it, `web::backups`).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn marker(&self) -> PathBuf {
        self.path()
    }

    /// The auto-backups this session wrote, by name in its folder.
    pub fn backups(&self) -> &[String] {
        &self.marker.backups
    }

    /// Delete a backup this session wrote (its drawing was saved or
    /// closed) and check in without it.
    pub fn forget(&mut self, name: &str, now: u64) {
        if !self.marker.backups.iter().any(|b| b == name) {
            return;
        }
        let _ = crate::files::remove(self.dir.join(name));
        self.marker.backups.retain(|b| b != name);
        self.write(now);
    }

    /// Check in when due (or at once the first time). Returns true when
    /// the marker was written.
    pub fn heartbeat(&mut self, now: u64) -> bool {
        if self
            .last_write
            .is_some_and(|t| now.saturating_sub(t) < HEARTBEAT_SECS)
        {
            return false;
        }
        self.write(now)
    }

    /// Record an auto-backup this session wrote and check in.
    pub fn wrote(&mut self, name: &str, now: u64) {
        if !self.marker.backups.iter().any(|b| b == name) {
            self.marker.backups.push(name.to_string());
        }
        self.write(now);
    }

    fn write(&mut self, now: u64) -> bool {
        self.marker.heartbeat = now;
        self.last_write = Some(now);
        let Ok(json) = serde_json::to_string(&self.marker) else {
            return false;
        };
        match crate::files::write_creating(self.path(), json) {
            Ok(()) => true,
            Err(e) => {
                log::warn!("session marker: {e}");
                false
            }
        }
    }

    /// A clean exit: the marker and this session's auto-backups go.
    pub fn end(&self) {
        for b in &self.marker.backups {
            let _ = crate::files::remove(self.dir.join(b));
        }
        let _ = crate::files::remove(self.path());
    }

    /// An exit that leaves drawings unsaved (the system quit the program
    /// without asking, the browser tab closed): the backups stay and the
    /// marker says the session is gone, so the next start offers them at
    /// once instead of after [`STALE_SECS`].
    pub fn leave(&mut self) {
        self.write(0);
    }
}

/// The id of the session whose marker is called `name`.
pub fn marker_id(name: &str) -> Option<&str> {
    name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)
}

/// Check in for the session whose marker is at `path`, keeping what it
/// holds (the browser does this from a timer, which runs while the tab
/// draws nothing). Nothing happens once the marker is gone.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn keep_alive(path: &Path, now: u64) {
    rewrite_heartbeat(path, now);
}

/// Say that the session whose marker is at `path` is gone, keeping its
/// backups for the next start.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn mark_gone(path: &Path) {
    rewrite_heartbeat(path, 0);
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn rewrite_heartbeat(path: &Path, heartbeat: u64) {
    let Some(mut marker) = crate::files::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Marker>(&b).ok())
    else {
        return;
    };
    marker.heartbeat = heartbeat;
    if let Ok(json) = serde_json::to_string(&marker) {
        if let Err(e) = crate::files::write_creating(path, json) {
            log::warn!("session marker: {e}");
        }
    }
}

/// The sessions in `dir` that stopped checking in by `now`, with the
/// backups of theirs that still exist; `own` is skipped.
pub fn find(dir: &Path, now: u64, own: Option<&str>) -> Vec<Found> {
    let mut out = Vec::new();
    let mut paths = crate::files::list_dir(dir);
    paths.sort();
    for path in paths {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(id) = marker_id(name) else {
            continue;
        };
        if Some(id) == own {
            continue;
        }
        // Unreadable: nothing to offer, and it can go.
        let marker: Marker = crate::files::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if now.saturating_sub(marker.heartbeat) < STALE_SECS {
            continue;
        }
        let backups = marker
            .backups
            .iter()
            // Only names inside the folder (the browser's have a folder
            // of their session's own): never a root, `..` or a drive.
            .filter(|b| {
                !b.is_empty()
                    && Path::new(b)
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_)))
            })
            .map(|b| dir.join(b))
            .filter(|p| crate::files::exists(p))
            .collect();
        out.push(Found {
            marker: path,
            backups,
        });
    }
    out
}

/// Forget what a gone session left: its marker, and with `with_backups`
/// its backups too.
pub fn dismiss(found: &[Found], with_backups: bool) {
    for f in found {
        if with_backups {
            for b in &f.backups {
                let _ = crate::files::remove(b);
            }
        }
        let _ = crate::files::remove(&f.marker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tracedraw-recovery-{name}-{}", new_id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_session_that_stops_checking_in_leaves_its_backups_to_recover() {
        let dir = temp("stale");
        let mut s = Session::new(dir.clone(), "a".into());
        assert!(s.heartbeat(1000));
        assert!(!s.heartbeat(1030));
        std::fs::write(dir.join("AutoBackup_of_x.tdraw"), b"{}").expect("write");
        s.wrote("AutoBackup_of_x.tdraw", 1040);
        s.wrote("AutoBackup_of_gone.tdraw", 1041);
        // Still alive: nothing to recover.
        assert!(find(&dir, 1100, None).is_empty());
        // Our own session is never offered.
        assert!(find(&dir, 5000, Some("a")).is_empty());
        let found = find(&dir, 5000, Some("b"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].backups, vec![dir.join("AutoBackup_of_x.tdraw")]);
        dismiss(&found, false);
        assert!(find(&dir, 5000, Some("b")).is_empty());
        assert!(dir.join("AutoBackup_of_x.tdraw").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_clean_exit_removes_the_marker_and_its_backups() {
        let dir = temp("clean");
        let mut s = Session::new(dir.clone(), "a".into());
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("AutoBackup_of_y.tdraw"), b"{}").expect("write");
        std::fs::write(dir.join("AutoBackup_of_other.tdraw"), b"{}").expect("write");
        s.wrote("AutoBackup_of_y.tdraw", 10);
        s.end();
        assert!(!dir.join("AutoBackup_of_y.tdraw").exists());
        assert!(dir.join("AutoBackup_of_other.tdraw").exists());
        assert!(find(&dir, 10_000, None).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The browser's timer keeps a session checked in while no frame is
    /// drawn, and leaving the page marks it gone at once; both keep the
    /// backups the marker names, and neither brings back a marker another
    /// session has dismissed (#1395).
    #[test]
    fn a_marker_kept_alive_or_marked_gone_keeps_its_backups() {
        let dir = temp("alive");
        let mut s = Session::new(dir.clone(), "a".into());
        std::fs::create_dir_all(dir.join("a")).expect("dir");
        // A browser session's backups are in a folder of its own.
        std::fs::write(dir.join("a").join("AutoBackup_of_x.tdraw"), b"{}").expect("write");
        s.wrote("a/AutoBackup_of_x.tdraw", 1000);
        keep_alive(&s.marker(), 5000);
        assert!(find(&dir, 5100, None).is_empty());
        mark_gone(&s.marker());
        let found = find(&dir, 5100, None);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].backups,
            vec![dir.join("a").join("AutoBackup_of_x.tdraw")]
        );
        dismiss(&found, true);
        keep_alive(&s.marker(), 6000);
        assert!(!s.marker().exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_session_forgets_the_backup_of_a_saved_drawing() {
        let dir = temp("forget");
        let mut s = Session::new(dir.clone(), "a".into());
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("AutoBackup_of_y.tdraw"), b"{}").expect("write");
        s.wrote("AutoBackup_of_y.tdraw", 10);
        s.forget("AutoBackup_of_y.tdraw", 20);
        assert!(s.backups().is_empty());
        assert!(!dir.join("AutoBackup_of_y.tdraw").exists());
        s.leave();
        let found = find(&dir, STALE_SECS, None);
        assert_eq!(found.len(), 1);
        assert!(found[0].backups.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broken_markers_never_reach_outside_the_folder() {
        let dir = temp("broken");
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("session-x.json"), b"not json").expect("write");
        std::fs::write(
            dir.join("session-y.json"),
            br#"{"heartbeat":0,"backups":["../outside.tdraw","/etc/passwd","a/../../b.tdraw",""]}"#,
        )
        .expect("write");
        let found = find(&dir, 10_000, None);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|f| f.backups.is_empty()));
        dismiss(&found, true);
        assert!(find(&dir, 10_000, None).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
