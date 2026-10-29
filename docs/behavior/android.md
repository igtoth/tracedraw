# Android app

The same editor built as an Android app for 64-bit ARM tablets and
phones (#893): `libtracedraw.so`, the `tracedraw` crate compiled as a
shared library for `aarch64-linux-android`, loaded by the app's activity
(`dev.tracedraw.app.TraceDrawActivity`, a `NativeActivity` with a page
of Java in `packaging/android/java`) and packaged as an APK and as an
App Bundle. eframe drives egui through winit's Android backend
(`android-activity`, native activity) and draws with wgpu (Vulkan, or
OpenGL ES through wgpu where Vulkan is missing); touch and stylus events
reach egui as pointer and touch events, so the canvas, toolbox, property
bar and menus work with a finger or a pen. Phase 2 (files and the
clipboard through the system, a bundle for Google Play) builds and
passes every check that can be made off a device; it has not been
launched on a device or emulator yet, and it is not one of the release
downloads. `docs/android-play.md` lists what the owner does to publish
it and what to test on a tablet first.

## What works and what does not yet

| Feature | Desktop | Android, phase 2 |
|---|---|---|
| The editor: drawing tools, shapes, text, effects, menus, dockers, undo | yes | yes, the same code |
| Workspace | the one chosen | a fresh install starts in the Touch workspace (`workspaces.md`: bars and buttons 1.4 times larger, no rulers, panels or status bar, two fingers pan, a pinch zooms); Window > Workspace picks another and it is kept |
| Open, Import, palettes, scripts, ICC profiles, every "pick a file" | system file dialog | the system's document picker (Storage Access Framework, `ACTION_OPEN_DOCUMENT`, any type): the picked document opens by the extension of its display name; the closure runs on a later frame, as in the browser |
| Save on an opened drawing | writes the file | writes the document back through its content URI (the picker grants read and write, kept across restarts) |
| Save As, Export, Publish to PDF, Save Bitmap As, exports of dockers that use `save_file` | the Save Drawing dialog with folders, or the system save dialog | the Save Drawing dialog shows name, type and version only, then the system's picker (`ACTION_CREATE_DOCUMENT`) asks where, with that name and the type's MIME; the file is written when it answers. Publish to PDF has no Browse button: OK opens the picker |
| The few saves that need a path at once (`Dialog::save_path`: tone curve and duotone presets, the mask file, macro scripts and exports, palette files) | system save dialog | no picker yet: the file goes to the app's documents folder (`Android/data/dev.tracedraw.app/files` on the shared storage, which file managers show) under the suggested name |
| Recent files, Welcome screen thumbnails | paths | the same list: a picked document is named `/content/<its URI, escaped>/<its display name>` (`files::document_path`), so the name shows and the URI reopens it; the system keeps the permission (`takePersistableUriPermission`) |
| Pick a folder (export folders, font folders) | system folder dialog | not available (nothing is picked) |
| Print, Send To, Edit Bitmap | system programs | not available (no printing service, no external programs) |
| Clipboard | text, images and files through the system clipboard | text through the system's clipboard service: Copy puts the selection's SVG text there (other apps paste it as text, the editor as objects), Paste reads text copied elsewhere (artistic text, or objects when it is SVG). Pictures and files copied elsewhere are not read yet; copies inside the editor work as before |
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

## Files through the system picker

Android apps built for Android 11 and later cannot browse the shared
storage themselves; they ask the system's picker, which hands back a
content URI with a grant to read it (and write it, for most providers).
TraceDraw keeps the editor's path-based code and names each document by
a synthetic path:

```text
/content/content:%2F%2Fcom.android.externalstorage.documents%2Fdocument%2Fprimary%253ADrawings%252Flogo.cdr/logo.cdr
```

`files::document_path(uri, name)` makes it (every `%` of the URI becomes
`%25`, every `/` becomes `%2F`, the display name is the last part),
`files::document_uri(path)` gives the URI back, `files::read` and
`files::write` call the activity's `readDocument` and `writeDocument`
for such a path (`write_with` gathers the bytes first), `canonical`
leaves it alone and `exists` says yes (the system holds the file and
asked about replacing it). A picked document without an extension gets
one from its MIME type (`name_with_extension`), and the save picker is
told the MIME type of the name's extension (`mime_for_extension`: PDF,
the image types, SVG, text; a plain byte stream for `.tdraw` and `.cdr`,
which keeps the name as typed).

The flow, from the editor's thread: `Dialog::pick_file` (or `pick_files`,
`pick_into`, `save_file`) takes a request code, remembers what to do
with the answer (`files::android_pickers`) and calls the activity's
`openDocument` or `createDocument` over JNI (`android::documents`); the
activity starts the picker on its UI thread; `onActivityResult` takes
the persistable permission, reads each document's display name and MIME
type and calls the native method `nativeDocumentResult` (registered from
`android_main` with `RegisterNatives` on the activity's class, as the
library is loaded by the system, not by `System.loadLibrary` alone);
the Rust side queues the answer and asks egui for a frame;
`files::run_finished`, called once per frame, runs the waiting closure
with the documents' paths. A cancelled picker runs nothing. Several
documents (Import of many) come as newline-separated lists.

## The system clipboard

`android::clipboard::text` and `set_text` call the activity's
`getClipboardText` and `setClipboardText`, which run on the UI thread
(`ClipboardManager` wants it) and wait up to two seconds. Android 10 and
later hand the clipboard's content only to the app in front, which the
editor is when it pastes.

## Building the APK and the bundle

```sh
rustup target add aarch64-linux-android
export ANDROID_NDK_HOME=/path/to/android-ndk-r28   # r26 or later
# aapt2, zipalign, apksigner and d8 from an Android SDK (ANDROID_HOME)
# with platforms/android-36 (or ANDROID_PLATFORM_JAR); a JDK (javac,
# keytool, jarsigner); bundletool (BUNDLETOOL_JAR, on PATH, or
# downloaded once into dist-android/tools).
scripts/android/build.sh
adb install -r dist-android/tracedraw-<version>-android-arm64-v8a-debug.apk
```

The script builds the library (`cargo rustc -p tracedraw --lib --release
--target aarch64-linux-android --crate-type cdylib`, with the NDK's clang
as the C compiler and linker for the Rust target and the dependencies'
C code), strips it, compiles the activity (`javac -source 8 -target 8`
against the platform's `android.jar`, then `d8 --min-api 24` into one
`classes.dex`), compiles the launcher icons (`packaging/android/res`,
from the application icon), links them with the manifest
(`packaging/android/AndroidManifest.xml`: package `dev.tracedraw.app`,
`TraceDrawActivity` with `android.app.lib_name` = `tracedraw`, minimum
Android 7.0 / API 24, target Android 16 / API 36, full-screen theme,
every configuration change handled by the app, `INTERNET` the only
permission) into an APK with the version code from `Cargo.toml`
(0.19.10 is 19010), adds the code and the library
(`lib/arm64-v8a/libtracedraw.so`), aligns it and signs it with a debug
key (`~/.android/debug.keystore`, made when missing, as the Android
tools do). A debug-signed APK installs on a device that allows apps from
unknown sources and is not for a store.

For the bundle the same parts are linked again in protocol buffer form
(`aapt2 link --proto-format`) and laid out as a base module
(`manifest/AndroidManifest.xml`, `resources.pb`, `res/`,
`dex/classes.dex`, `lib/arm64-v8a/`), which `bundletool build-bundle`
turns into the `.aab`; `jarsigner` signs it with the upload key when
`ANDROID_KEYSTORE_BASE64` (or `ANDROID_KEYSTORE`), `ANDROID_KEYSTORE_PASSWORD`,
`ANDROID_KEY_ALIAS` and `ANDROID_KEY_PASSWORD` are set
(`tracedraw-<version>-android-release.aab`, for Play App Signing with
our upload key) and with the debug key otherwise
(`tracedraw-<version>-android-debug.aab`, for `bundletool build-apks
--local-testing` only); `bundletool validate` checks the result.
bundletool 1.18.1 is pinned with its SHA-256 for the download.

CI (`build.yml`, job `android`) builds both on every push with the NDK
the GitHub runner has, installs platform 36 when the runner lacks it,
passes the four signing secrets to the script (absent: the debug key)
and keeps the `android-apk` and `android-aab` artifacts; nothing is
attached to the releases, uploading to Play is the owner's step
(`docs/android-play.md`).

Checks: `android_package::tests::the_manifest_and_the_build_agree` (the
manifest names the activity, the library, the package and the API
levels; the build makes the library, the code, the APK and the bundle
with the signing variables; the crate carries the Android dependencies
at exact versions; CI passes the secrets and keeps both artifacts) and
`the_activity_and_the_bridge_agree` (every Java method the Rust bridge
calls exists with the signature the bridge names; the native method the
activity declares is the one the bridge registers; every picker call
ends in a result). `files::tests` cover the document paths and the MIME
types. Built here on Linux x86_64 with NDK r28.2, build-tools 36.1.0 and
the API 36 `android.jar`: the APK and both bundle variants come out,
`aapt dump badging` gives the package, version code, minimum API 24,
target 36, `native-code: arm64-v8a` and the launchable activity,
`apksigner verify` passes with the v2 and v3 schemes, `bundletool
validate` accepts the bundle and `bundletool build-apks` makes a
universal APK from it. The APK installs and starts only on a device or
an emulator, which this build machine has not had: launching it, the
picker, the saves and the clipboard are the next checks, listed in
`docs/android-play.md`.

## How it is wired

- `apps/tracedraw/src/lib.rs` holds the whole app; `main.rs` is the
  desktop and browser program (`tracedraw::run()`); `android.rs`
  (`#[cfg(target_os = "android")]`) is the Android entry:
  `android_main(app: AndroidApp)`, called by the glue of
  `android-activity` on the app's thread, sets up the system logger
  (tag `tracedraw`), a panic hook that logs, the settings and documents
  folders from the activity's paths, keeps the activity's JNI reference
  and registers the native method, the Touch workspace on a first start,
  the language, then `eframe::run_native` with `NativeOptions {
  android_app: Some(app), renderer: Wgpu }`, keeping the egui context
  for the wake-ups. eframe drops the window and surface on Suspended and
  makes them again on Resumed, as Android asks.
- `android.rs` also holds the JNI bridge: `with_activity` attaches the
  thread (`JavaVM::singleton`, which `android-activity` set up) and
  borrows the activity; `documents` (open, create, read, write, the
  answer queue, `register_natives`) and `clipboard` (text, set_text).
  Every JNI failure is logged and comes back as `None`, never a panic.
- `packaging/android/java/dev/tracedraw/app/TraceDrawActivity.java`: the
  activity, with the picker intents, `onActivityResult`, the content
  resolver reads and writes and the clipboard, all on the right threads.
- `Cargo.toml`: a `[lib]` (doc tests off) beside the `[[bin]]`; `rfd`
  (file dialogs) on every target but Android; `arboard` (system
  clipboard) on the desktop targets only; `eframe` with
  `android-native-activity`, `android-activity 0.6.1`, `android_logger
  0.15.1` and `jni 0.22.4` (the version `android-activity` uses) on
  Android only. The desktop and browser builds resolve the same
  dependencies as before.
- `files.rs`: `ANDROID`, the document paths, the Android branches of
  `read`, `write`, `write_with`, `exists`, `canonical` and `Dialog`
  (`android_pickers`); `Dialog::save_file`'s closure is `'static` on
  every platform now, as the answer may come on a later frame.
  `clipboard.rs`: the system clipboard's text on Android;
  `ui/save_as.rs`: Save and Save As hand the name and type to the
  picker; `ui/publish_pdf.rs`: OK opens the picker; `startup_log.rs`:
  error boxes go to the log on Android.

## Later phases

1. The remaining synchronous saves (`Dialog::save_path`) and the folder
   pickers through the system picker; sharing a PDF or image to another
   app (`ACTION_SEND`); opening a `.cdr` or `.tdraw` from a file manager
   (`ACTION_VIEW`).
2. Pictures and files from the system clipboard.
3. Pen pressure and tilt (#850), palm rejection, a long press as the
   right click.
4. Printing through the Android print service; the Help inside the app.
5. 32-bit ARM and x86_64 libraries for Chromebooks; the store listing's
   screenshots from a device.
