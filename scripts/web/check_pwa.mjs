#!/usr/bin/env node
// Checks a built browser site (./site from scripts/web/build.sh or CI):
// the module is a valid WebAssembly module, the page links the manifest
// and registers the service worker, the manifest is valid and its icons
// exist at the sizes it gives, and the service worker is valid JavaScript
// that lists files the site holds, with the build's placeholders filled
// in. Node only, no packages, no network. Exit code 1 and a message per
// problem when something is off.
//
//   node scripts/web/check_pwa.mjs site

import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import vm from "node:vm";

const dir = process.argv[2];
if (!dir) {
  console.error("usage: node scripts/web/check_pwa.mjs <site-dir>");
  process.exit(2);
}

const problems = new Set();
const problem = (text) => problems.add(text);
const read = (name) => readFileSync(join(dir, name));
const text = (name) => read(name).toString("utf8");
const need = (name) => {
  if (!existsSync(join(dir, name))) {
    problem(`${name} is missing`);
    return false;
  }
  return true;
};

// The module.
if (need("tracedraw_bg.wasm")) {
  const bytes = read("tracedraw_bg.wasm");
  if (!WebAssembly.validate(bytes)) {
    problem("tracedraw_bg.wasm is not a valid WebAssembly module");
  }
}
// wasm-bindgen writes the default export as `export default init` or as
// `export { initSync, __wbg_init as default }`, depending on the version.
if (need("tracedraw.js") && !/export default|as default\s*[},]/.test(text("tracedraw.js"))) {
  problem("tracedraw.js has no default export (the page's init)");
}

// The page.
let page = "";
if (need("index.html")) {
  page = text("index.html");
  for (const [what, needle] of [
    ["the manifest link", '<link rel="manifest" href="manifest.webmanifest">'],
    ["the theme colour", '<meta name="theme-color"'],
    ["viewport-fit=cover", "viewport-fit=cover"],
    ["the standalone flag", '<meta name="apple-mobile-web-app-capable" content="yes">'],
    ["the home-screen icon", '<link rel="apple-touch-icon" href="icon-192.png">'],
    ["the service worker registration", 'navigator.serviceWorker.register("sw.js")'],
    ["the module", "tracedraw_bg.wasm"],
  ]) {
    if (!page.includes(needle)) {
      problem(`index.html lacks ${what}: ${needle}`);
    }
  }
  if (page.includes("@VERSION@")) {
    problem("index.html still has @VERSION@");
  }
  if (/data-wasm-size="0"/.test(page)) {
    problem("index.html still has data-wasm-size=\"0\"");
  }
}

// A PNG's width and height from its IHDR chunk.
function pngSize(bytes) {
  const magic = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (bytes.length < 24 || magic.some((b, i) => bytes[i] !== b)) {
    return null;
  }
  return [bytes.readUInt32BE(16), bytes.readUInt32BE(20)];
}

// The manifest.
if (need("manifest.webmanifest")) {
  let manifest = null;
  try {
    manifest = JSON.parse(text("manifest.webmanifest"));
  } catch (e) {
    problem(`manifest.webmanifest is not JSON: ${e.message}`);
  }
  if (manifest) {
    for (const key of ["name", "short_name", "start_url", "scope", "display", "icons", "theme_color", "background_color"]) {
      if (!(key in manifest)) {
        problem(`manifest.webmanifest lacks ${key}`);
      }
    }
    if (manifest.display !== "standalone") {
      problem(`manifest.webmanifest display is ${manifest.display}, not standalone`);
    }
    for (const key of ["start_url", "scope"]) {
      if (typeof manifest[key] === "string" && manifest[key].startsWith("/")) {
        problem(`manifest.webmanifest ${key} is absolute (${manifest[key]}); the site is served under a path`);
      }
    }
    const icons = Array.isArray(manifest.icons) ? manifest.icons : [];
    const sizes = new Set();
    for (const icon of icons) {
      if (!need(icon.src)) {
        continue;
      }
      const size = pngSize(read(icon.src));
      const [w, h] = (icon.sizes || "").split("x").map(Number);
      sizes.add(w);
      if (!size) {
        problem(`${icon.src} is not a PNG`);
      } else if (size[0] !== w || size[1] !== h) {
        problem(`${icon.src} is ${size[0]}x${size[1]}, the manifest says ${icon.sizes}`);
      }
    }
    for (const wanted of [192, 512]) {
      if (!sizes.has(wanted)) {
        problem(`manifest.webmanifest has no ${wanted}x${wanted} icon, which installing needs`);
      }
    }
  }
}

// The service worker.
if (need("sw.js")) {
  const sw = text("sw.js");
  try {
    new vm.Script(sw, { filename: "sw.js" });
  } catch (e) {
    problem(`sw.js is not valid JavaScript: ${e.message}`);
  }
  for (const placeholder of ["@VERSION@", "@BUILD@"]) {
    if (sw.includes(placeholder)) {
      problem(`sw.js still has ${placeholder}`);
    }
  }
  const list = sw.match(/const SHELL = \[([^\]]*)\]/);
  if (!list) {
    problem("sw.js has no SHELL list");
  } else {
    const files = [...list[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
    for (const wanted of ["./", "index.html", "tracedraw.js", "tracedraw_bg.wasm", "manifest.webmanifest"]) {
      if (!files.includes(wanted)) {
        problem(`sw.js does not cache ${wanted}`);
      }
    }
    for (const file of files) {
      need(file === "./" ? "index.html" : file);
    }
    // The page's module is the one the worker keeps.
    if (page && !files.every((f) => f === "./" || !f.endsWith(".wasm") || page.includes(f))) {
      problem("sw.js caches a module the page does not load");
    }
  }
}

if (problems.size) {
  for (const p of problems) {
    console.error("check_pwa: " + p);
  }
  process.exit(1);
}
console.log(`check_pwa: ${dir} is a valid installable site`);
