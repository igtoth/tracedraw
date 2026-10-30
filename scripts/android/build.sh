#!/bin/sh
# Build the Android app, a debug-signed APK for 64-bit ARM tablets and
# phones, into ./dist-android (see docs/behavior/android.md).
#
# Needs: the Rust target (rustup target add aarch64-linux-android), an
# Android NDK (r26 or later) in ANDROID_NDK_HOME, the Android build tools
# (aapt2, zipalign, apksigner: an Android SDK in ANDROID_HOME, or the
# android-sdk-build-tools and apksigner packages), an android.jar (the
# SDK's newest platform, ANDROID_PLATFORM_JAR, or the android-sdk-platform-23
# package) and keytool from a JDK for the debug key.
set -e
cd "$(dirname "$0")/../.."

NDK="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-${ANDROID_NDK:-}}}"
if [ -z "$NDK" ] || [ ! -d "$NDK/toolchains/llvm/prebuilt" ]; then
    echo "scripts/android/build.sh: set ANDROID_NDK_HOME to an Android NDK (r26 or later)" >&2
    exit 1
fi
# The NDK's toolchain for this build machine.
host=$(ls "$NDK/toolchains/llvm/prebuilt" | head -n 1)
TC="$NDK/toolchains/llvm/prebuilt/$host/bin"
# The lowest Android version the app runs on (Android 7.0); the manifest
# says the same.
API=24
TARGET=aarch64-linux-android
ABI=arm64-v8a

# The compiler and linker for the Rust target and for the C code of the
# dependencies (cc).
export CC_aarch64_linux_android="$TC/${TARGET}${API}-clang"
export CXX_aarch64_linux_android="$TC/${TARGET}${API}-clang++"
export AR_aarch64_linux_android="$TC/llvm-ar"
export RANLIB_aarch64_linux_android="$TC/llvm-ranlib"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TC/${TARGET}${API}-clang"
export ANDROID_NDK_HOME="$NDK"
export ANDROID_NDK_ROOT="$NDK"

# The editor as a shared library (the crate is a library plus a thin
# program; the library's type is chosen here so the desktop build stays
# an ordinary binary).
cargo rustc -p tracedraw --lib --release --target "$TARGET" --crate-type cdylib
so="${CARGO_TARGET_DIR:-target}/$TARGET/release/libtracedraw.so"
[ -f "$so" ] || { echo "no $so" >&2; exit 1; }

# The build tools: an SDK's newest build-tools folder, else the ones on PATH.
bt=""
if [ -n "${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}" ]; then
    sdk="${ANDROID_HOME:-$ANDROID_SDK_ROOT}"
    bt=$(ls -d "$sdk"/build-tools/*/ 2>/dev/null | sort -V | tail -n 1)
fi
tool() {
    if [ -n "$bt" ] && [ -x "$bt/$1" ]; then echo "$bt/$1"; else command -v "$1"; fi
}
AAPT2=$(tool aapt2); ZIPALIGN=$(tool zipalign); APKSIGNER=$(tool apksigner)
for t in "$AAPT2" "$ZIPALIGN" "$APKSIGNER"; do
    [ -n "$t" ] || { echo "scripts/android/build.sh: aapt2, zipalign and apksigner are needed (Android build tools)" >&2; exit 1; }
done
# The framework's resource table the manifest's attributes resolve against.
jar="${ANDROID_PLATFORM_JAR:-}"
if [ -z "$jar" ] && [ -n "${sdk:-}" ]; then
    jar=$(ls "$sdk"/platforms/android-*/android.jar 2>/dev/null | sort -V | tail -n 1)
fi
[ -n "$jar" ] || jar=/usr/lib/android-sdk/platforms/android-23/android.jar
[ -f "$jar" ] || { echo "scripts/android/build.sh: no android.jar (ANDROID_PLATFORM_JAR)" >&2; exit 1; }

# The version from Cargo.toml: 0.19.10 is version code 19010.
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
code=$(echo "$version" | awk -F. '{ printf "%d", $1 * 1000000 + $2 * 1000 + $3 }')

out=dist-android
rm -rf "$out/build" && mkdir -p "$out/build/lib/$ABI"
# The library, without its symbol table (a third smaller).
"$TC/llvm-strip" --strip-unneeded -o "$out/build/lib/$ABI/libtracedraw.so" "$so"
# The icons and the manifest into an APK, then the library beside them.
"$AAPT2" compile --dir packaging/android/res -o "$out/build/res.zip"
"$AAPT2" link -o "$out/build/unsigned.apk" -I "$jar" \
    --manifest packaging/android/AndroidManifest.xml \
    --version-code "$code" --version-name "$version" \
    "$out/build/res.zip"
(cd "$out/build" && zip -q -r unsigned.apk lib)
"$ZIPALIGN" -f -p 4 "$out/build/unsigned.apk" "$out/build/aligned.apk"

# A debug key, as the Android tools make for development builds: the APK
# installs on a device with "install from unknown sources" and is not for
# a store. ANDROID_DEBUG_KEYSTORE points at another keystore.
ks="${ANDROID_DEBUG_KEYSTORE:-$HOME/.android/debug.keystore}"
if [ ! -f "$ks" ]; then
    mkdir -p "$(dirname "$ks")"
    keytool -genkeypair -keystore "$ks" -storepass android -keypass android \
        -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 \
        -dname "CN=Android Debug,O=Android,C=US" >/dev/null 2>&1
fi
apk="$out/tracedraw-$version-android-$ABI-debug.apk"
"$APKSIGNER" sign --ks "$ks" --ks-pass pass:android --key-pass pass:android \
    --ks-key-alias androiddebugkey --min-sdk-version "$API" --out "$apk" "$out/build/aligned.apk"
"$APKSIGNER" verify --min-sdk-version "$API" "$apk"
echo "Built $apk; install it with: adb install -r $apk"
