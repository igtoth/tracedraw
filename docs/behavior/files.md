# Saving files

File > Save (Ctrl+S) writes the drawing back to its file. A drawing that
has none (new, or opened from a format TraceDraw only reads or converts,
`.cdr` included), and File > Save As (Ctrl+Shift+S), open the Save
Drawing dialog.

## Save Drawing

One window, 760 px wide, the same on every system:

- **Folder** (desktop): an Up button, the path bar (a folder typed there
  opens with Enter; one that does not exist says so) and New folder
  (makes "New folder", "New folder (2)" ... and opens it). Down the left,
  shortcuts to the folders that exist: Home, Desktop, Documents,
  Downloads and Pictures (on Linux the names in
  `~/.config/user-dirs.dirs`), then the drives (Windows) or `/`. On the
  right, 250 px high and scrolling, the folder's contents: folders, then
  the files of the chosen type, by name ignoring case (hidden ones left
  out), with Name, Date modified (local time), Type ("File folder",
  "CDR file") and Size (KB, MB from 1 MB). A click on a file takes its
  name; a double click on a folder opens it.
- **File name**: the drawing's file name, else its title, else
  "Graphic1", with the type's extension; a name typed with either
  extension, or none, gets the chosen type's.
- **Save as type**: TraceDraw (`*.tdraw`, the native JSON format, which
  keeps everything) or CDR files (`*.cdr`). It starts at `.cdr` for a
  drawing that came from or was saved to a `.cdr` file, and for a
  drawing without a file when Options > Global > File Formats > Save
  new drawings as .cdr files is on.
- **Version**: for TraceDraw files the `.tdraw` format version, a
  choice once there is more than one; while there is only one the row
  is left empty (`docs/tdraw-format.md`, #613); for `.cdr` 27 down to 14, each with its
  release name ("v14 (X4)", "v27 (2026)").
- **Embed colour profile (size)** with "Colour profile: name" (`.cdr`
  only): on, the drawing's CMYK profile is stored in the file; off, it
  is left out. Only version 16 and later store one, so the box is off
  and greyed for older versions and for a drawing without a profile
  ("Colour profile: none"). **Embed fonts (size)** sits beside it: on,
  the faces the text uses go into the file when their licence allows
  it; every face is listed in the file's font table either way. It is
  greyed below version 16 and when no face may be embedded. Both
  choices stay for the next Save As and for Save. Embed fonts starts
  off, so that drawings saved one after another do not each carry a copy
  of the same fonts (#886).
- **Embed fonts (size)** for TraceDraw files: on (it starts off, #886), the
  installed fonts the text uses go into the `.tdraw` file, one file per
  face (the regular face for a bold or italic the family lacks), where
  the font's licence allows use in documents, so the drawing keeps its
  letters on a machine without them; off, they are left out. Fonts that
  came with a drawing (`docs/behavior/text.md`, Fonts that come with a
  drawing) go along either way. The box is greyed when no such font may
  be embedded. The choice stays for the next Save As and for Save
  (`App::tdraw_embed_fonts`, `FontSystem::fonts_to_embed`).
- Only the versions checked in other programs are offered (14 to 27).
- **Title**, **Subject** and **Rating** (five stars; clicking the lit
  last star takes it off): the document's properties, also in File >
  Document Properties; changed ones are set (one undo step each) before
  the file is written.
- **Save** (or Enter) writes the file; a file that exists asks "<name>
  already exists. Replace it?" with Replace and Cancel. Cancel closes
  without writing. The dialog starts in the drawing's folder, else the
  last folder saved to, else Documents, else home.
- In the browser, where every save is a download, the dialog keeps the
  file name, type, version and properties, and says the browser saves
  the file in its downloads folder.

## `.cdr` versions

A program opens `.cdr` files of its own version and older ones, so the
version decides who can open a file:

- Save As starts at the drawing's own version when it was opened from
  (or saved to) a `.cdr` file, as Save keeps it; any other drawing starts
  at the version set in Options > Global > File Formats (27 at first),
  a drawing started from a template (`.cdt`, `.cdrt`) too, whatever
  version the template was written in.
  The hint under that version says so (#613,
  `pick_drag_tests::the_cdr_version_chosen_in_options_survives_a_restart_and_starts_save_as`).
- The version chosen stays with the drawing: Save writes the same
  version again without asking. Each open drawing keeps its own.
- TraceDraw saves only the versions checked in other programs, 14 to 27.
  A drawing opened from an older `.cdr` (versions 1 to 13) keeps its
  file, but Save does not write that version: the status line says
  which version Save writes ("Save writes version 14."), and Save opens
  "Older file version": the drawing came from version N, TraceDraw does
  not save in it, choose a version from 14 to 27; Choose Version...
  (or Enter) opens Save As on version 14, Cancel saves nothing. Once
  saved in a checked version, Save writes it without asking. A file
  newer than 27 is saved as 27. Options > Global > File Formats (version
  27 at first) is used for a drawing saved to a `.cdr` file without a
  version of its own.
- `tracedraw-cli convert in.svg out.cdr` writes version 27, or the
  version in `TRACEDRAW_CDR_VERSION` (14 to 27; another version is
  refused with exit code 2). Versions 7 to 13 are experimental and are
  written only with `TRACEDRAW_CDR_EXPERIMENTAL=1`, for testing them in
  other programs. The MCP `save` tool takes an optional `version`, 14 to
  27 (`mcp.md`).

What each version's file holds is in `docs/cdr-format.md`, Writing.

## Checks

- Given a folder with a subfolder, a `.cdr` and a `.txt` file, the
  TraceDraw type lists the subfolder only and CDR the subfolder and the
  `.cdr` file; saving "poster" as CDR version 15 with a title, subject
  and rating writes `poster.cdr` there (version 15), sets the properties
  and remembers the folder; the next Save As starts there as `.cdr`
  version 27; an existing name asks first and Replace overwrites it; a
  TraceDraw file is written with its own extension, in the format version
  chosen (0), which Ctrl+S keeps and opening the file brings back
  (`ui::save_as::tests::save_as_chooses_folder_name_type_version_and_properties`).
- An empty name, a bare extension and a missing folder save nothing and
  say why; typed extensions become the type's
  (`ui::save_as::tests::names_and_folders_are_checked`).
- A version 21 file opened: Ctrl+S opens Save As (an opened `.cdr` is
  never overwritten in place), and once saved Ctrl+S writes it again; a
  version 9 file opened says Save writes version 14, Ctrl+S writes
  nothing and asks for a version, Save As opens on 14 and saves version
  14, after which Ctrl+S saves without asking; versions 3 and 99 are
  saved as 14 and 27
  (`ui::save_as::tests::a_drawing_opened_from_a_cdr_file_keeps_its_version`).
- Options set to version 15 and `.cdr` for new drawings: Ctrl+Shift+S on
  a new drawing opens Save As at CDR version 15; with the defaults it
  opens at TraceDraw, version 27 for `.cdr`
  (`pick_drag_tests::save_as_starts_at_the_cdr_type_and_version_the_options_set`).
- A drawing with a CMYK profile offers it by name and size; saved with
  the box on, the version 27 file holds the profile, with the box off it
  does not, and the next Save As keeps the box off
  (`ui::save_as::tests::the_cmyk_profile_goes_along_unless_left_out`).
- Text in an installed font whose licence allows embedding: Save As for
  `.tdraw` shows the fonts' size with the box on; the file saved holds
  the font, the one saved with the box off does not, and the next Save
  As keeps the choice
  (`ui::save_as::tests::installed_fonts_go_into_tdraw_files_unless_left_out`);
  the faces picked, the licence rule and the stand-in regular face:
  `tests::installed_fonts_the_text_uses_are_embedded_when_their_licence_allows`
  in `crates/text`.
- The dialog draws, folders open (`ui::save_as::tests::the_dialog_draws_and_opens_folders`),
  and the Linux folder names are read from the user's settings
  (`ui::save_as::tests::linux_folder_names_come_from_the_users_settings`).
- The MCP `save` tool writes version 21 when asked and refuses versions 3
  and 23 (`mcp::tests::export_and_save_write_files`).
- Every version from 7 to 22 reads back with its pages, layers, objects,
  text runs and alignment (`write::tests` in `crates/cdr`).
- Checked on screen: the dialog with its shortcuts, list, type list,
  version and stars; Save as CDR wrote the file in the folder shown.
