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
    const BUILD_SCRIPT: &str = include_str!("../../../scripts/web/build.sh");
    const CI: &str = include_str!("../../../.github/workflows/build.yml");

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
            "init({ module_or_path: bytes })",
        ] {
            assert!(PAGE.contains(needed), "the page lacks {needed:?}");
        }
        // No word of a demo, and the house style.
        assert!(!PAGE.to_lowercase().contains("demo"));
        assert!(!PAGE.contains('\u{2014}'));
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

    /// Both builds put the interface languages the module does not embed
    /// where it fetches them (`i18n::file_url`).
    #[test]
    fn the_builds_ship_the_languages_the_module_fetches() {
        assert_eq!(crate::i18n::file_url("de").as_deref(), Some("lang/de.toml"));
        for (name, text) in [("scripts/web/build.sh", BUILD_SCRIPT), ("build.yml", CI)] {
            assert!(
                text.contains("mkdir -p site/lang")
                    && text.contains("for f in apps/tracedraw/lang/*.toml")
                    && text.contains("cp \"$f\" site/lang/"),
                "{name} does not ship the interface languages"
            );
        }
    }
}
