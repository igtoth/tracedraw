#!/usr/bin/env bash
# Build an AppImage. Usage: build-appimage.sh <version> <release-dir> [x86_64|arm64]
set -euo pipefail
VERSION="$1"; BIN="$2"; ARCH="${3:-x86_64}"
# linuxdeploy names the machine its own way. The x86_64 file is named
# amd64 so that it sorts before arm64 in a release's file list: updaters
# of 0.19.8 and earlier take the first AppImage listed (#1038).
case "$ARCH" in arm64) TOOL_ARCH=aarch64; NAME=arm64 ;; *) TOOL_ARCH=x86_64; NAME=amd64 ;; esac
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DIR="$ROOT/dist/AppDir"; rm -rf "$DIR"
mkdir -p "$DIR/usr/bin" "$DIR/usr/share/applications" "$DIR/usr/share/icons/hicolor/256x256/apps"
cp "$BIN/tracedraw" "$BIN/tracedraw-cli" "$DIR/usr/bin/"
cp "$ROOT/packaging/linux/tracedraw.desktop" "$DIR/usr/share/applications/"
cp "$ROOT/assets/icon/tracedraw-256.png" "$DIR/usr/share/icons/hicolor/256x256/apps/tracedraw.png"
TOOL="$ROOT/dist/linuxdeploy.AppImage"
[ -f "$TOOL" ] || curl -sSL -o "$TOOL" https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$TOOL_ARCH.AppImage
chmod +x "$TOOL"
cd "$ROOT/dist"
OUT="tracedraw-$VERSION-linux-$NAME.AppImage"
VERSION="$VERSION" ARCH="$TOOL_ARCH" OUTPUT="$OUT" "$TOOL" --appimage-extract-and-run \
  --appdir "$DIR" --desktop-file "$DIR/usr/share/applications/tracedraw.desktop" \
  --icon-file "$DIR/usr/share/icons/hicolor/256x256/apps/tracedraw.png" --output appimage
rm -rf "$DIR"
