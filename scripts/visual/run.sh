#!/bin/bash
# usage: run_drive.sh <script.py> [app args]
set -e
cd "$(dirname "$0")/../.."
export DISPLAY=:99
Xvfb :99 -screen 0 1600x1000x24 >/dev/null 2>&1 & XPID=$!
sleep 1
SCRIPT=$1; shift
WINIT_UNIX_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 ./target/debug/tracedraw "$@" >/tmp/claude-0/app.log 2>&1 & APID=$!
sleep 6
python3 "$(dirname "$0")/drive.py" "$SCRIPT" || true
kill $APID 2>/dev/null || true; sleep 0.5; kill $XPID 2>/dev/null || true
