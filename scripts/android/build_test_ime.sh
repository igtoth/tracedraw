#!/bin/sh
# Build the emulator check's keyboard (scripts/android/test-ime): an input
# method that types what adb broadcasts to it, so that the check types
# into the app through the system's input method service as an on-screen
# keyboard does (scripts/android/emulator_check.sh). It is for the check
# only and never shipped:
#   scripts/android/build_test_ime.sh [output.apk]
# (dist-android/test/tracedraw-test-ime.apk by default). Needs what
# scripts/android/build.sh needs for its Java part: the Android build
# tools (aapt2, zipalign, apksigner, d8) in ANDROID_HOME or on PATH, the
# android.jar of platform 36 (or ANDROID_PLATFORM_JAR) and a JDK.
set -eu
cd "$(dirname "$0")/../.."

apk="${1:-dist-android/test/tracedraw-test-ime.apk}"
src=scripts/android/test-ime
work=dist-android/test/ime-build
rm -rf "$work" && mkdir -p "$work/classes" "$(dirname "$apk")"

bt=""
sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [ -n "$sdk" ]; then
    bt=$(ls -d "$sdk"/build-tools/*/ 2>/dev/null | sort -V | tail -n 1)
fi
tool() {
    if [ -n "$bt" ] && [ -x "$bt/$1" ]; then echo "$bt/$1"; else command -v "$1" || true; fi
}
AAPT2=$(tool aapt2); ZIPALIGN=$(tool zipalign); APKSIGNER=$(tool apksigner); D8=$(tool d8)
for t in "$AAPT2" "$ZIPALIGN" "$APKSIGNER" "$D8"; do
    [ -n "$t" ] || { echo "build_test_ime.sh: aapt2, zipalign, apksigner and d8 are needed" >&2; exit 1; }
done
jar="${ANDROID_PLATFORM_JAR:-}"
if [ -z "$jar" ] && [ -n "$sdk" ]; then
    jar="$sdk/platforms/android-36/android.jar"
fi
[ -f "$jar" ] || { echo "build_test_ime.sh: no android.jar (ANDROID_PLATFORM_JAR)" >&2; exit 1; }

javac -source 8 -target 8 -Xlint:-options -encoding UTF-8 -bootclasspath "$jar" \
    -d "$work/classes" $(find "$src/java" -name '*.java')
"$D8" --release --min-api 24 --lib "$jar" --output "$work" \
    $(find "$work/classes" -name '*.class')
"$AAPT2" compile --dir "$src/res" -o "$work/res.zip"
"$AAPT2" link -o "$work/unsigned.apk" -I "$jar" --manifest "$src/AndroidManifest.xml" \
    --min-sdk-version 24 --target-sdk-version 34 "$work/res.zip"
(cd "$work" && zip -q unsigned.apk classes.dex)
"$ZIPALIGN" -f -p 4 "$work/unsigned.apk" "$work/aligned.apk"
# The debug key scripts/android/build.sh makes (or another one).
ks="${ANDROID_DEBUG_KEYSTORE:-$HOME/.android/debug.keystore}"
if [ ! -f "$ks" ]; then
    mkdir -p "$(dirname "$ks")"
    keytool -genkeypair -keystore "$ks" -storepass android -keypass android \
        -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 \
        -dname "CN=Android Debug,O=Android,C=US" >/dev/null 2>&1
fi
"$APKSIGNER" sign --ks "$ks" --ks-pass pass:android --key-pass pass:android \
    --ks-key-alias androiddebugkey --min-sdk-version 24 --out "$apk" "$work/aligned.apk"
rm -rf "$work"
echo "Built $apk"
