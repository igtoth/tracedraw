#!/bin/sh
# Build the browser version into ./site (see docs/behavior/web.md).
set -e
cd "$(dirname "$0")/../.."
cargo build --target wasm32-unknown-unknown --profile web -p tracedraw
rm -rf site && mkdir -p site
wasm-bindgen --target web --no-typescript --out-dir site \
    target/wasm32-unknown-unknown/web/tracedraw.wasm
cp web/index.html web/FONTS.md site/
cp assets/icon/tracedraw-256.png site/icon-256.png
echo "Built ./site; serve it with: python3 -m http.server -d site 8080"
