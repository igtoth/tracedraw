#!/bin/sh
# Install the Android app on the emulator or device adb sees, start it and
# check that it comes up and stays up (docs/behavior/android.md, CI job
# android-emulator):
#   scripts/android/emulator_check.sh <app.apk> [output folder]
# The app must still be running after TRACEDRAW_ANDROID_WAIT seconds (45
# by default) with no crash in the system log (a Java FATAL EXCEPTION, a
# native crash with its tombstone, a Rust panic, an ANR), its log must
# say it drew its first frame and loaded the system fonts, and its screen
# must show the editor, not a blank window (screen_check.py). The output
# folder (dist-android/emulator-check by default) gets the screenshot
# (screen.png), the whole system log (logcat.txt), the app's own lines
# (tracedraw.txt) and the crash log (crash.txt), also when the check fails.
set -eu
cd "$(dirname "$0")/../.."

apk="${1:?usage: scripts/android/emulator_check.sh <app.apk> [output folder]}"
out="${2:-dist-android/emulator-check}"
wait_s="${TRACEDRAW_ANDROID_WAIT:-45}"
pkg=dev.tracedraw.app
mkdir -p "$out"

fail() {
    echo "emulator_check: $*" >&2
    exit 1
}

# What adb prints from the device ends lines with \r.
sh_dev() {
    adb shell "$@" | tr -d '\r'
}

# The logs and the picture, whatever the outcome.
collect() {
    adb exec-out screencap -p > "$out/screen.png" 2>/dev/null || true
    adb logcat -d -v threadtime > "$out/logcat.txt" 2>/dev/null || true
    adb logcat -d -b crash -v threadtime > "$out/crash.txt" 2>/dev/null || true
    # Every line the app's processes wrote (each start is a new pid), and
    # the system's lines about the app.
    pids=$(sed -n "s/.*Start proc \([0-9]*\):$pkg\/.*/\1/p" "$out/logcat.txt" | tr '\n' ' ')
    awk -v pids=" $pids " -v pkg="$pkg" \
        'index(pids, " " $3 " ") || index($0, pkg) || / tracedraw *:/ || /RustPanic/' \
        "$out/logcat.txt" > "$out/tracedraw.txt" || true
}
trap collect EXIT

adb wait-for-device
echo "device: Android $(sh_dev getprop ro.build.version.release) (API $(sh_dev getprop ro.build.version.sdk)), ABIs $(sh_dev getprop ro.product.cpu.abilist)"
adb install -r "$apk"
adb logcat -c || true

# The launcher activity, as the home screen starts it.
component=$(sh_dev cmd package resolve-activity --brief "$pkg" | tail -n 1)
case "$component" in
    "$pkg"/*) ;;
    *) fail "no launcher activity for $pkg (got: $component)" ;;
esac
sh_dev am start -W -n "$component"

# Up and staying up: the process is looked for every five seconds.
pid=$(sh_dev pidof "$pkg" || true)
[ -n "$pid" ] || fail "$pkg is not running after the start"
waited=0
while [ "$waited" -lt "$wait_s" ]; do
    sleep 5
    waited=$((waited + 5))
    now=$(sh_dev pidof "$pkg" || true)
    [ -n "$now" ] || fail "$pkg stopped $waited s after the start (it was pid $pid)"
    [ "$now" = "$pid" ] || fail "$pkg restarted: pid $pid, then $now"
done
echo "$pkg is running as pid $pid after $wait_s s"

# In front, not behind an error or "isn't responding" dialog.
sh_dev dumpsys window | grep -E 'mCurrentFocus|mFocusedApp' > "$out/focus.txt" || true
cat "$out/focus.txt"
grep -q "$pkg" "$out/focus.txt" || fail "$pkg does not have the focus"

collect
crashes=$(grep -E "FATAL EXCEPTION|Fatal signal|\*\*\* \*\*\* \*\*\*|Tombstone written|ANR in $pkg|panicked at|RustPanic| tracedraw *: panic:" \
    "$out/logcat.txt" "$out/crash.txt" || true)
if [ -n "$crashes" ]; then
    echo "$crashes" >&2
    fail "the system log has a crash"
fi
# The editor's own word that it drew (frame_stats logs its first frame),
# found the system fonts, and the picture of it.
grep -q " tracedraw *: .*first frame" "$out/logcat.txt" || fail "the editor never drew a frame"
if grep -q " tracedraw *: .*loaded 0 font faces" "$out/logcat.txt"; then
    fail "the editor found no system fonts"
fi
python3 scripts/android/screen_check.py "$out/screen.png" || fail "the screen does not show the editor"
echo "emulator_check: $pkg started, stayed up for $wait_s s and drew its window"
