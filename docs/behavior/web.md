# Browser version

The same editor compiled to WebAssembly (`wasm32-unknown-unknown`) and
drawn with WebGL 2 by eframe, so TraceDraw runs in a browser with nothing
to install. CI publishes it to GitHub Pages; it is not one of the
release downloads. Each run keeps the folder as the `web-site` artifact
(`tracedraw-<version>-web.zip`, a static folder: serve it from any web
server; opening `index.html` from disk does not work because browsers
refuse to load modules from `file://`).

## What differs from the desktop

| Feature | Desktop | Browser |
|---|---|---|
| Open, Import, scripts, palettes, CSV, ICC profiles, thesaurus | system file dialog | the browser's file picker; the file is read into memory under `/upload/<name>` and opened from there |
| Save, Save As | the Save Drawing dialog with folders, name, type, `.cdr` version and properties (`files.md`) | the same dialog without folders (name, type, version, properties), then a download under that name |
| Export, Publish to PDF, Save as Template, palette and script saves | system save dialog | a download named after the document |
| Print | PDF opened in the system viewer | PDF downloaded; print it from the browser's viewer |
| Send To | PDF written to Desktop, Documents or a mail folder | PDF downloaded |
| Edit Bitmap (external editor) | system image editor | not available (a browser cannot hand files to programs) |
| Settings | `settings.json` in the configuration folder, saved as they change | browser local storage (key `tracedraw.settings`), saved as they change: they stay in that browser on that computer, until its site data is cleared |
| Recent documents | listed on the Welcome screen | not recorded (uploads have no path to reopen) |
| Fonts | installed system fonts | bundled at build time: Liberation Sans, Serif and Mono, DejaVu Sans, Noto Sans Devanagari and Bengali; the published page's font folder adds about a hundred free families (stand-ins of the same widths for common office faces, popular families, Chinese, Japanese, Korean, Arabic, Hebrew and Thai) and its font catalog every other family of the google/fonts collection (about 2,000), each fetched the first time a drawing's text needs it or a font list's pointer rests on it (see `web/FONTS.md`); Chinese and Japanese UI text needs a font the build machine had |
| System clipboard | text, images and copied files (arboard) | text through the browser; pasting images or files from other programs is not available |
| Help links | system browser | a new tab |

## Loading

While the module downloads, the page shows the desktop app's splash
screen on the grey of the drawing window: the same artwork (`splash.rs`,
`ui/brand.rs`), name, tagline, copyright and version, with the status
"Loading TraceDraw: 3.5 of 5.7 MB (61%)" and the progress bar under it
(the build writes the module's size into the page as `data-wasm-size`
and the version for `@VERSION@`; without the size the page counts the
megabytes and the bar creeps towards 90 %). The download fills the bar
to 90 %, "Preparing the workspace..." the rest while the browser
finishes compiling the module; the splash goes when the editor is up.
The page starts the download of the module together with its script
(`modulepreload` and `preload`) and hands the bytes to the browser as they
arrive, in a response of the type `application/wasm`, so the browser
compiles the module while it downloads, not after: the wait after the
last byte is what is left of the compiling, and a server that sends the
module with another type makes no difference. Under the
splash, small: "TraceDraw (alpha): open-source vector editor that opens
.cdr files. Written in Rust, free under the MIT or Apache-2.0 license.",
which screens it suits (a desktop browser with a mouse and keyboard; on
a tablet the Touch workspace; a phone screen is too small for editing)
and links to the desktop downloads
(`https://github.com/igtoth/tracedraw/releases/latest`), the source
code and the issues. When the editor cannot start, the splash says so,
its bar turns red and the reason shows under it, with the links.

Once the editor is up, each time the page opens, a message says that
this is the same Rust program as the desktop app compiled to
WebAssembly, that a browser is slower (large drawings, heavy effects,
big files), has only the bundled fonts and opens and saves files by
upload and download, and that the desktop app runs natively at full
speed. OK closes it (Enter too); Download the desktop app opens the
latest release (`app::tests::the_browser_notice_shows_at_every_visit`).
Nothing is added to the menu bar, and the Welcome Screen's Get Started
page leaves out the recent documents a browser cannot keep.

Heavy work is slower than on the desktop because the build is optimised
for download size (`profile.web`, `opt-level = "s"`), runs on one thread
(effects, tracing, mosaics and large exports keep the page busy until
they finish) and a page has less memory than a program.

## Installing on a tablet

The page is an installable web app (#893): a browser that supports web
apps offers to put it on the home screen or in the app list, and from
there it opens full screen, without the browser's address bar and tabs,
with the TraceDraw icon and name, in any orientation. There is no native
Android or iPadOS program and no app store listing: the installed app is
the same WebAssembly build, run by the system's browser engine.

- Android tablet (Chrome, Edge, Samsung Internet): open
  `https://igtoth.github.io/tracedraw/`, then the browser's menu and
  "Install app" or "Add to Home screen". Chrome may also show an install
  bar under the address bar.
- iPad (Safari): open the page, tap Share, then "Add to Home Screen".
- Desktop (Chrome, Edge): the install icon at the right end of the
  address bar, or the menu's "Install TraceDraw".

Once opened, choose Window > Workspace > Touch for the larger interface
(`workspaces.md`). The installed app stays at the version it was last
opened with and starts without a network connection (the font folder's
families still need one the first time text uses them); a new deploy is
taken the next time the app starts after every TraceDraw page or window
has been closed.

How it works: `web/manifest.webmanifest` (name, icons of 192, 256 and
512 px from `assets/icon/`, standalone display, any orientation, the
page's grey as the background colour and the splash's dark blue as the
theme colour, `start_url` and `scope` relative because the site is served
under a path) is linked from the page, which also carries the mobile
tags (`viewport-fit=cover`, `theme-color`, `apple-mobile-web-app-capable`,
`apple-touch-icon`) and keeps the canvas clear of a notch or rounded
corners with the safe-area insets. `web/sw.js` is a service worker that
stores the app shell (the page, `tracedraw.js`, `tracedraw_bg.wasm`, the
manifest and the icons) in a cache named after the version and a digest
of the page, script and module that the build fills in; it stores all of
them or none, serves them from that cache only, keeps font folder files
in a cache of their own once fetched, and leaves every other address
(the Help pages, `FONTS.md`) to the network. A new deploy changes
`sw.js`, so the browser installs a new worker with a new cache and drops
the old one when it takes over; the new worker waits until the old one's
pages are closed, so a page that is up never gets the new deploy's
script with the old module or the other way round. The page registers
the worker after it has loaded, so the module's download comes first;
served from disk or over plain http away from localhost the page has no
worker and works as before.
Checks: `web_page::tests::the_page_is_installable_as_an_app`,
`web_page::tests::the_builds_install_the_app_files`, and on the built
site `scripts/web/check_pwa.mjs` (Node, no packages: the module
validates, the page links the manifest and registers the worker, the
manifest is JSON with icons of the sizes it gives, the worker is valid
JavaScript listing files the site holds, no placeholder left), run by CI
and by `scripts/web/build.sh` when Node is installed. Checked in
Chromium against a stand-in site served under a path: the worker caches
the shell with the module's `application/wasm` type, an offline reload
serves the page, script and module, a Help page is not replaced by the
shell, and a changed `sw.js` installs a waiting worker that takes over
with its own cache once the pages close.

## Mouse

A right click during a move, scale, rotation or skew with the Pick tool
leaves the original and gives the change to a copy, as on the desktop
(`pick-tool.md`). A browser reports a button pressed while another is
held as a pointer move, so the editor listens for the browser's own
mouse event (`web.rs`, `interaction::note_chorded_right_press`) and
takes it on the next frame; a second press takes the copy back. The
browser's context menu does not open on the canvas.

## Keyboard

The editor's shortcuts reach the editor instead of the browser: while
the canvas has the focus, Ctrl (or Cmd) with a letter and the function
keys F1 to F12 are not passed to the browser, except Ctrl+C, Ctrl+X and
Ctrl+V (clipboard) and Ctrl+Shift+I (developer tools). Ctrl+R is Repeat,
as on the desktop, and F5 the Freehand tool, so neither reloads the
page: use the browser's reload button
(`web_page::tests::the_browser_keeps_only_the_clipboard_and_developer_keys`).
Ctrl+N, Ctrl+T and Ctrl+W stay with the browser because no page can take
them; use the File menu for New and Close.

## Building

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # must match Cargo.lock
cargo build --target wasm32-unknown-unknown --profile web -p tracedraw
wasm-bindgen --target web --no-typescript --out-dir site \
    target/wasm32-unknown-unknown/web/tracedraw.wasm
# binaryen 133 (https://github.com/WebAssembly/binaryen/releases)
wasm-opt -Oz --strip-debug --strip-producers \
    -o site/tracedraw_bg.wasm site/tracedraw_bg.wasm
cp web/FONTS.md site/
size=$(wc -c < site/tracedraw_bg.wasm | tr -d ' ')
sed -e "s/data-wasm-size=\"0\"/data-wasm-size=\"$size\"/" -e "s/@VERSION@/0.4.0/" \
    web/index.html > site/index.html
cp assets/icon/tracedraw-256.png site/icon-256.png
# The installable app: manifest, icons and the service worker, whose
# cache is named after the version and a digest of the page, script and
# module (see "Installing on a tablet").
cp web/manifest.webmanifest site/
cp assets/icon/tracedraw-192.png site/icon-192.png
cp assets/icon/tracedraw-512.png site/icon-512.png
build=$(cat site/index.html site/tracedraw.js site/tracedraw_bg.wasm site/manifest.webmanifest | sha256sum | cut -c1-16)
sed -e "s/@VERSION@/0.4.0/" -e "s/@BUILD@/$build/" web/sw.js > site/sw.js
node scripts/web/check_pwa.mjs site   # optional: Node, no packages
python3 -m http.server -d site 8080   # then open http://localhost:8080
```

Checks: the page keeps the splash, what it says, its links and the
elements the module uses, and never says "demo"
(`web_page::tests::the_loading_page_shows_the_splash_and_says_what_it_is`);
both builds fill in the module's size and the version
(`web_page::tests::the_builds_fill_in_the_modules_size_and_the_version`), after
binaryen's `wasm-opt` has optimised the module for size and dropped its
name, debug and producer sections (version 133 in CI, from the release
checked against its SHA-256;
`web_page::tests::the_builds_shrink_the_module_before_measuring_it`).
Checked in a browser with a stand-in module: the progress, the start, an
error with the links kept, on a desktop and a phone-sized window.

`TRACEDRAW_WEB_FONTS` (folders separated by `:`) points the build at the
fonts to bundle; without it the usual Linux font folders are searched.

The font folder is optional: `python3 web/font_folder.py web/fonts.txt
site/fonts` builds it (downloading the google/fonts files at the commit
`web/fonts.txt` pins, or copying them from a checkout with
`--google-fonts DIR`; the TeX Gyre and DejaVu files come from the
`fonts-texgyre` and `fonts-dejavu-*` packages). It writes the files, a
`licences/` folder and `fonts.json`, which maps each family to its files.
When the page starts it reads `fonts/fonts.json`; the families join the
font lists, and text that needs one (set in it, set in a family it
stands in for, or holding letters only it has) asks for it: the page
fetches its files, the text engine loads them and the page draws again.
Without the folder the page works with the bundled fonts.
The font catalog: `python3 web/font_catalog.py web/fonts.txt site/fonts`
writes `fonts/catalog.json`, every family of the google/fonts collection
at the commit `web/fonts.txt` pins that the folder does not hold, with the
paths of its files (from each family's `METADATA.pb`, read through a
sparse fetch of just those files; `--google-fonts DIR` reads a checkout
instead). The page reads it with the folder's list and fetches a family's
files from `https://raw.githubusercontent.com/google/fonts/<commit>/`
the first time text needs it; a catalog under another address, or not
at a commit, is ignored, and so are paths that are not plain. Checks:
`font_folder::tests::the_catalog_reads_plain_paths_under_one_commit`,
`font_folder::tests::the_catalog_script_and_the_page_agree`.
`.cargo/config.toml` selects the browser's random number source for
`getrandom`. `scripts/web/build.sh` runs these steps.
