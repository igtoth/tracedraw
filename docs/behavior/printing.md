# Printing

File > Print (Ctrl+P) builds a print-ready PDF of the sheets and sends
it to the printer chosen in the dialog, or opens it in the system
viewer, where the printer is chosen (in the browser the PDF is
downloaded). `export::print` builds it; the sheets come from
`imposition.rs`.

## Printers

The Printer list holds the system's printers in name order (ignoring
case; the default stays where its name puts it), then "System printer
(through the PDF viewer)", "PDF file" and "PostScript file". The list
is read off the interface's thread when the dialog opens, so a print
server that is slow to answer never holds the window: until it comes,
the list read last shows (or "Looking for printers..." the first
time, when Print waits for the list if a printer is the destination),
and the dialog takes the new list as soon as it is read, keeping a
destination picked meanwhile unless it names a printer the system does
not have (#614).
The printers come from CUPS (Linux and macOS): `lpstat -l -p -d` lists
them with their state, description and location (`printers.rs`,
`parse_lpstat`), and a print on one of them hands the PDF to `lp -d`
with the number of copies and their collation (`lp_args`) and the
drawing's name as the job title; the status bar says "Sent to printer
name", or what `lp` complained about. The dialog starts on the printer
printed to last when the system still has it, else on the default
printer, selected where it stands in the list, else on the PDF viewer
(`starting_printer`); the printer is
remembered with the other print choices (Options > Global > Printing).
On Windows the printers are every printer Windows has, the ones
installed on the computer and the network printers connected for the
user alike: PowerShell lists `Win32_Printer` when the dialog opens
(`WINDOWS_LIST`, `parse_windows_list`, #614) with each printer's
PrinterStatus and WorkOffline (Ready, Printing, Warming up, Stopped,
Offline, else Unknown), Location and Comment. A
print on one of them draws each sheet at 300 dpi and hands the pictures
to the printer through PowerShell's printing classes
(`windows_print_script`), so no PDF program is needed: each sheet takes
the printer's paper of its size when the printer has one (either way
round, within 2 mm), a wide sheet turns the paper, and the sheet is
drawn at its size from the paper's corner; the copies and their
collation go to the printer, the drawing's name is the job title. The
script runs without a window and shows no "Printing page" box, and gives
up after five minutes; what came of each print (the sheets sent, or the
error) is written to `tracedraw.log`. A one-sheet print used to fail
before it reached the printer, on every printer: the list of sheet sizes
flattened in PowerShell when it held one sheet (#614; checked on a
Windows machine with a printer writing to a file: one and two sheets
come out). In the browser the list has no printers and the print goes
through the PDF viewer.

Next to the list a Properties button (a gear, available when a system
printer is chosen) opens the printer's own settings: on Windows its
printing preferences (`rundll32 printui.dll,PrintUIEntry /e /n name`),
on macOS the system's Printers & Scanners settings, on Linux and other
CUPS systems the printer's page of the CUPS web interface in the browser
(`http://localhost:631/printers/name`, a web page, not a system dialog;
`properties_command`). Under
Orientation, Status, Location and Comment show what the system says of
the printer chosen (empty for the PDF viewer and the files, #614).

## Print dialog

Tabs: General, Color, Composite, Layout, Prepress, PostScript and the
preflight's Issues ("No Issues" or the count; hidden when Options >
Global > Printing turns the preflight off). Under them Print Preview
(hands the settings to the preview), Print, Cancel and Apply (keeps the
settings as Options > Global > Printing's). The dialog starts with those
preferences, and with the drawing's own bleed as the bleed limit when it
has one.

- General:
  - Destination: Printer is one of the system's printers (above), the
    system PDF viewer (which owns the printer and the copy count; in the
    browser the PDF is downloaded), a PDF file or a PostScript file (saved
    where chosen). Orientation: Match orientation (the Layout tab's
    Landscape box), Portrait or Landscape, for fixed sheets.
  - Sheet: the page's own size, the standard sizes, Smart Board 16:9
    (1354.582 x 762 mm) and Custom (width and height typed; the last one
    is kept). A Custom size saved with a name joins the list for the next
    time (Options keeps them, #591).
  - Print range: Current document, Current page, Pages (a list such as
    `1-3, 5, 8-`; an empty or unusable list prints them all), Selection
    (the selected objects of the current page).
  - Copies: a file gets its copies inside, as whole sets (Collate) or
    each sheet repeated.
  - Print as bitmap: each sheet becomes one picture at the resolution
    given (36 to 1200 dpi).
- Color: Composite or Separations (one grey page per ink and sheet: C, M,
  Y, K, then the spot inks; black = full ink); Convert spot colors to
  CMYK prints spot inks as their CMYK mix at their tint. The printed
  sheets keep the drawing's list of spot inks, so each spot ink gets its
  plate and its box in the print preview; before, the sheets lost the
  list and only the four process plates printed (#779,
  `export::tests::separations_print_a_plate_for_each_spot_ink`).
- Composite: Print text in black (fills, outlines and character colours
  of every text); Always overprint black (100 % black fills and outlines
  overprint).
- Layout: As in document, Fit to page or Scale (percent). Fit to page
  also brings objects that run off the page inside it: the objects the
  page shows (every printed one when the range is the selection) shrink
  as one to fit it, centred, when they do not fit; objects wholly off the
  page stay out (#534, `fit_contents_to_page`,
  `export::tests::fit_to_page_shrinks_what_runs_off_the_page`). Bleed limit
  (how far beyond the page objects print, 4 mm by default); the
  imposition, sheet, gap and the sketch (below).
- Prepress: Invert and Mirror (the whole sheet, marks included; Invert
  prints a negative, the sheet's empty paper black too, so black ink
  shows clear rather than white on white paper,
  `export::tests::black_text_prints_black_and_a_negative_blackens_the_sheet`); Print
  file information (with its text, the drawing's name when left empty)
  and Print page numbers (each page's number under it); Crop/fold marks;
  Registration marks; Color bars and Densitometer scale.
- PostScript: with a PostScript file as the destination, the language
  level written in its header (2 or 3).
- Issues: what the preflight finds, and the way to its dialog.

## Print Preview

File > Print Preview, or Print Preview in the Print dialog (which hands
over its settings), fills the window with the sheets exactly as Print
sends them: the imposition, the marks, Mirror and Invert, built by the
same function (`export::print_sheets`).

- Top bar: Print (prints these settings and closes), the sheet shown with previous and next (Page Up, Page Down), Zoom from 25 to 400 % of the fitted size
  with Fit, and Close (Esc) at the right end (#534).
- Left, at the top: the printer, as in the Print dialog's General tab
  (the system's printers in name order, the PDF viewer, PDF and
  PostScript files; read off the interface's thread when the preview
  opens, the list read last shown meanwhile), the gear button that
  opens the chosen printer's own Properties, its status, and the number
  of copies with Collate. Print in the top bar prints there (#792,
  `print_preview::tests::the_preview_offers_the_printer_and_the_copies`).
- Left: the Print dialog's options (all pages, fit, scale, prepress and
  layout with its sketch); every change rebuilds the sheets at once.
  While Print separations is on, a box for cyan, magenta, yellow, black
  and each spot ink stands indented under it, all ticked at first
  (#571; `print_preview::tests::the_ink_boxes_stand_under_print_separations`).
  Every ink ticked shows the sheet as it is; one alone shows its plate
  in grey; some show the colours those inks print together (process
  colours keep the channels ticked, a spot colour shows when its ink is
  ticked and is paper otherwise;
  `print_preview::tests::plates_split_the_sheet_by_ink`).
- A plate shows the ink's coverage in grey, black = full ink, as the
  separations print; registration colour shows on every plate.
- Paper margins (left, right, top, bottom, mm, 0 at first) keep the
  paper's edges clear: the pages and their marks go in the middle of what
  is left, shrinking to fit inside it unless Keep size is on; a sheet
  fitted to the pages grows by them (`imposition::tests::pages_sit_inside_the_paper_margins`).
  They are typed in the left panel or set by dragging their dashed blue
  lines on the sheet (within 6 pixels; the pointer turns to a resize
  arrow and the line under it is drawn bolder; a drag inwards makes the
  margin larger, 0 to 200 mm; #580, #582).
- A drag on the sheet moves the artwork on its page (the move cursor and
  a tip say so), clear of the margins a printer leaves blank; Artwork
  position on the left shows and takes the move in millimetres (x right,
  y up) with Reset. Print prints it so: the imposition moves every page
  of the sheet by it, its clip and marks with it (#536,
  `export::tests::the_artwork_moved_in_print_preview_prints_moved`).
- Transform (left panel, off at first) draws eight handles around the
  artwork's cell: a corner or side handle stretches it on that axis (the
  opposite side stays put, the resize cursor shows), a drag inside moves
  it; the stretch shows as x and y percentages there, typed or reset to
  100 % (#595). The handles come before the margin lines, which come
  before moving.
- Stretch to fill (a fixed sheet): the pages stretch on each axis on
  their own to fill the room inside the paper margins (#591,
  `imposition::tests::pages_stretch_to_fill_and_follow_the_preview_transform`).
  With the page information on, "Drag moves the page information" makes
  a drag move that instead (#388).

## Imposition

- Pages keep their aspect ratio; each cell is as large as the largest
  page on its sheet and a smaller page sits in its middle.
- A booklet's page count is rounded up to a multiple of four with blank
  pages at the back. The sheet sides pair the pages as a folded booklet
  needs: for eight pages 8 and 1, 2 and 7, 6 and 3, 4 and 5 (left and
  right). Its two pages meet at the spine with no gap and no bleed between
  them.
- Back side (n-up and step and repeat, #387): the columns run right to
  left, so a page's place on the sheet mirrors its place on the front
  about the sheet's middle and it prints behind its front on a
  double-sided sheet; the pages are not mirrored (unlike Mirror)
  (`imposition::tests::the_back_side_runs_the_columns_right_to_left`).
- Keep page size (a fixed sheet, #388): the pages are never scaled, not
  to make room for the marks (which take the room there is) nor to fit
  the grid: Several Pages per Sheet and Step and Repeat keep the columns
  and rows as set, and pages that do not fit run off the sheet; the
  sketch shows the sheet's part and a warning says what runs off is not
  printed (#496, `imposition::overflows`,
  `imposition::tests::kept_size_keeps_the_grid_and_the_page_size`).
  Without it the grid shrinks to fit inside the marks. Columns and rows go
  up to 99. The page information can be moved:
  with it on, a drag on the sheet in Print Preview moves it (an offset in
  mm, "Page information back in place" resets it)
  (`imposition::tests::kept_size_does_not_shrink_pages_for_marks`).
- Every page is clipped to its trim box plus the bleed, so nothing spills
  onto its neighbours or the margin. Hidden and non-printing layers are
  left out; master layers print with every page they show on; a page
  background prints under its page.

## Marks

With any mark on, a 12 mm margin (plus the bleed) surrounds the pages.

- Crop marks: 5 mm lines in registration colour, 0.1 mm wide, starting
  1.5 mm beyond the bleed, at every cut line of the grid, outside the
  pages only. A booklet also gets a dashed fold mark at the spine.
- Registration marks: a target (circle and cross) centred on each side
  of the pages.
- Color bars: patches of 4 mm of cyan, magenta, yellow, black, red,
  green, blue (two-ink overprints) and 50 % tints of the four inks, left
  of the top target.
- Densitometer scale: black from 0 to 100 % in steps of ten, right of the
  top target.
- Page numbers: each page's number, 6 pt, under the page.
- Page information: the drawing's name (or the text given), the pages on the sheet, "sheet n
  of n" and the date and time, in 6 pt under the pages.

## Checks

- `printers::tests::the_printer_list_is_read_from_lpstat`, `printers::tests::the_dialog_starts_on_the_remembered_printer_else_the_default`, `printers::tests::lp_is_told_the_printer_copies_and_title`
- `printers::tests::the_windows_printer_list_holds_network_printers`, `printers::tests::the_windows_print_script_names_the_printer_copies_and_sheets`
- `printers::tests::properties_open_the_printers_own_settings`, `pick_drag_tests::the_print_dialog_reads_the_printers_off_the_interface_thread`, `pick_drag_tests::a_destination_picked_while_the_printers_are_read_stays`
- `export::tests::page_lists_and_ranges_pick_pages`
- `export::tests::print_colour_options_change_the_sheets_objects`
- `export::tests::copies_repeat_sheets_collated_or_not_and_ps_gets_its_level`

- `ui::print_preview::tests::plates_split_the_sheet_by_ink`
- `ui::print_preview::tests::the_settings_key_ignores_copies_and_preflight`

- `imposition::tests::booklets_pair_the_pages_for_folding`
- `imposition::tests::n_up_fills_sheets_in_reading_order_and_scales_to_fit`
- `imposition::tests::fitted_sheets_grow_with_marks_bleed_and_gutters`
- `imposition::tests::imposing_places_and_clips_every_page`
- `export::tests::the_print_dialog_settings_make_the_layout`
- `export::tests::colours_are_mapped_inside_groups_frames_and_text`: the
  separations and Invert reach into groups, clip frames and character
  fills.
