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

On Windows the cursor's place is read from the system while files are
dragged, since the system drag sends the window no pointer moves. Where
the windowing layer does not say where the pointer is (Linux, macOS), a
file File > Open reads opens in a tab of its own, like a `.tdraw`, and a
picture is imported into the drawing shown.

Several files dropped on the Welcome Screen open side by side, one tab
each. Imports go in the middle of the view, not under the pointer;
File > Import's placement option (Options > Global > File Formats) is
not used for drops.

## While files are dragged over

A 3 px accent frame over a light accent tint goes round where the files
would land: the drawing window when they would be imported there, the
whole window otherwise. In its middle a label says what a drop would do:
"Drop to import into the drawing" or "Drop to open".

## Browser build

The browser hands over the bytes, not a path: they are read in the
background, kept under `/upload/<file name>` like a file picked in a
dialog, and opened or imported on a later frame.

## Checks

- `drop_files::tests` drop real `.svg`, `.png` and unknown files through
  egui's input: over the drawing window an import lands centred in the
  view and one undo removes it; over the menus the `.svg` opens in a new
  tab and the `.png` gets a new drawing; with the pointer unknown the
  `.png` is imported and the `.svg` opens in a new tab; without a drawing the `.svg` opens in a new tab and the `.png`
  gets a new drawing with the picture centred on the page; two files on
  the Welcome Screen give two tabs; an unknown file only sets the status.
