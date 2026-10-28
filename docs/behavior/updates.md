# Updates

Desktop only (the browser build is always the latest; there Help >
Updates opens the releases page).

- Help > Updates asks GitHub for the project's latest release
  (`api.github.com/repos/igtoth/tracedraw/releases/latest`) and says
  whether it is newer than this version (numbers compared as major,
  minor, patch; drafts, pre-releases and the nightly build never count).
- A newer release offers Download, Release notes, Skip this version and
  Close. Download opens the link of the release file named exactly for
  this system, its processor (x86_64, or arm64 for aarch64) and how this
  copy was installed; the system's installer does the installing:
  - Linux run as an AppImage (`APPIMAGE` set): `tracedraw-<v>-linux-<arch>.AppImage`,
    where x86_64 looks for `linux-amd64` first and then `linux-x86_64`
    (releases up to 0.19.8). From 0.19.9 on the release names the x86_64
    AppImage `linux-amd64`, which sorts before `linux-arm64`: updaters of
    0.19.8 and earlier take the first AppImage in GitHub's list, sorted
    by name, and so get the x86_64 file (#1038). The tar.gz keeps
    `linux-x86_64`;
  - Linux installed from the `.deb` (dpkg's list for `tracedraw` names
    the running program): `tracedraw_<v>_<amd64|arm64>.deb`;
  - Linux unpacked from the tar.gz (a program outside `/usr`):
    `tracedraw-<v>-linux-<arch>.tar.gz`;
  - Windows installed with setup: `tracedraw-<v>-windows-<arch>-setup.exe`;
    the portable copy (the `tracedraw-settings` folder next to the
    program): `tracedraw-<v>-windows-<arch>-portable.zip`;
  - macOS: `tracedraw-<v>-macos-<arch>.dmg`.
  When the release has no such file (an rpm or other system package
  under `/usr`, a Flatpak or Snap, a processor without a build such as
  Windows on arm64) no file is downloaded and Download opens the release
  page instead; a file for another processor or package is never offered
  (#1038: the x86_64 AppImage took the arm64 one, listed first).
- Install in place (Linux AppImage only): when TraceDraw runs as an
  AppImage (the AppImage runtime names its file in `APPIMAGE`) and the
  release's AppImage for this processor has a SHA-256 digest (GitHub gives one for every
  uploaded file), the dialog offers Install first. It downloads the
  AppImage next to the running one (`.<name>.update`), showing the
  megabytes done, checks its size and digest, gives it the running
  file's permissions and moves it over the running file. A short, long
  or mismatched download is deleted and the running file is left as it
  was; the dialog says why and offers Download. The new version runs
  from the next start. Windows, macOS, the `.deb` and the tar.gz keep
  Download and the system's installer.
- Options > Global > General, Check for updates at start (on by
  default): once a day, the same check runs in the background at start;
  only a newer release that was not skipped is shown, nothing otherwise.
- A failed check (no connection) says why and offers the releases page.

## Checks

- `updates::tests::versions_compare_by_number`
- `updates::tests::the_download_fits_the_system` (the file names of the
  v0.19.8 release, in GitHub's order and reversed)
- `updates::tests::no_file_of_another_system_is_offered`
- `updates::tests::the_x86_64_appimage_is_named_amd64_from_0_19_9` (the new
  name, the old one as fallback, the order an 0.19.8 updater sees, and the
  name in `build-appimage.sh`)
- `updates::tests::how_a_linux_copy_was_installed`
- `updates::tests::the_same_or_older_drafts_and_pre_releases_are_not_offered`
- `ui::dialogs::tests::every_dialog_draws_and_accepts_enter` (the dialog)
- Checked against GitHub from the app: 0.4.0 reads as the latest version.
- `updates::tests::the_appimage_comes_with_its_digest`
- `updates::tests::installing_replaces_the_file_only_when_it_checks_out`
- `ui::dialogs::tests::the_updates_dialog_draws_every_state`
- `updates::tests::live_install_from_the_latest_release` (run by hand,
  needs the network): the 0.4.0 AppImage downloaded through GitHub's
  redirect, its digest checked and the file put in place.
  Run again against v0.19.8: the x86_64 AppImage is picked, downloaded
  and checked.
