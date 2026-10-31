# Dropping files on the window

Files dragged from the desktop or a file manager (in a browser: from the
computer onto the page) and let go over the TraceDraw window open or
import, with no dialog.

## What a drop does

Where the files are let go decides for every file let go together:

| Let go over | The file | Result |
|---|---|---|
| the drawing window of the drawing shown | File > Import reads it (vector formats, `.cdr`, `.psd`/`.psb`, pictures, installed script filters) | imported into the active layer, selected, as one undo step, its middle at the middle of the drawing window |
| the drawing window | only File > Open reads it (`.tdraw`) | opened in a tab of its own |
| the title, menus, toolbars, document tabs or dockers, with a drawing shown | File > Open reads it (`.cdr`, `.svg`, `.tdraw`...) | opened in a tab of its own, as File > Open (#1142) |
| the same | a picture or a script filter's format | a new drawing with the file imported in the middle of its page |
| anywhere, with no drawing shown (Welcome Screen or no drawing) | File > Open reads it | opened in a tab of its own, as File > Open, or its tab shown when it is open already (#645) |
| the same | a picture (`.png`, `.jpg`, `.jpeg`, `.bmp`, `.gif`, `.webp`, `.tif`, `.tiff`) or a script filter's format | a new drawing with the last used new-document settings, the file imported in the middle of its page |
| anywhere | anything else | nothing; the status bar says the file cannot be opened or imported |

The place decides the same way on every platform (#1142). A system drag
sends the window no pointer moves, so: on Windows the cursor's place is
read from the system; on Linux (X11) the patched windowing layer turns
each XDND position message into a cursor move over the window, and on
macOS each `draggingEntered:`, `draggingUpdated:` and
`performDragOperation:`; in the browser the patched eframe turns each
`dragover` and the `drop` into a pointer move (`vendor/README.md`).
Should the place still be missing, a picture is imported into the drawing
shown, a `.tdraw` opens in a tab of its own, and a file that could go
either way (`.cdr`, `.svg`...) is asked about in a small window in the
middle: "Import into the drawing", "Open in its own tab" or Cancel (Esc).
Native Wayland windows get no file drops from winit 0.30 at all; a
Wayland desktop runs TraceDraw through XWayland for them.

Several files dropped on the Welcome Screen open side by side, one tab
each. Imports go in the middle of the view, not under the pointer;
File > Import's placement option (Options > Global > File Formats) is
not used for drops.

## While files are dragged over

A 3 px accent frame over a light accent tint goes round where the files
would land: the drawing window when they would be imported there, the
whole window otherwise. In its middle a label says what a drop would do:
"Drop to import into the drawing" or "Drop to open". Over the drawing
window a `.tdraw` says "Drop to open" with the frame round the whole
window, as it opens in a tab of its own; in the browser, which names no
files while they are dragged, the place alone decides
(`drop_files::tests::the_hint_says_what_a_drop_there_does`).

## Browser build

The browser hands over the bytes, not a path: they are read in the
background, kept under `/upload/<file name>` like a file picked in a
dialog, and opened or imported on a later frame.

## Checks

- `drop_files::tests` drop real `.svg`, `.png` and unknown files through
  egui's input: over the drawing window an import lands centred in the
  view and one undo removes it; over the menus the `.svg` opens in a new
  tab and the `.png` gets a new drawing; a drag's place reported as
  pointer moves decides between import and open
  (`the_place_of_a_drag_decides_on_every_platform`); with the pointer
  unknown the `.png` is imported and the `.svg` is asked about, then
  imported, opened or left as answered; without a drawing the `.svg` opens in a new tab and the `.png`
  gets a new drawing with the picture centred on the page; two files on
  the Welcome Screen give two tabs; an unknown file only sets the status.
