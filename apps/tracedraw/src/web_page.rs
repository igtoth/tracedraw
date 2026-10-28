//! The browser version's page (`web/index.html`): what the module needs
//! from it, and checks that the page and the build keep their parts. See
//! docs/behavior/web.md.

/// Text made safe to put into the page as HTML.
/// Whether a key pressed with Ctrl (or Cmd) on the canvas is left to the
/// browser: copy, cut and paste (egui needs their events) and the
/// developer tools (Ctrl+Shift+I). Everything else the editor binds,
/// Ctrl+R (Repeat) among them, is kept from the browser, which would
/// reload the page.
pub fn browser_keeps_key(key: &str, shift: bool) -> bool {
    let k = key.to_ascii_lowercase();
    matches!(k.as_str(), "c" | "v" | "x") || (shift && k == "i")
}

pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../../../web/index.html");
    const MANIFEST: &str = include_str!("../../../web/manifest.webmanifest");
    const WORKER: &str = include_str!("../../../web/sw.js");
    const BUILD_SCRIPT: &str = include_str!("../../../scripts/web/build.sh");
    const CI: &str = include_str!("../../../.github/workflows/build.yml");
    const CHECK: &str = include_str!("../../../scripts/web/check_pwa.mjs");

    /// Ctrl+R is Repeat in the editor, not the browser's reload; the
    /// clipboard keys and the developer tools stay with the browser.
    #[test]
    fn the_browser_keeps_only_the_clipboard_and_developer_keys() {
        assert!(browser_keeps_key("c", false) && browser_keeps_key("V", false));
        assert!(browser_keeps_key("x", true) && browser_keeps_key("I", true));
        for k in ["r", "R", "s", "p", "d", "g", "i", "e", "f"] {
            assert!(!browser_keeps_key(k, false), "{k}");
        }
    }

    #[test]
    fn text_is_escaped() {
        assert_eq!(
            escape_html("<b>\"a\" & 'b'</b>"),
            "&lt;b&gt;&quot;a&quot; &amp; &#39;b&#39;&lt;/b&gt;"
        );
        assert_eq!(escape_html("plain"), "plain");
    }

    /// The loading page shows the splash screen of the desktop app (name,
    /// tagline, status, progress bar, copyright and version), says what
    /// TraceDraw is, links to the downloads and the code, and has the
    /// elements the module and its loader use.
    #[test]
    fn the_loading_page_shows_the_splash_and_says_what_it_is() {
        for needed in [
            // The splash: its artwork's colours, the name, the tagline,
            // the footer.
            "id=\"splash\"",
            "#244475",
            "#1a3158",
            "#0e192d",
            "#5caeff",
            "Trace<tspan fill=\"#7fc3ff\">Draw</tspan>",
            "Vector illustration editor",
            "Ighor Toth and the TraceDraw contributors",
            "v@VERSION@",
            // What it is, how ready it is, what it is made of and its terms.
            "open-source vector editor that opens .cdr files",
            ">alpha<",
            "Written in Rust",
            "MIT or Apache-2.0",
            // Where to go when loading stalls.
            "https://github.com/igtoth/tracedraw/releases/latest",
            "https://github.com/igtoth/tracedraw\"",
            "https://github.com/igtoth/tracedraw/issues",
            // Which screens it is for.
            "desktop browser",
            "Window &gt; Workspace &gt; Touch",
            "phone",
            // The elements web.rs and the loader use.
            "id=\"tracedraw\"",
            "id=\"loading\"",
            "id=\"loading-status\"",
            "id=\"loading-bar\"",
            "id=\"loading-error\"",
            // The module's size for the progress, and the module itself.
            "data-wasm-size=\"0\"",
            "import init from \"./tracedraw.js\"",
            "tracedraw_bg.wasm",
            "init({ module_or_path: response })",
            // The module is compiled while it downloads, and fetched
            // together with the script.
            "new Response(counted",
            "application/wasm",
            "rel=\"modulepreload\"",
            "rel=\"preload\"",
        ] {
            assert!(PAGE.contains(needed), "the page lacks {needed:?}");
        }
        // No word of a demo, and the house style.
        assert!(!PAGE.to_lowercase().contains("demo"));
        assert!(!PAGE.contains('\u{2014}'));
    }

    /// The page can be installed as an app on a tablet or a desktop
    /// (#893): it links the manifest, carries the mobile tags and
    /// registers the service worker; the manifest is valid JSON, opens the
    /// page standalone at a relative address (the site is served under a
    /// path) with icons the repository holds at the sizes it gives; the
    /// worker keeps the page, the script, the module, the manifest and the
    /// icons, under a cache named after the build.
    #[test]
    fn the_page_is_installable_as_an_app() {
        for needed in [
            "viewport-fit=cover",
            "<link rel=\"manifest\" href=\"manifest.webmanifest\">",
            "<meta name=\"theme-color\" content=\"#1a3158\">",
            "<meta name=\"mobile-web-app-capable\" content=\"yes\">",
            "<meta name=\"apple-mobile-web-app-capable\" content=\"yes\">",
            "<link rel=\"apple-touch-icon\" href=\"icon-192.png\">",
            "navigator.serviceWorker.register(\"sw.js\")",
            // The editor keeps clear of notches and rounded corners.
            "env(safe-area-inset-top, 0px)",
        ] {
            assert!(PAGE.contains(needed), "the page lacks {needed:?}");
        }

        let manifest: serde_json::Value =
            serde_json::from_str(MANIFEST).expect("the manifest is JSON");
        assert_eq!(manifest["name"], "TraceDraw");
        assert_eq!(manifest["short_name"], "TraceDraw");
        assert_eq!(manifest["display"], "standalone");
        assert_eq!(manifest["orientation"], "any");
        assert_eq!(manifest["start_url"], "./");
        assert_eq!(manifest["scope"], "./");
        assert_eq!(manifest["theme_color"], "#1a3158");
        // The page's own background, so the start shows no flash.
        assert_eq!(manifest["background_color"], "#d4d4d4");
        assert!(PAGE.contains("background: #d4d4d4"));
        let icons = manifest["icons"].as_array().expect("icons");
        let mut sizes = Vec::new();
        for icon in icons {
            let src = icon["src"].as_str().expect("src");
            let size = icon["sizes"].as_str().expect("sizes");
            let n: u32 = size
                .split_once('x')
                .and_then(|(w, h)| (w == h).then(|| w.parse().ok()).flatten())
                .expect("square size");
            assert_eq!(src, format!("icon-{n}.png"));
            assert_eq!(icon["type"], "image/png");
            // The source file is in the repository and the builds copy it.
            let source = format!("assets/icon/tracedraw-{n}.png");
            let path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../{source}"));
            assert!(path.is_file(), "{source} is missing");
            for (name, text) in [("scripts/web/build.sh", BUILD_SCRIPT), ("build.yml", CI)] {
                assert!(
                    text.contains(&format!("{source} site/{src}")),
                    "{name} does not copy {src}"
                );
            }
            // The worker keeps it.
            assert!(
                WORKER.contains(&format!("\"{src}\"")),
                "sw.js does not keep {src}"
            );
            sizes.push(n);
        }
        // What installing asks for, and the icon the page itself uses.
        for wanted in [192, 256, 512] {
            assert!(sizes.contains(&wanted), "no {wanted} px icon");
        }

        // The worker: the build's placeholders in the cache name, the whole
        // shell in the list, same-deploy consistency (no skipWaiting, which
        // would hand a running page the next deploy's files).
        assert!(WORKER.contains("const VERSION = \"@VERSION@\""));
        assert!(WORKER.contains("const BUILD = \"@BUILD@\""));
        assert!(WORKER.contains("\"tracedraw-shell-\" + VERSION + \"-\" + BUILD"));
        for kept in [
            "\"./\"",
            "\"index.html\"",
            "\"tracedraw.js\"",
            "\"tracedraw_bg.wasm\"",
            "\"manifest.webmanifest\"",
        ] {
            assert!(WORKER.contains(kept), "sw.js does not keep {kept}");
        }
        assert!(WORKER.contains("cache.addAll("));
        assert!(!WORKER.contains("skipWaiting"));
        // A Help page is a page of its own, not the editor's shell.
        assert!(WORKER.contains("path === \"\" || path === \"index.html\""));
        assert!(!WORKER.contains("request.mode === \"navigate\""));
        for text in [MANIFEST, WORKER, CHECK] {
            assert!(!text.contains('\u{2014}'));
        }
    }

    /// Both builds put the app files beside the page: the manifest, the
    /// icons and the worker with the version and a digest of the page, the
    /// script and the module (taken after the page is written, so the
    /// digest counts the page served), then run the Node check on the site.
    #[test]
    fn the_builds_install_the_app_files() {
        for (name, text) in [("scripts/web/build.sh", BUILD_SCRIPT), ("build.yml", CI)] {
            assert!(text.contains("cp web/manifest.webmanifest site/"), "{name}");
            let page = text.find("web/index.html > site/index.html");
            let digest = text.find("build=$(cat site/index.html site/tracedraw.js site/tracedraw_bg.wasm site/manifest.webmanifest");
            assert!(
                matches!((page, digest), (Some(p), Some(d)) if p < d),
                "{name} does not digest the page, script and module after writing the page"
            );
            assert!(
                text.contains("-e \"s/@BUILD@/$build/\" web/sw.js > site/sw.js")
                    && text.contains("s/@VERSION@/"),
                "{name} does not fill in the worker's version and build"
            );
            assert!(
                text.contains("node scripts/web/check_pwa.mjs site"),
                "{name} does not check the site"
            );
        }
        // The check looks at what matters: the module, the links, the icons'
        // sizes, the worker's list, and leaves no placeholder behind.
        for needed in [
            "WebAssembly.validate(",
            "manifest.webmanifest",
            "pngSize(",
            "const SHELL = ",
            "@VERSION@",
            "@BUILD@",
            "process.exit(1)",
        ] {
            assert!(CHECK.contains(needed), "check_pwa.mjs lacks {needed:?}");
        }
        assert!(
            !CHECK.contains("import(") && !CHECK.contains("fetch("),
            "the check needs no network"
        );
    }

    /// Both builds of the page fill in the module's size and the version.
    #[test]
    fn the_builds_fill_in_the_modules_size_and_the_version() {
        for (name, text) in [("scripts/web/build.sh", BUILD_SCRIPT), ("build.yml", CI)] {
            assert!(
                text.contains("data-wasm-size=\\\"0\\\"") && text.contains("tracedraw_bg.wasm"),
                "{name} does not fill in the module's size"
            );
            assert!(
                text.contains("s/@VERSION@/"),
                "{name} does not fill in the version"
            );
        }
    }

    /// Both builds shrink the module with wasm-opt before its size goes
    /// into the page, so the progress bar counts the file served.
    #[test]
    fn the_builds_shrink_the_module_before_measuring_it() {
        for (name, text) in [("scripts/web/build.sh", BUILD_SCRIPT), ("build.yml", CI)] {
            let opt = text.find("wasm-opt -Oz --strip-debug --strip-producers");
            let size = text.find("size=$(wc -c < site/tracedraw_bg.wasm");
            assert!(
                matches!((opt, size), (Some(o), Some(s)) if o < s),
                "{name} does not run wasm-opt before measuring the module"
            );
        }
        assert!(CI.contains("BINARYEN: version_133") && CI.contains("sha256sum -c"));
    }
}
