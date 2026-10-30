//! The Android app's package (`packaging/android`, `scripts/android`):
//! checks that the manifest, the activity's Java code, the build script,
//! the CI job and the crate agree. See docs/behavior/android.md and
//! docs/android-play.md.

#[cfg(test)]
mod tests {
    const MANIFEST: &str = include_str!("../../../packaging/android/AndroidManifest.xml");
    const ACTIVITY: &str =
        include_str!("../../../packaging/android/java/dev/tracedraw/app/TraceDrawActivity.java");
    const BUILD: &str = include_str!("../../../scripts/android/build.sh");
    const CARGO: &str = include_str!("../Cargo.toml");
    const CI: &str = include_str!("../../../.github/workflows/build.yml");
    const BRIDGE: &str = include_str!("android.rs");
    const PLAY: &str = include_str!("../../../docs/android-play.md");

    /// The manifest names the activity the Java code defines, the
    /// library the build packages, the package and the API levels; the
    /// build makes that library for the target of the manifest's ABI and
    /// the code for the activity; the crate carries the Android
    /// dependencies at exact versions and stays a library plus a program.
    #[test]
    fn the_manifest_and_the_build_agree() {
        for needed in [
            "package=\"dev.tracedraw.app\"",
            "android:name=\"dev.tracedraw.app.TraceDrawActivity\"",
            "android:name=\"android.app.lib_name\" android:value=\"tracedraw\"",
            "android:minSdkVersion=\"24\"",
            "android:targetSdkVersion=\"36\"",
            "android:hasCode=\"true\"",
            "android:exported=\"true\"",
            "android:icon=\"@mipmap/ic_launcher\"",
            "android.intent.category.LAUNCHER",
        ] {
            assert!(MANIFEST.contains(needed), "the manifest lacks {needed:?}");
        }
        // The only permission is the network: files go through the
        // system's picker, which needs none.
        assert_eq!(MANIFEST.matches("<uses-permission").count(), 1);
        assert!(MANIFEST.contains("android.permission.INTERNET"));
        for needed in [
            "API=24",
            "PLATFORM=36",
            "ABIS=\"${ANDROID_ABIS:-arm64-v8a}\"",
            "arm64-v8a) echo aarch64-linux-android ;;",
            "x86_64) echo x86_64-linux-android ;;",
            "cargo rustc -p tracedraw --lib --release --target \"$TARGET\" --crate-type cdylib",
            "libtracedraw.so",
            "packaging/android/AndroidManifest.xml",
            "packaging/android/res",
            "packaging/android/java",
            "javac -source 8 -target 8",
            "--min-api \"$API\"",
            "classes.dex",
            "--version-code \"$code\" --version-name \"$version\"",
            "--min-sdk-version \"$API\" --target-sdk-version \"$PLATFORM\"",
            "zipalign",
            "apksigner",
            "android-$ABI-debug.apk",
            "--proto-format",
            "build-bundle",
            "ANDROID_KEYSTORE_BASE64",
            "ANDROID_KEYSTORE_PASSWORD",
            "ANDROID_KEY_ALIAS",
            "ANDROID_KEY_PASSWORD",
            "android-release.aab",
            "android-debug.aab",
            "jarsigner",
            "bundletool validate",
        ] {
            assert!(
                BUILD.contains(needed),
                "scripts/android/build.sh lacks {needed:?}"
            );
        }
        // The bundletool the script downloads when none is installed is
        // pinned with its checksum.
        assert!(BUILD.contains("BUNDLETOOL_VERSION=1.18.1"));
        assert!(BUILD.contains(
            "BUNDLETOOL_SHA256=675786493983787ffa11550bdb7c0715679a44e1643f3ff980a529e9c822595c"
        ));
        // No shell function shadows a command the script looks up with
        // `command -v`: the lookup would find the function and `command`
        // then fail with exit code 127 (it did in CI).
        for line in BUILD.lines() {
            if let Some(name) = line.strip_suffix("() {") {
                assert!(
                    !BUILD.contains(&format!("command -v {name} ")),
                    "the function {name} shadows the command it looks up"
                );
            }
        }
        for needed in [
            "[lib]\ndoctest = false",
            "[target.'cfg(target_os = \"android\")'.dependencies]",
            "features = [\"wgpu\", \"android-native-activity\"]",
            "android-activity = { version = \"=0.6.1\", features = [\"native-activity\"] }",
            "android_logger = \"=0.15.1\"",
            "jni = \"=0.22.4\"",
            "[target.'cfg(not(target_os = \"android\"))'.dependencies]\nrfd.workspace = true",
            "[target.'cfg(not(any(target_arch = \"wasm32\", target_os = \"android\")))'.dependencies]\narboard",
        ] {
            assert!(CARGO.contains(needed), "Cargo.toml lacks {needed:?}");
        }
        // CI builds the APK and the bundle with the runner's NDK, keeps
        // both and hands the script the signing secrets.
        assert!(CI.contains("scripts/android/build.sh") && CI.contains("aarch64-linux-android"));
        assert!(CI.contains("name: android-apk") && CI.contains("name: android-aab"));
        for secret in [
            "ANDROID_KEYSTORE_BASE64",
            "ANDROID_KEYSTORE_PASSWORD",
            "ANDROID_KEY_ALIAS",
            "ANDROID_KEY_PASSWORD",
        ] {
            assert!(
                CI.contains(&format!("{secret}: ${{{{ secrets.{secret} }}}}")),
                "build.yml does not pass {secret}"
            );
        }
        for text in [MANIFEST, BUILD, ACTIVITY, PLAY] {
            assert!(!text.contains('\u{2014}'));
        }
        // The launcher icons at every density.
        for density in ["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../packaging/android/res/mipmap-{density}/ic_launcher.png"
            ));
            assert!(path.is_file(), "no launcher icon for {density}");
        }
    }

    /// Google Play updates the Android app: it never asks GitHub for a
    /// release (nor asks at its first start whether to), and Help >
    /// Updates opens its Play page, named by the manifest's package.
    #[test]
    fn play_updates_the_android_app() {
        let updates = include_str!("updates.rs");
        let android = updates
            .split("#[cfg(target_os = \"android\")]\npub fn store_install() -> bool {")
            .nth(1)
            .expect("an Android store_install");
        assert!(android.trim_start().starts_with("true"));
        assert!(updates.contains(
            "pub const STORE_PAGE: &str = \"https://play.google.com/store/apps/details?id=dev.tracedraw.app\";"
        ));
        assert!(MANIFEST.contains("package=\"dev.tracedraw.app\""));
    }

    const EMULATOR_CHECK: &str = include_str!("../../../scripts/android/emulator_check.sh");
    const SCREEN_CHECK: &str = include_str!("../../../scripts/android/screen_check.py");

    /// CI starts the APK it built on an emulator and fails when the app
    /// does not stay up, crashed or left its window blank.
    #[test]
    fn ci_starts_the_apk_on_an_emulator() {
        let job = CI
            .split("\n  android-emulator:\n")
            .nth(1)
            .and_then(|rest| rest.split("\n\n  ").next())
            .expect("build.yml has an android-emulator job");
        for needed in [
            "needs: android",
            "name: android-apk",
            "udevadm trigger --name-match=kvm",
            "arch: x86_64",
            "scripts/android/emulator_check.sh",
            "name: android-emulator",
        ] {
            assert!(
                job.contains(needed),
                "the android-emulator job lacks {needed:?}"
            );
        }
        // The emulator action is pinned to a commit, not a moving tag.
        let pinned = job
            .split("reactivecircus/android-emulator-runner@")
            .nth(1)
            .map(|rest| rest.chars().take_while(|c| c.is_ascii_hexdigit()).count());
        assert_eq!(
            pinned,
            Some(40),
            "the emulator action is pinned to a commit"
        );
        for needed in [
            "pidof",
            "FATAL EXCEPTION",
            "Fatal signal",
            "panicked at",
            "ANR in",
            "screencap -p",
            "first frame",
            "loaded 0 font faces",
            "scripts/android/screen_check.py",
        ] {
            assert!(
                EMULATOR_CHECK.contains(needed),
                "scripts/android/emulator_check.sh lacks {needed:?}"
            );
        }
        assert!(SCREEN_CHECK.contains("MAX_SHARE") && SCREEN_CHECK.contains("MIN_COLOURS"));
        for text in [EMULATOR_CHECK, SCREEN_CHECK] {
            assert!(!text.contains('\u{2014}'));
        }
    }

    /// The Java activity and the Rust bridge agree on every method that
    /// crosses JNI: the activity extends NativeActivity and loads the
    /// library, each Java method the bridge calls exists with the
    /// signature the bridge names, and the native method the activity
    /// declares is the one the bridge registers.
    #[test]
    fn the_activity_and_the_bridge_agree() {
        assert!(ACTIVITY.contains("package dev.tracedraw.app;"));
        assert!(ACTIVITY.contains("public class TraceDrawActivity extends NativeActivity"));
        assert!(ACTIVITY.contains("System.loadLibrary(\"tracedraw\")"));
        // (Java declaration, the bridge's jni_str name, the bridge's signature.)
        for (java, name, sig) in [
            (
                "public void openDocument(final int request, final boolean multiple)",
                "openDocument",
                "(IZ)V",
            ),
            (
                "public void createDocument(final int request, final String mime, final String name)",
                "createDocument",
                "(ILjava/lang/String;Ljava/lang/String;)V",
            ),
            (
                "public byte[] readDocument(String uri)",
                "readDocument",
                "(Ljava/lang/String;)[B",
            ),
            (
                "public boolean writeDocument(String uri, byte[] data)",
                "writeDocument",
                "(Ljava/lang/String;[B)Z",
            ),
            (
                "public String getClipboardText()",
                "getClipboardText",
                "()Ljava/lang/String;",
            ),
            (
                "public boolean setClipboardText(final String text)",
                "setClipboardText",
                "(Ljava/lang/String;)Z",
            ),
        ] {
            assert!(ACTIVITY.contains(java), "the activity lacks {java:?}");
            assert!(
                BRIDGE.contains(&format!("jni_str!(\"{name}\")")),
                "the bridge does not call {name}"
            );
            assert!(
                BRIDGE.contains(&format!("jni_sig!(\"{sig}\")")),
                "the bridge does not use the signature {sig:?}"
            );
        }
        assert!(ACTIVITY.contains(
            "private native void nativeDocumentResult(int request, String uris, String names, String mimes);"
        ));
        assert!(BRIDGE.contains("jni_str!(\"nativeDocumentResult\")"));
        assert!(BRIDGE
            .contains("jni_str!(\"(ILjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V\")"));
        // Every call to the activity's methods, cancel included, ends in
        // a result (the Rust side would otherwise wait for ever).
        assert_eq!(
            ACTIVITY
                .matches("nativeDocumentResult(request, null, null, null)")
                .count(),
            2
        );
        assert!(ACTIVITY.contains("nativeDocumentResult(requestCode, null, null, null)"));
        assert!(ACTIVITY.contains("takePersistableUriPermission"));
        // The listing skeleton for Play is in place.
        let meta = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fastlane/metadata/android/en-US");
        for file in ["title.txt", "short_description.txt", "full_description.txt"] {
            assert!(meta.join(file).is_file(), "fastlane metadata lacks {file}");
        }
        let short = std::fs::read_to_string(meta.join("short_description.txt")).unwrap_or_default();
        assert!(!short.is_empty() && short.chars().count() <= 80);
    }
}
