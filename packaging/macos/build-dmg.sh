#!/usr/bin/env bash
# Build TraceDraw.app and a DMG. Usage: build-dmg.sh <version> <release-dir> <arch-label>
# Unsigned: first launch needs right-click > Open (or xattr -d com.apple.quarantine).
set -euo pipefail
VERSION="$1"; BIN="$2"; ARCH="$3"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP="$ROOT/dist/TraceDraw.app"
rm -rf "$APP"; mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
sed "s/__VERSION__/$VERSION/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"
cp "$BIN/tracedraw" "$BIN/tracedraw-cli" "$APP/Contents/MacOS/"
chmod +x "$APP/Contents/MacOS/"*

# Icon: build .icns from the PNG set.
ICONSET="$ROOT/dist/tracedraw.iconset"; rm -rf "$ICONSET"; mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  cp "$ROOT/assets/icon/tracedraw-$s.png" "$ICONSET/icon_${s}x${s}.png"
  d=$((s*2)); cp "$ROOT/assets/icon/tracedraw-$d.png" "$ICONSET/icon_${s}x${s}@2x.png"
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/tracedraw.icns"

# Ad-hoc signature so Gatekeeper reports a consistent bundle.
codesign --force --deep --sign - "$APP"

DMG="$ROOT/dist/tracedraw-$VERSION-macos-$ARCH.dmg"
rm -f "$DMG"
STAGE="$ROOT/dist/dmg-stage"; rm -rf "$STAGE"; mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"; ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "TraceDraw" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
rm -rf "$STAGE" "$ICONSET"
echo "built $DMG"
