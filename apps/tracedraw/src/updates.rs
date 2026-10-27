//! Checking for a newer release (Help > Updates, and once a day at start
//! when Options > Global > General allows it). The check asks GitHub for
//! the latest release of the project, compares its version with this one
//! and, when it is newer, offers the download for this system and the
//! release notes. Nothing is installed without the user: Download opens
//! the installer's link, and the system's installer does the rest. A
//! Linux AppImage (run as one, so `APPIMAGE` names its file) can also be
//! replaced in place by the release's AppImage after its SHA-256 digest
//! checks out ([`Installer`]).
//!
//! Desktop only: the browser build is always the latest.

use serde::Deserialize;
use std::sync::{Arc, Mutex};

/// The project's latest release, as GitHub describes it.
pub const LATEST_URL: &str = "https://api.github.com/repos/igtoth/tracedraw/releases/latest";
/// The release list, for when no download fits this system.
pub const RELEASES_PAGE: &str = "https://github.com/igtoth/tracedraw/releases";
/// How often the check at start runs, seconds.
pub const DAILY: u64 = 24 * 60 * 60;

/// A release newer than this version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    /// The release's page (its notes).
    pub page: String,
    /// The download for this system, when the release has one.
    pub download: Option<String>,
    /// The release's AppImage with its digest (Linux), for installing in
    /// place.
    pub appimage: Option<Package>,
}

/// A release file that can be installed in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub url: String,
    /// Lower-case hex.
    pub sha256: String,
    pub size: u64,
}

/// Where a check stands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    /// This version is the latest.
    Latest,
    Available(Release),
    Failed(String),
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    /// "sha256:<hex>" (GitHub computes it for every upload).
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// `1.2.3` from `v1.2.3` (pre-release parts after `-` are dropped).
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut it = core.split('.').map(|p| p.parse::<u64>());
    let major = it.next()?.ok()?;
    let minor = it.next().unwrap_or(Ok(0)).ok()?;
    let patch = it.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch))
}

/// Whether `tag` names a version later than `current`.
pub fn is_newer(tag: &str, current: &str) -> bool {
    match (parse_version(tag), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// How this copy of TraceDraw was installed, which decides the release
/// file that updates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Linux, run as an AppImage (`APPIMAGE` names its file).
    AppImage,
    /// Linux, installed from the release's `.deb` (dpkg lists the program).
    Deb,
    /// Linux, the program unpacked from the release's tar.gz.
    Tarball,
    /// Windows, installed with the release's setup program.
    Setup,
    /// Windows, the portable zip (the settings folder next to the program).
    Portable,
    /// macOS, the disk image's app.
    Dmg,
    /// Anything the release has no file for (an rpm or other distribution
    /// package, a sandboxed copy, the browser): no download is offered.
    Other,
}

/// The release's architecture names for `arch` (as
/// `std::env::consts::ARCH`): the one in the archive, AppImage, disk
/// image and Windows names, and the Debian one. `None` for an
/// architecture the release has no build of.
fn arch_names(arch: &str) -> Option<(&'static str, &'static str)> {
    match arch {
        "x86_64" => Some(("x86_64", "amd64")),
        "aarch64" => Some(("arm64", "arm64")),
        _ => None,
    }
}

/// The exact name of the release file of `version` that updates a copy
/// installed as `kind` on `os` (as `std::env::consts::OS`) and `arch`.
/// `None` when the release builds no such file. The names are the ones
/// the release workflow gives its files.
pub fn asset_name(version: &str, os: &str, arch: &str, kind: Kind) -> Option<String> {
    let (a, deb) = arch_names(arch)?;
    Some(match (os, kind) {
        ("linux", Kind::AppImage) => format!("tracedraw-{version}-linux-{a}.AppImage"),
        ("linux", Kind::Tarball) => format!("tracedraw-{version}-linux-{a}.tar.gz"),
        ("linux", Kind::Deb) => format!("tracedraw_{version}_{deb}.deb"),
        ("windows", Kind::Setup) => format!("tracedraw-{version}-windows-{a}-setup.exe"),
        ("windows", Kind::Portable) => format!("tracedraw-{version}-windows-{a}-portable.zip"),
        ("macos", Kind::Dmg) => format!("tracedraw-{version}-macos-{a}.dmg"),
        _ => return None,
    })
}

/// How a Linux copy was installed: run as an AppImage, inside a sandbox
/// (Flatpak, Snap), from the `.deb` (dpkg's file list for the package
/// names `exe`), from another system package (a program under `/usr`
/// that dpkg does not list, such as an rpm), else unpacked from the
/// tar.gz.
pub fn linux_kind(
    appimage: bool,
    sandboxed: bool,
    exe: &std::path::Path,
    dpkg_list: Option<&str>,
) -> Kind {
    if appimage {
        return Kind::AppImage;
    }
    if sandboxed {
        return Kind::Other;
    }
    let listed = dpkg_list.is_some_and(|l| l.lines().any(|line| std::path::Path::new(line) == exe));
    if listed {
        Kind::Deb
    } else if exe.starts_with("/usr") {
        Kind::Other
    } else {
        Kind::Tarball
    }
}

/// How the running copy was installed.
#[cfg(not(target_arch = "wasm32"))]
pub fn current_kind() -> Kind {
    let exe = std::env::current_exe().ok();
    match std::env::consts::OS {
        "linux" => {
            let Some(exe) = exe else {
                return Kind::Other;
            };
            let dpkg = std::fs::read_to_string("/var/lib/dpkg/info/tracedraw.list").ok();
            linux_kind(
                std::env::var_os("APPIMAGE").is_some(),
                std::env::var_os("FLATPAK_ID").is_some() || std::env::var_os("SNAP").is_some(),
                &exe,
                dpkg.as_deref(),
            )
        }
        "windows" => match exe.as_deref().and_then(crate::settings::portable_dir) {
            Some(_) => Kind::Portable,
            None => Kind::Setup,
        },
        "macos" => Kind::Dmg,
        _ => Kind::Other,
    }
}

/// Read GitHub's answer: the release when it is newer than `current`
/// (drafts and pre-releases never count), `None` when this version is the
/// latest. The download is the release file named exactly for this
/// system, its processor and how this copy was installed (`kind`); when
/// the release has no such file there is none, never a file built for
/// another processor or package.
pub fn parse_latest(
    json: &str,
    current: &str,
    os: &str,
    arch: &str,
    kind: Kind,
) -> Result<Option<Release>, String> {
    let r: ApiRelease = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if r.draft || r.prerelease || !is_newer(&r.tag_name, current) {
        return Ok(None);
    }
    let version = r.tag_name.trim_start_matches(['v', 'V']).to_string();
    let find = |kind: Kind| {
        let name = asset_name(&version, os, arch, kind)?;
        r.assets.iter().find(|a| a.name == name)
    };
    let download = find(kind).map(|a| a.browser_download_url.clone());
    let appimage = if kind == Kind::AppImage {
        find(Kind::AppImage).and_then(|a| {
            let hex = a.digest.as_deref()?.strip_prefix("sha256:")?;
            (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) && a.size > 0).then(
                || Package {
                    url: a.browser_download_url.clone(),
                    sha256: hex.to_ascii_lowercase(),
                    size: a.size,
                },
            )
        })
    } else {
        None
    };
    Ok(Some(Release {
        version,
        page: r.html_url,
        download,
        appimage,
    }))
}

/// A check running in the background; its result lands in `state`.
#[derive(Debug, Clone, Default)]
pub struct Checker {
    pub state: Arc<Mutex<UpdateState>>,
}

impl Checker {
    pub fn get(&self) -> UpdateState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Start a check unless one is running; `ctx` gets a frame when it
    /// ends. In a browser the check is a fetch of the same address.
    pub fn start(&self, ctx: Option<egui::Context>) {
        if let Ok(mut s) = self.state.lock() {
            if *s == UpdateState::Checking {
                return;
            }
            *s = UpdateState::Checking;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let state = self.state.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let req = crate::net::HttpRequest {
                    method: "GET",
                    url: LATEST_URL.into(),
                    headers: vec![("Accept".into(), "application/vnd.github+json".into())],
                    body: String::new(),
                    bytes: Vec::new(),
                };
                let next = match crate::net::send(req).await {
                    Ok(r) if r.status == 200 => {
                        match parse_latest(
                            &r.body,
                            env!("CARGO_PKG_VERSION"),
                            "web",
                            "wasm32",
                            Kind::Other,
                        ) {
                            Ok(Some(r)) => UpdateState::Available(r),
                            Ok(None) => UpdateState::Latest,
                            Err(e) => UpdateState::Failed(e),
                        }
                    }
                    Ok(r) => UpdateState::Failed(format!("GitHub answered {}", r.status)),
                    Err(e) => UpdateState::Failed(e),
                };
                if let Ok(mut s) = state.lock() {
                    *s = next;
                }
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
                crate::web::request_repaint();
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let state = self.state.clone();
            std::thread::spawn(move || {
                let result = fetch().and_then(|json| {
                    parse_latest(
                        &json,
                        env!("CARGO_PKG_VERSION"),
                        std::env::consts::OS,
                        std::env::consts::ARCH,
                        current_kind(),
                    )
                });
                let next = match result {
                    Ok(Some(r)) => UpdateState::Available(r),
                    Ok(None) => UpdateState::Latest,
                    Err(e) => UpdateState::Failed(e),
                };
                if let Ok(mut s) = state.lock() {
                    *s = next;
                }
                if let Some(ctx) = ctx {
                    ctx.request_repaint();
                }
            });
        }
    }
}

/// Where an in-place install goes: the running AppImage's file (Linux,
/// `APPIMAGE` set by the AppImage runtime).
pub fn appimage_target() -> Option<std::path::PathBuf> {
    if std::env::consts::OS != "linux" || crate::files::WEB {
        return None;
    }
    let p = std::path::PathBuf::from(std::env::var_os("APPIMAGE")?);
    p.is_file().then_some(p)
}

/// Where an in-place install stands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InstallState {
    #[default]
    Idle,
    Downloading {
        done: u64,
        total: u64,
    },
    /// The new version is in place; it runs from the next start.
    Installed(String),
    Failed(String),
}

/// Write `from` next to `target`, check its size and SHA-256 digest, give
/// it the target's permissions and move it over the target. On any
/// failure the target is left as it was and the partial file removed.
/// `progress` gets the bytes written so far.
pub fn install_file(
    from: &mut dyn std::io::Read,
    package: &Package,
    target: &std::path::Path,
    progress: &mut dyn FnMut(u64),
) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Write;
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("no file name")?;
    let part = target.with_file_name(format!(".{name}.update"));
    let result = (|| {
        let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buf = vec![0u8; 64 * 1024];
        let mut done = 0u64;
        loop {
            let n = from.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            done += n as u64;
            if done > package.size {
                return Err("the download is larger than the release says".to_string());
            }
            hash.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            progress(done);
        }
        if done != package.size {
            return Err(format!(
                "the download stopped at {done} of {} bytes",
                package.size
            ));
        }
        let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if got != package.sha256 {
            return Err("the download does not match the release's digest".to_string());
        }
        out.sync_all().map_err(|e| e.to_string())?;
        drop(out);
        let perms = std::fs::metadata(target)
            .map_err(|e| e.to_string())?
            .permissions();
        std::fs::set_permissions(&part, perms).map_err(|e| e.to_string())?;
        std::fs::rename(&part, target).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

/// An in-place install running in the background.
#[derive(Debug, Clone, Default)]
pub struct Installer {
    pub state: Arc<Mutex<InstallState>>,
}

impl Installer {
    pub fn get(&self) -> InstallState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Download `package` and put it in place of `target`, unless an
    /// install is running.
    pub fn start(
        &self,
        version: String,
        package: Package,
        target: std::path::PathBuf,
        ctx: Option<egui::Context>,
    ) {
        #[cfg(target_arch = "wasm32")]
        let _ = (version, package, target, ctx);
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(mut s) = self.state.lock() {
                if matches!(*s, InstallState::Downloading { .. }) {
                    return;
                }
                *s = InstallState::Downloading {
                    done: 0,
                    total: package.size,
                };
            }
            let state = self.state.clone();
            std::thread::spawn(move || {
                let set = |next: InstallState| {
                    if let Ok(mut s) = state.lock() {
                        *s = next;
                    }
                    if let Some(ctx) = &ctx {
                        ctx.request_repaint();
                    }
                };
                let result = download(&package.url).and_then(|mut body| {
                    let mut last = 0u64;
                    install_file(&mut body, &package, &target, &mut |done| {
                        // A frame now and then, not for every block.
                        if done - last >= 512 * 1024 || done == package.size {
                            last = done;
                            set(InstallState::Downloading {
                                done,
                                total: package.size,
                            });
                        }
                    })
                });
                set(match result {
                    Ok(()) => InstallState::Installed(version),
                    Err(e) => InstallState::Failed(e),
                });
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn download(url: &str) -> Result<Box<dyn std::io::Read + Send>, String> {
    use ureq::tls::{RootCerts, TlsConfig};
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    // Redirects (GitHub sends downloads to its file host) are followed
    // here, each on a request of its own; the redirect's body is skipped.
    let mut url = url.to_string();
    for _ in 0..5 {
        let resp = agent
            .get(&url)
            .config()
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .header(
                "User-Agent",
                concat!("TraceDraw/", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        if (300..400).contains(&status) {
            let next = resp
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or("a redirect without a location")?;
            url = next.to_string();
            continue;
        }
        if status != 200 {
            return Err(format!("the download answered {status}"));
        }
        return Ok(Box::new(resp.into_body().into_reader()));
    }
    Err("too many redirects".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch() -> Result<String, String> {
    use ureq::tls::{RootCerts, TlsConfig};
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .into();
    let mut resp = agent
        .get(LATEST_URL)
        .header(
            "User-Agent",
            concat!("TraceDraw/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    resp.body_mut()
        .with_config()
        .limit(1 << 20)
        .read_to_string()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The files of the v0.19.8 release, named and listed in the order
    /// GitHub gave them (the arm64 Linux files come first), with the
    /// version moved to 0.5.0. Each URL ends in the file's name.
    const SAMPLE: &str = r#"{
        "tag_name": "v0.5.0",
        "html_url": "https://github.com/igtoth/tracedraw/releases/tag/v0.5.0",
        "prerelease": false,
        "draft": false,
        "assets": [
            {"name": "tracedraw-0.5.0-linux-arm64.AppImage", "browser_download_url": "https://x/tracedraw-0.5.0-linux-arm64.AppImage", "size": 7, "digest": "sha256:68e481c13fc5dc04ed6b106a9f801e1fa2689889f070a02b4fb89e8cb29c91ef"},
            {"name": "tracedraw-0.5.0-linux-arm64.tar.gz", "browser_download_url": "https://x/tracedraw-0.5.0-linux-arm64.tar.gz"},
            {"name": "tracedraw-0.5.0-linux-x86_64.AppImage", "browser_download_url": "https://x/tracedraw-0.5.0-linux-x86_64.AppImage", "size": 5, "digest": "sha256:2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824"},
            {"name": "tracedraw-0.5.0-linux-x86_64.tar.gz", "browser_download_url": "https://x/tracedraw-0.5.0-linux-x86_64.tar.gz"},
            {"name": "tracedraw-0.5.0-macos-arm64.dmg", "browser_download_url": "https://x/tracedraw-0.5.0-macos-arm64.dmg"},
            {"name": "tracedraw-0.5.0-macos-x86_64.dmg", "browser_download_url": "https://x/tracedraw-0.5.0-macos-x86_64.dmg"},
            {"name": "tracedraw-0.5.0-windows-x86_64-portable.zip", "browser_download_url": "https://x/tracedraw-0.5.0-windows-x86_64-portable.zip"},
            {"name": "tracedraw-0.5.0-windows-x86_64-setup.exe", "browser_download_url": "https://x/tracedraw-0.5.0-windows-x86_64-setup.exe"},
            {"name": "tracedraw_0.5.0_amd64.deb", "browser_download_url": "https://x/tracedraw_0.5.0_amd64.deb"},
            {"name": "tracedraw_0.5.0_arm64.deb", "browser_download_url": "https://x/tracedraw_0.5.0_arm64.deb"}
        ]
    }"#;

    fn pick_in(json: &str, os: &str, arch: &str, kind: Kind) -> Option<String> {
        parse_latest(json, "0.4.0", os, arch, kind)
            .expect("parses")
            .expect("newer")
            .download
            .map(|u| u.trim_start_matches("https://x/").to_string())
    }

    fn pick(os: &str, arch: &str, kind: Kind) -> Option<String> {
        pick_in(SAMPLE, os, arch, kind)
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("v0.5.0", "0.4.0"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0", "0.99.0"));
        assert!(!is_newer("v0.4.0", "0.4.0"));
        assert!(!is_newer("v0.3.9", "0.4.0"));
        assert!(!is_newer("nightly", "0.4.0"));
        assert!(!is_newer("v0.5.0-rc1", "0.5.0"));
    }

    /// Every system, processor and kind of install gets exactly its own
    /// file of the release (#1038: an x86_64 AppImage took the arm64 one,
    /// listed first).
    #[test]
    fn the_download_fits_the_system() {
        use Kind::*;
        let cases = [
            (
                "linux",
                "x86_64",
                AppImage,
                "tracedraw-0.5.0-linux-x86_64.AppImage",
            ),
            (
                "linux",
                "aarch64",
                AppImage,
                "tracedraw-0.5.0-linux-arm64.AppImage",
            ),
            (
                "linux",
                "x86_64",
                Tarball,
                "tracedraw-0.5.0-linux-x86_64.tar.gz",
            ),
            (
                "linux",
                "aarch64",
                Tarball,
                "tracedraw-0.5.0-linux-arm64.tar.gz",
            ),
            ("linux", "x86_64", Deb, "tracedraw_0.5.0_amd64.deb"),
            ("linux", "aarch64", Deb, "tracedraw_0.5.0_arm64.deb"),
            (
                "windows",
                "x86_64",
                Setup,
                "tracedraw-0.5.0-windows-x86_64-setup.exe",
            ),
            (
                "windows",
                "x86_64",
                Portable,
                "tracedraw-0.5.0-windows-x86_64-portable.zip",
            ),
            ("macos", "aarch64", Dmg, "tracedraw-0.5.0-macos-arm64.dmg"),
            ("macos", "x86_64", Dmg, "tracedraw-0.5.0-macos-x86_64.dmg"),
        ];
        for (os, arch, kind, want) in cases {
            assert_eq!(
                pick(os, arch, kind).as_deref(),
                Some(want),
                "{os} {arch} {kind:?}"
            );
            // The file list's order does not matter.
            let reversed = {
                let mut v: serde_json::Value = serde_json::from_str(SAMPLE).expect("json");
                v["assets"].as_array_mut().expect("assets").reverse();
                v.to_string()
            };
            assert_eq!(pick_in(&reversed, os, arch, kind).as_deref(), Some(want));
        }
        let r = parse_latest(SAMPLE, "0.4.0", "linux", "x86_64", AppImage)
            .expect("parses")
            .expect("newer");
        assert_eq!(r.version, "0.5.0");
        assert!(r.page.ends_with("v0.5.0"));
    }

    /// With no file for this system, processor or install there is no
    /// download (the release page is offered instead), never another
    /// processor's or package's file.
    #[test]
    fn no_file_of_another_system_is_offered() {
        use Kind::*;
        // No builds for these processors and systems.
        assert_eq!(pick("windows", "aarch64", Setup), None);
        assert_eq!(pick("windows", "aarch64", Portable), None);
        assert_eq!(pick("linux", "riscv64", AppImage), None);
        assert_eq!(pick("linux", "x86", Tarball), None);
        assert_eq!(pick("freebsd", "x86_64", Tarball), None);
        assert_eq!(pick("web", "wasm32", Other), None);
        // No rpm or other system package is released.
        assert_eq!(pick("linux", "x86_64", Other), None);
        // A kind that belongs to another system.
        assert_eq!(pick("linux", "x86_64", Setup), None);
        assert_eq!(pick("macos", "aarch64", AppImage), None);
        assert_eq!(pick("windows", "x86_64", Dmg), None);
        // A release missing this processor's file offers none, not the
        // other processor's.
        let no_x86: String = SAMPLE
            .lines()
            .filter(|l| !l.contains("x86_64") && !l.contains("amd64"))
            .collect::<Vec<_>>()
            .join("\n");
        for kind in [AppImage, Tarball, Deb] {
            assert_eq!(pick_in(&no_x86, "linux", "x86_64", kind), None, "{kind:?}");
        }
        assert_eq!(pick_in(&no_x86, "macos", "x86_64", Dmg), None);
        let r = parse_latest(&no_x86, "0.4.0", "linux", "x86_64", AppImage)
            .expect("parses")
            .expect("newer");
        assert!(r.appimage.is_none(), "no install in place either");
        // Names that only end like this system's file do not count.
        let lookalike = SAMPLE.replace(
            "tracedraw-0.5.0-linux-x86_64.AppImage\", \"browser",
            "tracedraw-0.5.0-linux-x86_64.AppImage.zsync\", \"browser",
        );
        assert_eq!(pick_in(&lookalike, "linux", "x86_64", AppImage), None);
    }

    #[test]
    fn how_a_linux_copy_was_installed() {
        use std::path::Path;
        let list = "/.\n/usr\n/usr/bin\n/usr/bin/tracedraw\n/usr/bin/tracedraw-cli\n";
        let bin = Path::new("/usr/bin/tracedraw");
        let home = Path::new("/home/me/tracedraw/tracedraw");
        assert_eq!(linux_kind(true, false, bin, Some(list)), Kind::AppImage);
        assert_eq!(linux_kind(false, false, bin, Some(list)), Kind::Deb);
        assert_eq!(linux_kind(false, false, bin, None), Kind::Other, "rpm");
        assert_eq!(linux_kind(false, true, bin, Some(list)), Kind::Other);
        assert_eq!(linux_kind(false, false, home, Some(list)), Kind::Tarball);
        assert_eq!(linux_kind(false, false, home, None), Kind::Tarball);
        assert_eq!(
            linux_kind(
                false,
                false,
                Path::new("/usr/local/bin/tracedraw"),
                Some(list)
            ),
            Kind::Other
        );
    }

    #[test]
    fn the_same_or_older_drafts_and_pre_releases_are_not_offered() {
        assert_eq!(
            parse_latest(SAMPLE, "0.5.0", "linux", "x86_64", Kind::AppImage),
            Ok(None)
        );
        let pre = SAMPLE.replace("\"prerelease\": false", "\"prerelease\": true");
        assert_eq!(
            parse_latest(&pre, "0.4.0", "linux", "x86_64", Kind::AppImage),
            Ok(None)
        );
        let draft = SAMPLE.replace("\"draft\": false", "\"draft\": true");
        assert_eq!(
            parse_latest(&draft, "0.4.0", "linux", "x86_64", Kind::AppImage),
            Ok(None)
        );
        assert!(parse_latest("not json", "0.4.0", "linux", "x86_64", Kind::AppImage).is_err());
        assert!(parse_latest("{}", "0.4.0", "linux", "x86_64", Kind::AppImage).is_err());
    }

    #[test]
    fn the_appimage_comes_with_its_digest() {
        let r = parse_latest(SAMPLE, "0.4.0", "linux", "x86_64", Kind::AppImage)
            .expect("parses")
            .expect("newer");
        let p = r.appimage.expect("appimage");
        assert_eq!(p.url, "https://x/tracedraw-0.5.0-linux-x86_64.AppImage");
        assert_eq!(p.size, 5);
        assert_eq!(
            p.sha256,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        // Not on other systems, nor without a usable digest.
        let w = parse_latest(SAMPLE, "0.4.0", "windows", "x86_64", Kind::Setup).expect("parses");
        assert!(w.expect("newer").appimage.is_none());
        // Nor for a copy that is not an AppImage.
        for kind in [Kind::Tarball, Kind::Deb, Kind::Other] {
            let t = parse_latest(SAMPLE, "0.4.0", "linux", "x86_64", kind).expect("parses");
            assert!(t.expect("newer").appimage.is_none(), "{kind:?}");
        }
        // An arm64 AppImage installs the arm64 file, with its digest.
        let arm = parse_latest(SAMPLE, "0.4.0", "linux", "aarch64", Kind::AppImage)
            .expect("parses")
            .expect("newer")
            .appimage
            .expect("appimage");
        assert_eq!(arm.url, "https://x/tracedraw-0.5.0-linux-arm64.AppImage");
        assert_eq!(arm.size, 7);
        assert!(arm.sha256.starts_with("68e481c1"));
        let bad = SAMPLE.replace("sha256:2CF2", "md5:2CF2");
        let b = parse_latest(&bad, "0.4.0", "linux", "x86_64", Kind::AppImage).expect("parses");
        assert!(b.expect("newer").appimage.is_none());
    }

    #[test]
    fn installing_replaces_the_file_only_when_it_checks_out() {
        let dir = std::env::temp_dir().join(format!("td-install-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("TraceDraw.AppImage");
        std::fs::write(&target, b"old").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        let good = Package {
            url: String::new(),
            // sha256("hello")
            sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824".into(),
            size: 5,
        };
        let part = dir.join(".TraceDraw.AppImage.update");
        // A wrong digest, a short and a long download leave it alone.
        for (body, pkg) in [
            (&b"hellp"[..], good.clone()),
            (&b"hell"[..], good.clone()),
            (&b"hello!"[..], good.clone()),
        ] {
            let mut r = body;
            assert!(install_file(&mut r, &pkg, &target, &mut |_| {}).is_err());
            assert_eq!(std::fs::read(&target).expect("read"), b"old");
            assert!(!part.exists());
        }
        let mut seen = 0;
        let mut r = &b"hello"[..];
        install_file(&mut r, &good, &target, &mut |d| seen = d).expect("installs");
        assert_eq!(seen, 5);
        assert_eq!(std::fs::read(&target).expect("read"), b"hello");
        assert!(!part.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&target)
                .expect("meta")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o755);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Against GitHub: the latest release's x86_64 AppImage downloads and
    /// checks out. Run by hand (`cargo test -- --ignored`), it needs the network.
    #[test]
    #[ignore]
    fn live_install_from_the_latest_release() {
        let json = fetch().expect("latest release");
        let r = parse_latest(&json, "0.0.1", "linux", "x86_64", Kind::AppImage)
            .expect("parses")
            .expect("newer");
        let p = r.appimage.expect("appimage with digest");
        assert!(p.url.ends_with("-linux-x86_64.AppImage"), "{}", p.url);
        let dir = std::env::temp_dir().join(format!("td-live-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("TraceDraw.AppImage");
        std::fs::write(&target, b"old").expect("write");
        let installer = Installer::default();
        installer.start(r.version.clone(), p.clone(), target.clone(), None);
        for _ in 0..1200 {
            if !matches!(installer.get(), InstallState::Downloading { .. }) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert_eq!(installer.get(), InstallState::Installed(r.version));
        assert_eq!(std::fs::metadata(&target).expect("meta").len(), p.size);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
