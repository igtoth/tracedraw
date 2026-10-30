# Publishing the Android app on Google Play

What the repository builds and what only the owner of the Google Play
account can do. The code side is `scripts/android/build.sh` and the
`android` job of `build.yml` (see `behavior/android.md`); everything
below the first section happens in the Play Console and cannot be
automated from here. Checked against Google Play's requirements of
October 2026; the Console changes its wording now and then, so take the
field names as a guide.

## What the build gives you

- `tracedraw-<version>-android-arm64-v8a-debug.apk` (tablets and
  phones) and `tracedraw-<version>-android-x86_64-debug.apk` (Chromebooks
  with Intel or AMD processors, emulators): debug-signed, for
  `adb install` with "install from unknown sources". Never upload them.
- `tracedraw-<version>-android-release.aab`: the Android App Bundle for
  Play, with both libraries (Play hands each device its own), signed with the upload key when the four secrets below are set in
  the repository (Settings > Secrets and variables > Actions). Without
  them the job makes `tracedraw-<version>-android-debug.aab`, signed with
  the debug key, which Play rejects; it is only for
  `bundletool build-apks --local-testing`.
- Both are artifacts of the `android` job on every push and pull request
  (`android-apk`, `android-aab`); nothing is attached to the GitHub
  releases automatically. Download the bundle from the run that built
  the release's tag.
- Version code = major * 1000000 + minor * 1000 + patch from `Cargo.toml`
  (0.19.10 is 19010); Play needs each upload's code to be higher than the
  last, so a release must bump the version.
- Target: Android 16 (API 36), the level Play asks of new apps and
  updates from August 31, 2026 (existing apps must target at least API 35
  to stay visible). Minimum: Android 7.0 (API 24). ABI: arm64-v8a only;
  the 16 KB page size newer devices require is met (the library's
  segments are 16 KB aligned, which `llvm-readelf -l` shows).
- Permissions: `INTERNET` only. Files go through the system's document
  picker, which needs no storage permission; nothing reads contacts,
  location, the camera or the microphone.

## The upload key (once)

Play App Signing holds the app signing key; you keep an upload key and
give Play its certificate. Make it once and keep it safe (losing it means
a key reset request to Play support):

```sh
keytool -genkeypair -v -keystore tracedraw-upload.jks -alias upload \
    -keyalg RSA -keysize 4096 -validity 10000 \
    -dname "CN=TraceDraw, O=TraceDraw, C=PT"
base64 -w0 tracedraw-upload.jks > tracedraw-upload.jks.b64
```

Then the four repository secrets:

| Secret | Value |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | the content of `tracedraw-upload.jks.b64` |
| `ANDROID_KEYSTORE_PASSWORD` | the keystore password |
| `ANDROID_KEY_ALIAS` | `upload` (the alias above) |
| `ANDROID_KEY_PASSWORD` | the key's password: the same as the keystore's for a `.jks` made by a current keytool (PKCS12 keystores have one password; keytool ignores a different `-keypass`), so this secret can be left out |

When you create the app in the Console, choose Play App Signing with
"Use a Google-generated key" and upload the first release bundle signed
with this key: Play registers its certificate as the upload certificate.

## Owner's checklist

1. **Developer account.** Register at play.google.com/console (a one-off
   fee, identity verification with a document and a payment profile; a
   personal account also needs a phone number and address that Play
   shows on the listing, an organisation account a D-U-N-S number). Allow
   a few days for the verification.
2. **Create the app.** Name TraceDraw, default language English (United
   States), App, Free. The package name `dev.tracedraw.app` is fixed by
   the first upload and can never change: keep it.
3. **Closed testing first.** A personal account created after November
   13, 2023 must run a closed test with at least 12 testers opted in,
   continuously, for 14 days before it may apply for production access;
   the Console then asks a few questions about the test and the app. Make
   a closed track, add the testers' Google accounts (or a Google Group),
   upload the bundle, and share the opt-in link with them. Keep the 12
   enrolled for the full 14 days.
4. **Privacy policy URL.** Required for every app. The repository's
   `PRIVACY.md` is the text; publish it at a stable public URL (the
   GitHub page or the project site) and enter it under App content >
   Privacy policy. It should say what the app sends over the network: the
   update check to GitHub when enabled, and requests to an AI service the
   user configures (AI Drawing), nothing else; no accounts, no analytics.
5. **Data safety form.** App content > Data safety. With the app as it
   is: no data collected or shared (the update check and AI requests are
   made at the user's request to servers the user chooses; drawings stay
   on the device or where the user saves them). Data is not encrypted in
   transit by the app beyond HTTPS, there is no deletion request path
   because nothing is stored. Answer "No" to collection unless a later
   version adds telemetry.
6. **App access.** All features are available without login: "All
   functionality is available without special access".
7. **Ads.** No ads.
8. **Content rating.** Fill in the IARC questionnaire as a Utility,
   Productivity, Communication or Other app: no violence, sexual content,
   gambling, user interaction or sharing of location. Expect Everyone / 3.
9. **Target audience.** 18 and over (or 13 and over); not designed for
   children, so no Families policy requirements.
10. **News, COVID, financial, health, government declarations.** None
    apply.
11. **Store listing** (Main store listing). Reuse `docs/store-listing.md`
    and `fastlane/metadata/android/en-US/`:
    - App name (30): TraceDraw
    - Short description (80): the first sentence of the listing.
    - Full description (4000): the listing text; no mention of other
      products' names.
    - App icon: 512 x 512 PNG, 32-bit, no transparency, from
      `assets/icon/` (render the SVG at 512).
    - Feature graphic: 1024 x 500 PNG or JPEG (the app's name on a plain
      background with a drawing is enough).
    - Phone screenshots: at least 2, 16:9 or 9:16, 320 to 3840 px.
    - 7-inch tablet screenshots: at least 2 for the tablet listing.
    - 10-inch tablet screenshots: at least 2; the app is meant for
      tablets, so take these on a real tablet in landscape (1920 x 1200
      or larger) showing the Touch workspace with a drawing open, the
      toolbox and a docker.
    - Category: Art & Design. Tags: vector graphics, drawing.
    - Contact details: an email address (shown publicly), the website.
12. **Upload the bundle** to the chosen track (Release > Testing >
    Closed, then Production once admitted). Play checks the bundle: the
    target API, the 16 KB page size, the permissions and the signing
    certificate. Write the release notes (fastlane's
    `changelogs/<version code>.txt` holds them).
13. **Device catalogue.** Release > Device catalogue: with
    `arm64-v8a` and `x86_64` and OpenGL ES 3.0 required, phones and
    tablets from 2016 on and Chromebooks qualify; exclude nothing more.
14. **Review.** First reviews take up to a week. Common rejections to
    avoid: a privacy policy that does not match the Data safety form, a
    listing that names other products, screenshots that are not of the
    app, an app that cannot open files (the document picker covers this).

## Test on a real tablet before uploading

CI starts the x86_64 build on Android 11 and 14 emulators on every push
and checks that it comes up, stays up and draws the editor
(`behavior/android.md`, "The emulator check"); nothing there taps,
picks a file or uses the ARM64 library. These are the first things to
check on a tablet with the debug APK (`adb install -r`, then
`adb logcat -s tracedraw`):

- The app starts into the Touch workspace and survives rotation and
  going to the background and back.
- File > Open shows the system's picker; a `.tdraw` and a `.cdr` picked
  from Downloads or Drive open with their names in the title; the file
  then appears in File > Open Recent and in the Welcome screen, and
  opens from there after the app was closed.
- File > Import of a PNG and an SVG from the picker.
- Save on an opened drawing writes it back where it is (check the file
  manager's modified time); Save As and Export show the create-document
  picker with the suggested name and write the file; Publish to PDF
  writes a PDF that another app opens.
- Edit > Copy of text, Paste in another app; a text copied elsewhere
  pastes as artistic text here.
- The App Bundle installs through `bundletool build-apks --local-testing`
  and `bundletool install-apks`, and later from the closed-testing track.

## fastlane

`fastlane/metadata/android/en-US/` holds the listing text in the layout
fastlane supply reads (`title.txt`, `short_description.txt`,
`full_description.txt`, `changelogs/`), so a later step can push the
listing with `fastlane supply --aab <file>` once a Play service account
exists. Nothing in the repository runs fastlane yet; it is a skeleton to
fill, with the screenshot folders (`images/phoneScreenshots`,
`images/sevenInchScreenshots`, `images/tenInchScreenshots`) left empty
until there are screenshots from a device.
