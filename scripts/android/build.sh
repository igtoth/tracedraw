#!/bin/sh
# Build the Android app for 64-bit ARM tablets and phones into
# ./dist-android (see docs/behavior/android.md and docs/android-play.md):
# a debug-signed APK per ABI to install by hand, and an Android App
# Bundle (.aab) with every ABI for Google Play, signed with the upload key
# when the release signing variables are set and with the debug key
# otherwise.
#
# ANDROID_ABIS lists the ABIs: arm64-v8a (64-bit ARM, the default) and
# x86_64 (Chromebooks with Intel or AMD processors, and the emulator),
# e.g. ANDROID_ABIS="arm64-v8a x86_64".
#
# Needs: the Rust targets (rustup target add aarch64-linux-android, and
# x86_64-linux-android for x86_64), an
# Android NDK (r26 or later) in ANDROID_NDK_HOME, the Android build tools
# (aapt2, zipalign, apksigner, d8: an Android SDK in ANDROID_HOME, or the
# packages on PATH), the android.jar of platform 36 (the SDK's
# platforms/android-36, or ANDROID_PLATFORM_JAR), a JDK (javac, keytool,
# jarsigner) and bundletool (BUNDLETOOL_JAR, `bundletool` on PATH, or it
# is downloaded once into dist-android/tools).
#
# Release signing (Play App Signing, the upload key): set
#   ANDROID_KEYSTORE_BASE64  the upload keystore (.jks or .p12), base64
#   ANDROID_KEYSTORE_PASSWORD, ANDROID_KEY_ALIAS, ANDROID_KEY_PASSWORD
# or ANDROID_KEYSTORE with the path of the keystore file instead of the
# base64 text. Without them the bundle is signed with the debug key and
# is for testing only (Play rejects it).
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
# says the same. PLATFORM is the version the app is built for
# (targetSdkVersion in the manifest) and the android.jar it links against.
API=24
PLATFORM=36
ABIS="${ANDROID_ABIS:-arm64-v8a}"
# The Rust target of an ABI.
rust_target() {
    case "$1" in
        arm64-v8a) echo aarch64-linux-android ;;
        x86_64) echo x86_64-linux-android ;;
        *) echo "scripts/android/build.sh: unknown ABI $1 (arm64-v8a, x86_64)" >&2; return 1 ;;
    esac
}
for ABI in $ABIS; do rust_target "$ABI" >/dev/null || exit 1; done
export ANDROID_NDK_HOME="$NDK"
export ANDROID_NDK_ROOT="$NDK"

out=dist-android
rm -rf "$out/build" && mkdir -p "$out/build/classes" "$out/tools"
for ABI in $ABIS; do
    TARGET=$(rust_target "$ABI")
    # The compiler and linker for the Rust target and for the C code of
    # the dependencies (cc), from the NDK.
    t=$(echo "$TARGET" | tr - _)
    T=$(echo "$t" | tr '[:lower:]' '[:upper:]')
    export "CC_$t=$TC/${TARGET}${API}-clang"
    export "CXX_$t=$TC/${TARGET}${API}-clang++"
    export "AR_$t=$TC/llvm-ar"
    export "RANLIB_$t=$TC/llvm-ranlib"
    export "CARGO_TARGET_${T}_LINKER=$TC/${TARGET}${API}-clang"
    # The editor as a shared library (the crate is a library plus a thin
    # program; the library's type is chosen here so the desktop build
    # stays an ordinary binary).
    cargo rustc -p tracedraw --lib --release --target "$TARGET" --crate-type cdylib
    so="${CARGO_TARGET_DIR:-target}/$TARGET/release/libtracedraw.so"
    [ -f "$so" ] || { echo "no $so" >&2; exit 1; }
    # The library, without its symbol table (a third smaller).
    mkdir -p "$out/build/lib/$ABI"
    "$TC/llvm-strip" --strip-unneeded -o "$out/build/lib/$ABI/libtracedraw.so" "$so"
done

# The build tools: an SDK's newest build-tools folder, else the ones on PATH.
bt=""
if [ -n "${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}" ]; then
    sdk="${ANDROID_HOME:-$ANDROID_SDK_ROOT}"
    bt=$(ls -d "$sdk"/build-tools/*/ 2>/dev/null | sort -V | tail -n 1)
fi
tool() {
    if [ -n "$bt" ] && [ -x "$bt/$1" ]; then echo "$bt/$1"; else command -v "$1"; fi
}
AAPT2=$(tool aapt2); ZIPALIGN=$(tool zipalign); APKSIGNER=$(tool apksigner); D8=$(tool d8)
for t in "$AAPT2" "$ZIPALIGN" "$APKSIGNER" "$D8"; do
    [ -n "$t" ] || { echo "scripts/android/build.sh: aapt2, zipalign, apksigner and d8 are needed (Android build tools)" >&2; exit 1; }
done
for t in javac keytool jarsigner; do
    command -v "$t" >/dev/null || { echo "scripts/android/build.sh: $t is needed (a JDK)" >&2; exit 1; }
done
# The framework the manifest's attributes resolve against and the Java
# code compiles against: the platform the app targets.
jar="${ANDROID_PLATFORM_JAR:-}"
if [ -z "$jar" ] && [ -n "${sdk:-}" ]; then
    jar="$sdk/platforms/android-$PLATFORM/android.jar"
fi
[ -f "$jar" ] || { echo "scripts/android/build.sh: no android.jar for platform $PLATFORM (ANDROID_PLATFORM_JAR)" >&2; exit 1; }

# The version from Cargo.toml: 0.19.10 is version code 19010.
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
code=$(echo "$version" | awk -F. '{ printf "%d", $1 * 1000000 + $2 * 1000 + $3 }')

# The activity (packaging/android/java): Java 8 class files, then one
# classes.dex for API 24 and later.
javac -source 8 -target 8 -Xlint:-options -encoding UTF-8 -bootclasspath "$jar" \
    -d "$out/build/classes" $(find packaging/android/java -name '*.java')
"$D8" --release --min-api "$API" --lib "$jar" --output "$out/build" \
    $(find "$out/build/classes" -name '*.class')
[ -f "$out/build/classes.dex" ] || { echo "no classes.dex" >&2; exit 1; }
# The icons and the manifest.
"$AAPT2" compile --dir packaging/android/res -o "$out/build/res.zip"
link() {
    # $1: the output, then the extra flags (--proto-format for the bundle).
    o="$1"; shift
    "$AAPT2" link -o "$o" -I "$jar" \
        --manifest packaging/android/AndroidManifest.xml \
        --min-sdk-version "$API" --target-sdk-version "$PLATFORM" \
        --version-code "$code" --version-name "$version" \
        "$@" "$out/build/res.zip"
}

# ----- the APKs: resources, the code, one ABI's library; aligned; debug-signed -----
link "$out/build/resources.apk"

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
for ABI in $ABIS; do
    cp "$out/build/resources.apk" "$out/build/unsigned.apk"
    (cd "$out/build" && zip -q -r unsigned.apk "lib/$ABI" classes.dex)
    "$ZIPALIGN" -f -p 4 "$out/build/unsigned.apk" "$out/build/aligned.apk"
    apk="$out/tracedraw-$version-android-$ABI-debug.apk"
    "$APKSIGNER" sign --ks "$ks" --ks-pass pass:android --key-pass pass:android \
        --ks-key-alias androiddebugkey --min-sdk-version "$API" --out "$apk" "$out/build/aligned.apk"
    "$APKSIGNER" verify --min-sdk-version "$API" "$apk"
    rm -f "$out/build/unsigned.apk" "$out/build/aligned.apk"
    echo "Built $apk; install it with: adb install -r $apk"
done

# ----- the App Bundle: the same parts laid out as bundletool wants them -----
# bundletool: the jar given, the command on PATH, or the pinned release
# downloaded once (checked against its SHA-256). The function has a name
# of its own: called `bundletool`, `command -v bundletool` would find the
# function itself and `command bundletool` then fail with "not found".
BUNDLETOOL_VERSION=1.18.1
BUNDLETOOL_SHA256=675786493983787ffa11550bdb7c0715679a44e1643f3ff980a529e9c822595c
run_bundletool() {
    if [ -n "${BUNDLETOOL_JAR:-}" ]; then
        java -jar "$BUNDLETOOL_JAR" "$@"
    elif command -v bundletool >/dev/null; then
        command bundletool "$@"
    else
        local_jar="$out/tools/bundletool-all-$BUNDLETOOL_VERSION.jar"
        if [ ! -f "$local_jar" ]; then
            curl -sSL -o "$local_jar" \
                "https://github.com/google/bundletool/releases/download/$BUNDLETOOL_VERSION/bundletool-all-$BUNDLETOOL_VERSION.jar"
            echo "$BUNDLETOOL_SHA256  $local_jar" | sha256sum -c - >/dev/null \
                || { echo "bundletool download does not match its checksum" >&2; rm -f "$local_jar"; exit 1; }
        fi
        java -jar "$local_jar" "$@"
    fi
}
# The base module: the manifest and resources in protocol buffer form
# (aapt2 --proto-format), the code under dex/, the library under lib/.
mod="$out/build/module"
rm -rf "$mod" && mkdir -p "$mod/manifest" "$mod/dex"
link "$out/build/proto.apk" --proto-format
(cd "$out/build" && rm -rf proto && mkdir proto && cd proto && unzip -q ../proto.apk)
mv "$out/build/proto/AndroidManifest.xml" "$mod/manifest/AndroidManifest.xml"
mv "$out/build/proto/resources.pb" "$mod/resources.pb"
[ -d "$out/build/proto/res" ] && mv "$out/build/proto/res" "$mod/res"
cp "$out/build/classes.dex" "$mod/dex/classes.dex"
cp -R "$out/build/lib" "$mod/lib"
(cd "$mod" && zip -q -r ../base.zip .)
aab_unsigned="$out/build/unsigned.aab"
rm -f "$aab_unsigned"
run_bundletool build-bundle --modules="$out/build/base.zip" --output="$aab_unsigned"

# The signature: the upload key from the environment (Play App Signing),
# else the debug key (a bundle for local testing with bundletool
# build-apks, not for the store).
rks=""; rks_pass=""; rks_alias=""; rks_key_pass=""; signed_with="debug key"
if [ -n "${ANDROID_KEYSTORE_BASE64:-}" ] || [ -n "${ANDROID_KEYSTORE:-}" ]; then
    if [ -n "${ANDROID_KEYSTORE:-}" ]; then
        rks="$ANDROID_KEYSTORE"
    else
        # Decoded for the signing only and removed however the script ends.
        rks="$out/build/upload.keystore"
        trap 'rm -f "$out/build/upload.keystore"' EXIT
        echo "$ANDROID_KEYSTORE_BASE64" | base64 -d > "$rks"
    fi
    rks_pass="${ANDROID_KEYSTORE_PASSWORD:-}"
    rks_alias="${ANDROID_KEY_ALIAS:-}"
    rks_key_pass="${ANDROID_KEY_PASSWORD:-$rks_pass}"
    if [ -z "$rks_pass" ] || [ -z "$rks_alias" ]; then
        echo "scripts/android/build.sh: ANDROID_KEYSTORE_PASSWORD and ANDROID_KEY_ALIAS go with the keystore" >&2
        exit 1
    fi
    signed_with="upload key ($rks_alias)"
    aab="$out/tracedraw-$version-android-release.aab"
else
    rks="$ks"; rks_pass=android; rks_alias=androiddebugkey; rks_key_pass=android
    aab="$out/tracedraw-$version-android-debug.aab"
fi
cp "$aab_unsigned" "$aab"
# (A PKCS12 keystore, keytool's default, has one password for the store
# and the key: give ANDROID_KEY_PASSWORD the same value or leave it out.)
jarsigner -sigalg SHA256withRSA -digestalg SHA-256 \
    -keystore "$rks" -storepass "$rks_pass" -keypass "$rks_key_pass" "$aab" "$rks_alias" >/dev/null
jarsigner -verify "$aab" >/dev/null
rm -f "$out/build/upload.keystore"
run_bundletool validate --bundle="$aab" >/dev/null
echo "Built $aab (signed with the $signed_with)"
