//! The Android app's package (`packaging/android`, `scripts/android`):
//! checks that the manifest, the build script and the crate agree. See
//! docs/behavior/android.md.

#[cfg(test)]
mod tests {
    const MANIFEST: &str = include_str!("../../../packaging/android/AndroidManifest.xml");
    const BUILD: &str = include_str!("../../../scripts/android/build.sh");
    const CARGO: &str = include_str!("../Cargo.toml");
    const CI: &str = include_str!("../../../.github/workflows/build.yml");

    /// The manifest names the library the build packages, the package,
    /// the activity and the API level; the build makes that library for
    /// the target of the manifest's ABI; the crate carries the Android
    /// dependencies at exact versions and stays a library plus a program.
    #[test]
    fn the_manifest_and_the_build_agree() {
        for needed in [
            "package=\"dev.tracedraw.app\"",
            "android:name=\"android.app.NativeActivity\"",
            "android:name=\"android.app.lib_name\" android:value=\"tracedraw\"",
            "android:minSdkVersion=\"24\"",
            "android:hasCode=\"false\"",
            "android:exported=\"true\"",
            "android:icon=\"@mipmap/ic_launcher\"",
            "android.intent.category.LAUNCHER",
        ] {
            assert!(MANIFEST.contains(needed), "the manifest lacks {needed:?}");
        }
        for needed in [
            "API=24",
            "TARGET=aarch64-linux-android",
            "ABI=arm64-v8a",
            "cargo rustc -p tracedraw --lib --release --target \"$TARGET\" --crate-type cdylib",
            "libtracedraw.so",
            "packaging/android/AndroidManifest.xml",
            "packaging/android/res",
            "--version-code \"$code\" --version-name \"$version\"",
            "zipalign",
            "apksigner",
            "android-$ABI-debug.apk",
        ] {
            assert!(
                BUILD.contains(needed),
                "scripts/android/build.sh lacks {needed:?}"
            );
        }
        for needed in [
            "[lib]\ndoctest = false",
            "[target.'cfg(target_os = \"android\")'.dependencies]",
            "features = [\"wgpu\", \"android-native-activity\"]",
            "android-activity = { version = \"=0.6.1\", features = [\"native-activity\"] }",
            "android_logger = \"=0.15.1\"",
            "[target.'cfg(not(target_os = \"android\"))'.dependencies]\nrfd.workspace = true",
            "[target.'cfg(not(any(target_arch = \"wasm32\", target_os = \"android\")))'.dependencies]\narboard",
        ] {
            assert!(CARGO.contains(needed), "Cargo.toml lacks {needed:?}");
        }
        // CI builds the APK with the runner's NDK and keeps it.
        assert!(CI.contains("scripts/android/build.sh") && CI.contains("aarch64-linux-android"));
        for text in [MANIFEST, BUILD] {
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
}
