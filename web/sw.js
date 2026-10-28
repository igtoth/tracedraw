// The browser version's service worker: keeps the app shell (the page,
// the script, the WebAssembly module, the manifest and the icons) so the
// editor starts without a network once it has been opened once, and so
// the page can be installed on a tablet or a desktop as an app
// (docs/behavior/web.md, "Installing on a tablet").
//
// The build fills in @VERSION@ and @BUILD@ (a digest of the page, the
// script and the module; scripts/web/build.sh and CI). A new deploy
// changes this file, so the browser installs a new worker with a new
// cache, filled in one go from the deploy's own files: the page, script
// and module served to one visit always come from the same deploy, and
// a stale module is never served against a new script. The new worker
// waits until every TraceDraw page of the old one has closed, so a page
// that is up keeps its own files; the next start takes the new deploy.
const VERSION = "@VERSION@";
const BUILD = "@BUILD@";
const SHELL_CACHE = "tracedraw-shell-" + VERSION + "-" + BUILD;
// Fonts the page fetches from its font folder the first time text needs
// them (web/FONTS.md): kept across versions, their files are pinned.
const FONT_CACHE = "tracedraw-fonts";
// Relative to this file, which sits beside index.html.
const SHELL = [
  "./",
  "index.html",
  "tracedraw.js",
  "tracedraw_bg.wasm",
  "manifest.webmanifest",
  "icon-192.png",
  "icon-256.png",
  "icon-512.png",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    (async () => {
      const cache = await caches.open(SHELL_CACHE);
      // cache: "reload" skips the browser's HTTP cache, so the shell is
      // the deploy's current files; addAll stores all of them or none.
      await cache.addAll(SHELL.map((path) => new Request(path, { cache: "reload" })));
    })()
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const names = await caches.keys();
      await Promise.all(
        names
          .filter((name) => name.startsWith("tracedraw-shell-") && name !== SHELL_CACHE)
          .map((name) => caches.delete(name))
      );
      await self.clients.claim();
    })()
  );
});

// The shell: from this worker's cache, the network only when the cache
// lacks the file (it cannot, the install stores all or none).
async function fromShell(path) {
  const cache = await caches.open(SHELL_CACHE);
  const hit = await cache.match(path);
  return hit || fetch(path);
}

// A font file: from the font cache, else fetched and kept.
async function fontFile(request) {
  const cache = await caches.open(FONT_CACHE);
  const hit = await cache.match(request);
  if (hit) {
    return hit;
  }
  const response = await fetch(request);
  if (response.ok && response.status === 200) {
    await cache.put(request, response.clone());
  }
  return response;
}

// The font folder's lists (fonts.json, catalog.json): the network's copy,
// kept for a start without one.
async function fontList(request) {
  const cache = await caches.open(FONT_CACHE);
  try {
    const response = await fetch(request);
    if (response.ok && response.status === 200) {
      await cache.put(request, response.clone());
    }
    return response;
  } catch (e) {
    const hit = await cache.match(request);
    if (hit) {
      return hit;
    }
    throw e;
  }
}

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") {
    return;
  }
  const url = new URL(request.url);
  const base = new URL("./", self.location.href);
  if (url.origin !== base.origin || !url.pathname.startsWith(base.pathname)) {
    return;
  }
  const path = url.pathname.slice(base.pathname.length);
  // Only the editor's own page: a Help page under help/ is a page of
  // its own and goes to the network below.
  if (path === "" || path === "index.html") {
    event.respondWith(fromShell("./"));
  } else if (SHELL.includes(path)) {
    event.respondWith(fromShell(path));
  } else if (path.startsWith("fonts/")) {
    event.respondWith(path.endsWith(".json") ? fontList(request) : fontFile(request));
  }
  // Everything else (the Help pages, FONTS.md) goes to the network as usual.
});
