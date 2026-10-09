#!/bin/bash
# usage: run.sh <script.py> [app args]
# Runs the debug binary under Xvfb, drives it with the script (see
# drive.py for click/drag/key/shot) and reports the app's exit status.
# Screenshots go to $TRACEDRAW_SHOTS (default /tmp/tracedraw-visual).
set -e
cd "$(dirname "$0")/../.."
export DISPLAY=:99
export TRACEDRAW_SHOTS="${TRACEDRAW_SHOTS:-/tmp/tracedraw-visual}"
mkdir -p "$TRACEDRAW_SHOTS"
LOG="$TRACEDRAW_SHOTS/app.log"
Xvfb :99 -screen 0 1600x1000x24 >/dev/null 2>&1 & XPID=$!
sleep 1
SCRIPT=$1; shift
(WINIT_UNIX_BACKEND=x11 TRACEDRAW_CONFIG_DIR="$TRACEDRAW_SHOTS/config" LIBGL_ALWAYS_SOFTWARE=1 ./target/debug/tracedraw "$@" >"$LOG" 2>&1; echo "APP EXIT $?" >> "$LOG") &
sleep 6
python3 "$(dirname "$0")/drive.py" "$SCRIPT" || true
sleep 1
grep "APP EXIT" "$LOG" || echo "app still running"
grep -i "panic" "$LOG" && echo "PANIC in log" || true
pkill -x tracedraw 2>/dev/null || true; sleep 0.5; kill $XPID 2>/dev/null || true
