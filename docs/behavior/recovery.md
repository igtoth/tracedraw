# Session recovery

While Options > Save > Auto-backup is on, on the desktop, in the Android
app and in the browser (see "In the browser" below).

Each running TraceDraw keeps a small marker, `session-<id>.json`, in the
auto-backup folder (`TraceDraw` in the temporary folder unless Options >
Save names another). It holds the time the session last checked in
(once a minute) and the names of the auto-backups it wrote
(`AutoBackup_of_<name>.tdraw`, every 20 minutes by default, see
`options.md`).

- Closing TraceDraw normally deletes the marker and the auto-backups that
  session wrote (unsaved drawings were already asked about).
- At start, a marker that has not checked in for 3 minutes belongs to a
  session that ended without closing (a crash, a power cut). When its
  auto-backups still exist, a question names them: Recover opens each as
  an unsaved drawing without a file (Save asks where), and deletes the
  backup that opened; Discard leaves the backups in the folder and forgets
  the marker. A marker without backups is forgotten silently.
- Two TraceDraw windows running at once each keep their own marker; one
  never offers the other's drawings while it still checks in.
- Auto-backups are written on a thread of their own from a copy of each
  drawing (pictures shared, not copied), straight to the file as the JSON
  is made, so the window keeps working while a large drawing is backed
  up. Each is written as `AutoBackup_of_<name>.tdraw.part` and renamed
  when complete, so a backup cut short never replaces a whole one. The
  status line says "Auto-backup saved" on the frame the thread is done; a
  clean exit waits for a backup still being written and then removes it
  with the others. Save (`.tdraw`) also writes straight to the file.
- A drawing that has not changed since its last auto-backup is not
  written again.
- Backing up at once (`App::backup_now`): eframe's `App::save` writes the
  due auto-backups without waiting for the minutes, and deletes the
  backups of drawings saved or closed since. It runs when the program
  closes, when the Android app goes to the background (the system may end
  it there without another word) and in the browser as below.
- A program closed with unsaved drawings left (`App::leave_session`): the
  system quit it without asking (the Quit of the macOS menu, a log-out) or
  the renderer check closed the window. The drawings are backed up, the
  backups stay and the marker is written with a check-in time of 0, so the
  next start offers them at once. With no drawing left unsaved (File >
  Exit and the close button ask about each first) the exit is clean.
- An idle window still checks in: it draws a frame each minute while it
  has a marker, so another window never takes it for gone.
- The Android app is one process: a marker found when it starts belongs to
  a session the system ended, however recently it checked in.

## In the browser

The browser has no folder; the page keeps both in its storage for the
site (`web::backups`, `files::WEB_BACKUP_DIR`, the path `/backup`):

- Markers go to local storage (key `tracedraw.recovery:/backup/session-<id>.json`),
  written at once, so a page being left can still mark its session gone.
- Backups go to the IndexedDB database `tracedraw` (store `backups`), keyed
  by `/backup/<session id>/AutoBackup_of_<name>.tdraw`: each tab writes in
  a folder of its own, so two tabs never write over each other's. They are
  written on the spot (a browser has no threads). Options > Save shows no
  folder.
- The database is read once when the page opens; the recovery waits for
  it. Backups that no marker names any more are deleted then; those of
  tabs still open are not kept in memory.
- A timer checks the marker in every 30 seconds, as a hidden tab draws no
  frames.
- Leaving the page (closing or reloading the tab, going back, following a
  link away) with unsaved drawings asks first with the browser's own
  question (`beforeunload`; the browser words it, and Ctrl+W or the
  close button cannot be kept from it). The page backs up at that moment,
  while the question shows, and again when it is hidden (another tab, a
  minimized window, a phone's home screen), as a hidden page may be
  discarded with no other event.
- Left for good (`pagehide`, not into the back-forward cache): the marker
  is marked gone and the session ends as a closing program's does, so the
  next visit, a reload included, offers what was left at once. A page with
  nothing unsaved leaves nothing.
- Discard deletes the offered backups as well (nothing outside the page
  can reach them). Clearing the site's data removes everything.

## Checks

- `recovery::tests::a_session_that_stops_checking_in_leaves_its_backups_to_recover`
- `recovery::tests::a_clean_exit_removes_the_marker_and_its_backups`
- `recovery::tests::broken_markers_never_reach_outside_the_folder`
- `app::tests::drawings_a_session_left_without_closing_are_recovered`
- `app::tests::a_program_quit_without_asking_keeps_its_unsaved_drawings_for_the_next_start`
- `app::tests::a_clean_exit_leaves_no_backup_behind`
- `app::tests::backing_up_at_once_writes_changed_drawings_and_drops_saved_ones`
- `recovery::tests::a_marker_kept_alive_or_marked_gone_keeps_its_backups`
- `recovery::tests::a_session_forgets_the_backup_of_a_saved_drawing`
- `app::tests::saving_over_a_file_keeps_a_backup_and_auto_backup_saves_dirty_drawings`
- `document::tests::tdraw_json_streams_to_the_writer` (core)
- `engine::tests::a_drawing_marked_unsaved_stays_so_until_saved` (core)
