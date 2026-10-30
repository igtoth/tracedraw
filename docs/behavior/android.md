# Android app

The same editor built as an Android app for 64-bit ARM tablets and
phones (#893): `libtracedraw.so`, the `tracedraw` crate compiled as a
shared library for `aarch64-linux-android`, loaded by the system's
`NativeActivity` and packaged as an APK. eframe drives egui through
winit's Android backend (`android-activity`, native activity) and draws
with wgpu (Vulkan, or OpenGL ES through wgpu where Vulkan is missing);
touch and stylus events reach egui as pointer and touch events, so the
canvas, toolbox, property bar and menus work with a finger or a pen.
Phase 1 is a start, not a release: it is not one of the release
downloads and has not been through a store.

## What works and what does not yet

| Feature | Desktop | Android, phase 1 |
|---|---|---|
| The editor: drawing tools, shapes, text, effects, menus, dockers, undo | yes | yes, the same code |
| Workspace | the one chosen | a fresh install starts in the Touch workspace (`workspaces.md`: bars and buttons 1.4 times larger, no rulers, panels or status bar, two fingers pan, a pinch zooms); Window > Workspace picks another and it is kept |
| Open, Import, palettes, scripts, ICC profiles | system file dialog | not available: no file picker yet (`files.rs`, `Dialog::pick_file` does nothing and logs a warning); drawings start from New or a template |
| Save, Save As, Export, Publish to PDF | the Save Drawing dialog with folders | the dialog without folders: name, type and version; the file goes to the app's documents folder, named under the note (`Android/data/dev.tracedraw.app/files` on the shared storage, which file managers show) |
| Print, Send To, Edit Bitmap | system programs | not available (no printing service, no external programs) |
| Clipboard | text, images and files through the system clipboard | inside the app only: text and objects copied in the editor paste in the editor; nothing reaches other apps yet (`clipboard.rs`) |
| Settings | `settings.json` in the configuration folder | `settings.json` in the app's private folder (`TRACEDRAW_CONFIG_DIR` set by `android_main`) |
| Fonts | installed system fonts | the system's fonts (`/system/fonts`: Roboto, Noto and the rest) |
| Keyboard | the physical keyboard | a physical or Bluetooth keyboard works; the on-screen keyboard is raised by the system when a text field has the focus (winit's IME), not checked on a device yet |
| Updates, AI Drawing | HTTPS requests | the requests compile; the certificate check needs the Java side Android gives apps with code, so they are expected to fail until a later phase |
| Help | the user Help in the browser | the same pages open in the browser |
| Errors | a message box and the log file | the system log only: `adb logcat -s tracedraw` |

Pen pressure is #850 (another change): `android-activity` exposes
`MotionEvent` pressure per pointer, so when winit passes it on the
Freehand tool's pressure code can read it the same way as on the
desktop.

## Building the APK

```sh
rustup target add aarch64-linux-android
export ANDROID_NDK_HOME=/path/to/android-ndk-r29   # r26 or later
# aapt2, zipalign and apksigner from an Android SDK (ANDROID_HOME) or the
# android-sdk-build-tools and apksigner packages; android.jar from the
# SDK's newest platform, ANDROID_PLATFORM_JAR, or the android-sdk-platform-23
# package; keytool from a JDK.
scripts/android/build.sh
adb install -r dist-android/tracedraw-<version>-android-arm64-v8a-debug.apk
```

The script builds the library (`cargo rustc -p tracedraw --lib --release
--target aarch64-linux-android --crate-type cdylib`, with the NDK's clang
as the C compiler and linker for the Rust target and the dependencies'
C code), strips it, compiles the launcher icons (`packaging/android/res`,
from the application icon), links them with the manifest
(`packaging/android/AndroidManifest.xml`: package `dev.tracedraw.app`,
`NativeActivity` with `android.app.lib_name` = `tracedraw`, no Java
code, minimum Android 7.0 / API 24, full-screen theme, every
configuration change handled by the app) into an APK with the version
code from `Cargo.toml` (0.19.10 is 19010), adds the library as
`lib/arm64-v8a/libtracedraw.so`, aligns it and signs it with a debug key
(`~/.android/debug.keystore`, made when missing, as the Android tools
do). A debug-signed APK installs on a device that allows apps from
unknown sources and is not for a store.

CI (`build.yml`, job `android`) builds it on every push with the NDK the
GitHub runner has and keeps it as `tracedraw-<version>-android-arm64-v8a-debug.apk`;
it is not attached to the releases.

Checks: `android_package::tests::the_manifest_and_the_build_agree`
(the manifest names the library the build packages, the package, the
activity and the API level; the build makes a library for the target the
manifest's ABI needs; the crate carries the Android dependencies at
exact versions). Built here on Linux x86_64 with NDK r29, aapt2 29.0.3 and
the API 23 `android.jar` (so the manifest uses no attribute newer than
API 23): a 23 MB APK holding a 56 MB stripped library. Checked on the
file: `aapt dump badging` gives the package, version code 19010,
minimum API 24, target 34, `native-code: arm64-v8a` and the launchable
`NativeActivity`; the library exports `ANativeActivity_onCreate` (the
symbol the system looks up) and `android_main`, needs only `libc`, `libm`,
`libdl`, `liblog` and `libandroid`, and its segments are aligned to 16 KB
(the page size newer devices require); `apksigner verify` passes with
the v2 and v3 schemes. The APK installs and starts only on a device or
an emulator, which this build machine has not had: launching it is the
next check, not done here.

## How it is wired

- `apps/tracedraw/src/lib.rs` holds the whole app; `main.rs` is the
  desktop and browser program (`tracedraw::run()`); `android.rs`
  (`#[cfg(target_os = "android")]`) is the Android entry:
  `android_main(app: AndroidApp)`, called by the glue of
  `android-activity` on the app's thread, sets up the system logger
  (tag `tracedraw`), a panic hook that logs, the settings and documents
  folders from the activity's paths, the Touch workspace on a first
  start, the language, then `eframe::run_native` with
  `NativeOptions { android_app: Some(app), renderer: Wgpu }`. eframe
  drops the window and surface on Suspended and makes them again on
  Resumed, as Android asks.
- `Cargo.toml`: a `[lib]` (doc tests off) beside the `[[bin]]`; `rfd`
  (file dialogs) on every target but Android; `arboard` (system
  clipboard) on the desktop targets only; `eframe` with
  `android-native-activity`, `android-activity 0.6.1` and
  `android_logger 0.15.1` on Android only. The desktop and browser builds
  resolve the same dependencies as before.
- `files.rs`: `ANDROID`, `save_dir` (the documents folder) and the
  Android branches of `Dialog`; `clipboard.rs`: no system clipboard on
  Android; `startup_log.rs`: error boxes go to the log on Android.

## Later phases

1. Files: open and save through the system's document picker (Storage
   Access Framework, through JNI from the activity), the recent list,
   sharing a PDF or image to another app.
2. The system clipboard (text and images) through the activity's
   clipboard service.
3. Pen pressure and tilt (#850), palm rejection, a long press as the
   right click.
4. Printing through the Android print service; the Help inside the app.
5. A signed release build, 32-bit ARM and x86_64 libraries for
   Chromebooks, and a store listing.
