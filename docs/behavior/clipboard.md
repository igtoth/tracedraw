# Copy and paste

## Copy (Ctrl+C) and Cut (Ctrl+X)

The selection goes to TraceDraw's own clipboard, which keeps everything
(styles, effects, links to object styles), and as SVG text to the system
clipboard, so other programs can paste it. Right after the copy TraceDraw
notes what the system clipboard holds, without reading it back: the text
it wrote (writing text replaces everything there), and on Windows the
system's clipboard change count. A copy that writes nothing there (nodes,
guidelines) reads it: the count on Windows, else its text and files, and
a sample of its picture (the size and about 4096 bytes) only when there
is neither. Ctrl+C with nothing selected changes nothing.

## Paste (Ctrl+V, Edit > Paste)

- The system clipboard still holds what the last copy here left there
  (or there is no system clipboard): the objects copied here are pasted.
  Ctrl+V finds that out as cheaply as it can (#183): on Windows an
  unchanged change count pastes without reading the clipboard at all;
  elsewhere its text and files decide, and its picture is decoded only
  when there is neither text nor a file, or when the picture is what
  gets pasted (and then once). A 4000 x 3000 picture on the clipboard
  costs about 0.1 s to decode, so Ctrl+C never reads it and Ctrl+V reads
  it only when it must.
- Something was copied in another program since, or nothing was copied
  here: what the system offers, in this order:
  1. files copied in a file manager: each is imported like File >
     Import (one undo step each); a line of the list that ends in CR LF
     loses the CR;
  2. SVG text: its objects;
  3. an image: a bitmap object;
  4. other text: artistic text in the middle of the page.

Ctrl+Shift+V pastes the objects copied here in the middle of the view.
Paste on the page's context menu centres what it pastes where the menu
was opened.

### In a browser and on Android (#1403)

A browser hands the clipboard only to a paste event and to its
clipboard API, never to a plain read:

- Ctrl+V: the paste event's text goes the way above; its files (a
  picture copied in an image program or a web page, files copied in a
  file manager) are read in the background and pasted a frame later.
  The shortcut's own frame then pastes nothing, so the copy made here
  does not come as well. One picture alone is placed as a bitmap; other
  files are imported like File > Import. While a text is edited, a
  picture is not put into it.
- Edit > Paste (and the context menu's Paste) reads the clipboard with
  the browser's clipboard API (`navigator.clipboard.read()`): the first
  time the browser asks for permission (Firefox shows a Paste button).
  Its picture and text are pasted as for Ctrl+V; text that is still the
  copy made here pastes the objects copied here. When the browser
  refuses or has no such API, the copy made here is pasted and the
  status bar says that Ctrl+V pastes what other programs copied.

On Android the activity reads the system clipboard (`ClipboardManager`):
its text, and the documents other apps copied (content URIs, which
reading the clip grants the editor the right to read). One picture alone
(copied in a gallery, a browser or a chat) is the clipboard's picture,
placed as a bitmap; other documents are imported like files from the
document picker. A document's URI is never pasted as text.

Symbol instances pasted into another drawing (or inside pasted groups and
clip frames) bring their symbols along: a symbol the drawing holds
already, the same objects under the same name, is used; any other is
added to the drawing's symbols, with the symbols it shows, in the same
step as the paste. In the drawing copied from, an instance stays an
instance of its own symbol (#678,
`ops2::tests::an_instance_pasted_in_another_drawing_brings_its_symbol`,
`symbol_paste::tests`).

## When the clipboard holds no text

The windowing layer turns Ctrl+V into a paste event only while the
system clipboard holds text; with only an image or files on it, it drops
the key press and sends nothing but the release of V. A release of V
with Ctrl (Cmd), Shift+Insert or the Paste key that came without a paste
event or press before it pastes too, once. V let go after Ctrl is typing
and does not paste.

## Browser build

The browser hands the clipboard only to paste events, and only as text:
a copy notes the SVG it wrote, and Ctrl+V compares the paste event's
text with it. Images and files copied in other programs are not
available.

## Checks

- `clipboard::tests`: the copy here wins while the system clipboard
  holds what it left, or holds nothing; a picture or text copied
  elsewhere afterwards wins; copied files are imported; copied file paths
  lose the line end; the change check and the paste decode a picture only
  when they must (`the_change_check_decodes_a_picture_only_when_it_must`,
  `a_paste_decodes_the_picture_only_when_it_needs_it`).
- `clipboard::clipboard_bench::clipboard_costs` (ignored, needs a
  clipboard): what copy and paste cost with a 4000 x 3000 picture on it.
- `pick_drag_tests::ctrl_v_pastes_when_only_its_release_comes_through`
  and `copy_cut_and_paste_events_work_on_objects`.
- Checked under Xvfb with `xclip`: a picture (`image/png`) and a file
  (`text/uri-list` with CR LF) copied after a copy here paste as a bitmap
  and as an imported file; copying here again pastes the copy.
