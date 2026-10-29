# TraceDraw options

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
sets how the program works. Choose a page from the list on the left.

## General {#general}

![The General page of the TraceDraw options](../../img/customize-options-app-general.png)

- [[options.on_startup]]: show the Welcome Screen, start a new document, or
  open the last edited document.
- [[options.show_new_document_dialog]] (on): ask for the settings of a new
  drawing; see [Creating a drawing](../get-started/new-document.md).
- [[options.show_splash]] (on).
- [[options.hardware_acceleration]] (on): draw the window with the graphics card chosen beside it ([[options.gpu_automatic]] at first); off, with the computer's software renderer. It takes effect at the next start. If the window cannot open that way, TraceDraw starts again with the software renderer, then with OpenGL, and keeps the one that worked.
- [[options.undo_levels]]: how many steps you can undo (150 at first).

## Display {#display}

![The Display page of the TraceDraw options](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: show colors as they will print.
- [[options.show_tooltips]] (on), [[options.hide_bbox_curve_tools]] and
  [[options.node_tracking]] (off: when on, the [[tool.pick]] tool can drag the
  nodes of a selected curve).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (at first) or
  [[options.wheel_scroll]].
- [[options.open_view]]: [[options.open_view_page]] (at first),
  [[options.open_view_objects]], [[options.open_view_width]],
  [[options.open_view_height]], [[options.open_view_actual]] or
  [[options.open_view_saved]] (the page, place and zoom kept each time a
  TraceDraw file is saved).
- [[options.ruler_calibration]]: hold a ruler against the screen and change
  the [[options.screen_resolution]] until the marks match, so that zoom 100%
  shows real sizes. [[options.reset]] puts back 96.

## Edit {#edit}

![The Edit page of the TraceDraw options](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: the step of <kbd>Ctrl</kbd> when rotating and
  drawing (15 degrees at first).
- [[options.drawing_precision]]: the decimal places on the property bar and
  status bar (3 at first).
- [[options.focus_inactive]] (20% at first) and
  [[options.focus_double_click]]; see [Focus mode](../objects/focus-mode.md).

## Nodes and Handles {#nodes-and-handles}

![The Nodes and Handles page of the TraceDraw options](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: small, medium or large.
- [[options.node_shape]] for [[options.node_cusp]], [[options.node_smooth]]
  and [[options.node_symmetrical]] nodes.
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: the [[options.main_color]] of selected nodes and handles
  and the [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Clip Frame {#clip-frame}

![The Clip Frame page of the TraceDraw options](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (at first),
  [[options.auto_center_always]] or [[options.auto_center_never]].
- [[options.empty_frame_lines]] (on).

See [Placing objects inside a frame](../objects/clip-frames.md).

## Snapping {#snapping}

![The Snapping page of the TraceDraw options](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], the baseline grid
  and [[options.snap_to_pixels]].
- [[options.snapping_radius]] (10 pixels at first).
- [[options.show_snap_marks]] and the [[options.snap_screen_tip]].
- [[options.snap_modes]]: the points objects snap to, with
  [[options.select_all]] and [[options.deselect_all]].

See [Snapping](../pages-layout/snapping.md).

## Save {#save}

![The Save page of the TraceDraw options](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (on) keeps the previous
  file as `backup_of_<name>`, [[options.backup_to]] the
  [[options.same_folder]] or a chosen place.
- [[options.auto_backup]] (on): [[options.backup_every]] 20
  [[options.minutes]], to a temporary folder or a chosen place. After a crash
  the backups are offered; see [Auto-backup and recovery](../get-started/open-and-save.md#recovery).

## Text {#text}

![The Text page of the TraceDraw options](../../img/customize-options-app-text.png)

The page has five tabs.

- [[options.display]]: [[options.keyboard_text_increment]] (how much <kbd>Ctrl+8</kbd>
  and <kbd>Ctrl+2</kbd> grow and shrink the selected text, 1 point), the
  [[options.default_font]] and [[options.default_size]] of new text, and
  [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] and the [[options.hyphenation_patterns]]
  (built-in, or a hyphenation dictionary for your language).
- [[docker.fonts]]: [[options.show_recent_fonts]] at the top of the font lists,
  the [[options.recent_font_count]] (5, up to 20) and the
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  and [[options.add_to_autocorrect]], so a word you correct in the spell checker
  is corrected as you type next time; and the [[options.dictionaries]] it uses,
  see [Dictionaries](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: what is corrected as you type; see
  [Autocorrect](../text/writing-tools.md#autocorrect).
