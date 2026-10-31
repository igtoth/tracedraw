#!/bin/sh
# Install the Android app on the emulator or device adb sees, start it and
# check that it comes up and stays up (docs/behavior/android.md, CI job
# android-emulator):
#   scripts/android/emulator_check.sh <app.apk> [output folder]
# The app must still be running after TRACEDRAW_ANDROID_WAIT seconds (45
# by default) with no crash in the system log (a Java FATAL EXCEPTION, a
# native crash with its tombstone, a Rust panic, an ANR), its log must
# say it drew its first frame and loaded the system fonts, and its screen
# must show the editor, not a blank window (screen_check.py). A relaunch
# of the activity by the system (a resource overlay switched on, as the
# check then does on purpose) must bring the app back in a new process
# that draws again. The output folder (dist-android/emulator-check by
# default) gets the screenshot (screen.png), the whole system log
# (logcat.txt), the app's own lines (tracedraw.txt) and the crash log
# (crash.txt), also when the check fails; on GitHub Actions a failure
# also leaves its key log lines as annotations on the job.
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

# A line of the check's progress worth seeing without the job's log (a
# relaunch and what came of it): on GitHub Actions also a notice on the
# job.
note() {
    echo "emulator_check: $*"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then
        echo "::notice title=emulator check::$(echo "$*" | sed 's/%/%25/g')"
    fi
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
# On GitHub Actions a failed check leaves its key lines as annotations
# on the job (each one message, lines joined), readable without the
# artifacts.
annotate() {
    [ -s "$2" ] || return 0
    msg=$(head -n 40 "$2" | cut -c1-220 | sed 's/%/%25/g' | awk 'BEGIN{ORS="%0A"} {print}')
    echo "::error title=$1::$msg"
}
summary() {
    [ -n "${GITHUB_ACTIONS:-}" ] || return 0
    s="$out/summary"
    mkdir -p "$s"
    grep -hE "FATAL EXCEPTION|AndroidRuntime|Fatal signal|Abort message|Tombstone written|ANR in|panicked at|RustPanic| tracedraw *: (panic|ERROR)" \
        "$out/logcat.txt" "$out/crash.txt" 2>/dev/null | cut -c1-220 | head -n 40 > "$s/crash" || true
    grep -E " tracedraw *:" "$out/tracedraw.txt" 2>/dev/null | tail -n 40 > "$s/app" || true
    adb logcat -d -b events -v threadtime 2>/dev/null \
        | grep -E "_relaunch|_on_(create|destroy|stop)_called|am_proc_died|am_kill" \
        | grep -E "$pkg|TraceDrawActivity" | tail -n 20 > "$s/events" || true
    { python3 scripts/android/screen_check.py "$out/screen.png" 2>&1 || true
      sh_dev dumpsys window | grep -E 'mCurrentFocus|mFocusedApp' || true
      echo "pidof: $(sh_dev pidof "$pkg" || echo none)"; } > "$s/screen"
    # A process still there but not drawing: where its main thread waits.
    p=$(sh_dev pidof "$pkg" || true)
    if [ -n "$p" ] && adb root >/dev/null 2>&1; then
        adb wait-for-device
        sleep 2
        adb shell debuggerd -j "$p" 2>/dev/null | tr -d '\r' \
            | awk '/^"/{show=($0 ~ /^"main"/); if (show) print; next} show && /^  at /{print}' \
            | head -n 25 > "$s/main" || true
    fi
    annotate "crash lines" "$s/crash"
    annotate "the app's log (last lines)" "$s/app"
    annotate "activity events" "$s/events"
    annotate "screen focus and process" "$s/screen"
    annotate "main thread" "$s/main"
}
on_exit() {
    status=$?
    collect
    [ "$status" -eq 0 ] || summary
}
trap on_exit EXIT

# Whether the process $1 said in the log that the system relaunched the
# activity and it ends for that (TraceDrawActivity.onDestroy), and
# whether it logged its first frame.
said() {
    adb logcat -d -v threadtime 2>/dev/null \
        | awk -v p="$1" -v what="$2" '$3 == p && index($0, what) {found = 1} END {exit !found}'
}
ended_for_relaunch() {
    said "$1" "the activity is relaunched: the process ends"
}
drew() {
    said "$1" "first frame"
}
pid_now() {
    sh_dev pidof "$pkg" || true
}
# Whether the process $1 began a relaunch of its activity: its onStop
# for handleRelaunchActivity, in the event log (Android 11 and 14 alike).
# A process that did must end (TraceDrawActivity.onDestroy); one still
# there long after is frozen in NativeActivity.onDestroy.
relaunching() {
    adb logcat -d -b events -v threadtime 2>/dev/null \
        | awk -v p="$1" '$3 == p && /handleRelaunchActivity/ {found = 1} END {exit !found}'
}
# The app is not frozen in a relaunch: when its process began one, a new
# process follows within 20 s and is the one watched from then on.
not_frozen() {
    relaunching "$pid" || return 0
    now=$(new_pid "$pid" 20)
    [ -n "$now" ] || fail "the system relaunched the activity of pid $pid and it never ended: frozen"
    ended_for_relaunch "$pid" || fail "$pkg restarted: pid $pid, then $now"
    note "the system relaunched $pkg: pid $pid ended, pid $now started"
    pid=$now
}
# The one process of the app that is not $1, waited for up to $2 seconds
# (while the old one ends both may be listed); empty when none came.
new_pid() {
    w=0
    while :; do
        p=$(pid_now)
        case "$p" in
            "" | "$1" | *" "*) ;;
            *) echo "$p"; return 0 ;;
        esac
        [ "$w" -lt "$2" ] || return 0
        sleep 2
        w=$((w + 2))
    done
}

# Watch the app for $1 seconds, looking every five: it keeps running as
# the process $pid. The one change allowed is a relaunch: the system
# relaunched the activity, the app said so and ended its process, and the
# system started it again; the new process is then the one watched.
watch_app() {
    waited=0
    while [ "$waited" -lt "$1" ]; do
        sleep 5
        waited=$((waited + 5))
        now=$(pid_now)
        [ "$now" = "$pid" ] && continue
        if ended_for_relaunch "$pid"; then
            now=$(new_pid "$pid" 30)
            [ -n "$now" ] || fail "$pkg ended for a relaunch and was not started again"
            note "the system relaunched $pkg: pid $pid ended, pid $now started"
            pid=$now
            continue
        fi
        [ -n "$now" ] || fail "$pkg stopped $waited s after the start (it was pid $pid)"
        fail "$pkg restarted: pid $pid, then $now"
    done
    not_frozen
}

# Switch a resource overlay of the system on or off ($1: enable or
# disable): every app's assets change (configuration change 0x80000000),
# which relaunches the activity whatever its manifest declares. The app
# must come back in a new process and draw again within a minute. A
# frozen app keeps its process and, as the system keeps the old window
# for a relaunch, the focus: the event log tells whether the system
# relaunched the activity at all.
relaunch_with() {
    before=$pid
    sh_dev cmd overlay "$1" --user 0 "$overlay"
    t=0
    while :; do
        sleep 3
        t=$((t + 3))
        now=$(new_pid "$before" 0)
        if [ -n "$now" ] && drew "$now"; then
            break
        fi
        [ "$t" -lt 60 ] && continue
        if [ "$(pid_now)" = "$before" ] && ! relaunching "$before"; then
            note "$1 $overlay did not relaunch $pkg; the relaunch is not checked"
            return 0
        fi
        fail "$pkg did not come back and draw after a relaunch ($1 $overlay; pid $before, now $(pid_now))"
    done
    note "relaunch ($1 $overlay): pid $before ended, pid $now drew its first frame"
    pid=$now
}

adb wait-for-device
echo "device: Android $(sh_dev getprop ro.build.version.release) (API $(sh_dev getprop ro.build.version.sdk)), ABIs $(sh_dev getprop ro.product.cpu.abilist)"
adb install -r "$apk"
adb logcat -c || true
# Room for the whole run in the log buffer (the default drops the start).
adb logcat -b all -G 16M >/dev/null 2>&1 || adb logcat -G 16M >/dev/null 2>&1 || true

# The launcher activity, as the home screen starts it.
component=$(sh_dev cmd package resolve-activity --brief "$pkg" | tail -n 1)
case "$component" in
    "$pkg"/*) ;;
    *) fail "no launcher activity for $pkg (got: $component)" ;;
esac
sh_dev am start -W -n "$component"

# Up and staying up. Right after booting, the system may switch resource
# overlays and so relaunch the activity once (watch_app).
pid=$(pid_now)
t=0
while [ -z "$pid" ] && [ "$t" -lt 20 ]; do
    sleep 2
    t=$((t + 2))
    pid=$(pid_now)
done
[ -n "$pid" ] || fail "$pkg is not running after the start"
watch_app "$wait_s"
echo "$pkg is running as pid $pid after $wait_s s"

# A relaunch on purpose, and back: the first display cutout the emulator
# offers, switched on and off again.
overlay=$(sh_dev cmd overlay list android 2>/dev/null \
    | sed -n 's/^ *\[ \] *\(com\.android\.internal\.display\.cutout\.emulation\.[A-Za-z0-9_.]*\).*/\1/p' \
    | head -n 1)
if [ -n "$overlay" ]; then
    relaunch_with enable
    relaunch_with disable
    watch_app 10
    echo "$pkg came back from two relaunches and runs as pid $pid"
else
    note "no display cutout overlay to switch, the relaunch is not checked: $(sh_dev cmd overlay list android 2>&1 | head -n 12 | tr '\n' ' ')"
fi

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
