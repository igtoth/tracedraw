# Font Manager

Tools > Font Manager (and Launch > Font Manager) opens a window of its
own, resizable (1000 x 720 at first), over the drawing. Like the other
dialogs it keeps the rest of the application waiting until it closes.

## Layout

- **Menu row**: File (Add Folder..., Add Font...), Edit (Add to
  Collection, Remove from Collection, Select None), Font (Install,
  Uninstall, Use in Drawing), View (Show Details) and Help (Font Manager
  Help).
- **Toolbar**: Add Folder, Add Font, Install, then Delete (out of the
  collection shown, or the folder shown out of the libraries), then Add
  to Collection (a list of Favorites, the collections and New
  Collection), Remove from Collection and Uninstall, then Show Details
  and Sample Text (Family Name or a preset). Each greys out when it
  cannot act on the selection.
- **Libraries** (top left), in folding sections with a blue title and
  line: Folders lists My fonts (the installed fonts) and the folders
  added; My Collections lists Favorites, the collections and New
  Collection. Double-clicking a collection renames it; right-clicking
  offers Rename and Delete Collection, and on a folder Remove Folder from
  Libraries. Dropping faces dragged from the list on Favorites or a
  collection adds them.
- **Filters** (bottom left), folding sections of check boxes: Font Status
  (Installed, Not installed, Protected system fonts, Duplicates),
  Embedding Rights (Installable, Editable, Preview and print, Restricted,
  Bitmap only), Font Technology (TrueType, OpenType with PostScript
  outlines, Variable), Weight (Thin to Black), Width (Condensed, Normal,
  Expanded), Style (Upright, Italic), Character Range (Latin to Chinese,
  Symbols) and OpenType (standard ligatures, kerning, small capitals,
  figure styles, fractions, swashes, stylistic alternates and set 1).
  Within a section any checked box lets a face through; every section
  with a checked box must. Clear unchecks them all.
- **Right side**: the sample text (empty: each face shows its family's
  name) with a list of presets, and a Search field matching family,
  style or PostScript name. Under them the faces of the library, sorted
  by family: the family's name large and faded, drawn in its first face,
  then a card per face with "Family (Style, Width)" above the sample
  drawn in that face (samples in the theme's text colour, light on the
  dark themes, #380), a technology mark on the right (O for OpenType and
  variable fonts, TT for TrueType) and a bar at the right edge, green
  when installed and grey when not, with a lock on the system's own
  fonts. The pointer over a card shows the family and its status. Only
  the cards in view are drawn; samples are drawn once and kept.
- Click picks a face, Ctrl+click adds or removes one; double-clicking an
  installed face sets it as the text font (and applies it to the
  selected text). Right-click: Install, Uninstall, Use in Drawing, Add to
  or Remove from Favorites, Add to Collection, Remove from Collection,
  Show Details.
- **Status line**: "<library>: N fonts" (or the result of the last
  action, or "Loading fonts..."), and a slider for the sample size.
- **Details** (Show Details): family, style, PostScript name, version,
  file, technology, embedding rights, weight, width, glyph count,
  scripts, OpenType features and status of the face clicked last. The
  pane takes its 270 px from the list: the window keeps its width when it
  is shown (#381).
- Fonts the drawing names that are not installed are listed above the
  faces in orange, as in the status bar.

## Installing

- A folder added is scanned for font files (TTF, OTF, TTC, OTC) three
  levels down, off the interface thread, and its fonts are loaded for
  TraceDraw at once and at every start while the folder is in the
  libraries, so they can be used in drawings without Install. Remove
  Folder takes them out again, except files installed on their own or
  in another folder still added; the system's fonts are never taken out
  (#866, `ui::font_manager::tests::the_fonts_of_an_added_folder_are_usable`).
- Install loads the picked faces' files into the font database for this
  session and keeps their paths in the settings: TraceDraw loads them
  again at every start. Add Font installs one file at once.
- Uninstall takes out files installed for TraceDraw; the system's own
  fonts (in the operating system's font folders) are protected and are
  never uninstalled.
- Folders, installed files, Favorites, collections, the sample text and
  size and Show Details are kept in the settings.
- In the browser, Add Folder is greyed (there are no folders to read) and
  Add Font loads the uploaded file for the session.

## Checks

- A face is described from its file: family, weight, width class,
  glyph count, Latin script, PostScript name and version; embedding
  rights follow the licence bits
  (`catalog::tests::a_face_is_described_by_what_its_file_says`,
  `catalog::tests::embedding_rights_follow_the_licence_bits`).
- A folder with two font files in it and a subfolder, a text file and a
  broken file lists the two faces as not installed
  (`catalog::tests::folders_are_scanned_for_font_files`).
- A font file installed shows up as a family and in the catalog, once,
  and uninstalling takes it out
  (`catalog::tests::font_files_are_installed_and_uninstalled`).
- Rows group faces by family, upright first; filters combine within and
  across sections; the search matches family, style and PostScript name
  (`ui::font_manager::tests::rows_group_faces_by_family_upright_first`,
  `ui::font_manager::tests::filters_combine_within_and_across_sections`).
- Favorites and collections list the faces they keep and survive the
  settings file; a folder's face shows as installed once its file is
  (`ui::font_manager::tests::collections_and_favorites_list_the_faces_they_keep`,
  `ui::font_manager::tests::a_folder_lists_its_fonts_installed_or_not`).
- The window draws with faces and details
  (`ui::font_manager::tests::the_window_draws_with_faces_and_details`),
  keeps its width when the details pane is shown
  (`ui::font_manager::tests::the_details_pane_does_not_widen_the_window`)
  and draws the samples in the theme's text colour
  (`ui::font_manager::tests::samples_take_the_theme_text_colour`).
