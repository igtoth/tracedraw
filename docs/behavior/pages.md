# Pages

## Duplicate Page

Layout > Duplicate Page..., the page tab menu's Duplicate Page... and the
Duplicate Page button of the Pages panel open the Duplicate Page dialog
for the current page:

- Insert new page: Before selected page or After selected page (the
  default).
- Copy layer(s) only: the copy has the page's layers (names, visibility,
  printing and lock) with no objects in them.
- Copy layer(s) and their contents (the default): the objects are copied
  too, with fresh ids.
- OK duplicates, Cancel and Esc close the dialog, "?" opens the Help
  (`pages-layout/pages#duplicate-page`).

The copy keeps the page size, guides and background, is named after the
page with " (copy)", and becomes the current page. It is one command,
`Command::DuplicatePage { page, before, contents }`, so one undo step
removes it. A saved or scripted command that names only the page copies
it with its contents after it, as before (Print Merge works that way).

Tests: `command::tests::duplicate_page_goes_before_or_after_with_or_without_contents`,
`command::tests::a_saved_duplicate_page_without_options_copies_contents_after`,
`ui::window_bars::tests::the_duplicate_page_dialog_puts_the_copy_where_asked_and_undoes`,
`ui::window_bars::tests::layout_duplicate_page_opens_the_dialog`.

## Page size on the property bar

With nothing selected the property bar starts with the page size list:
the favourites (A4, Letter, A5, A3, Legal), the sizes the user saved,
then every preset under its group heading (ISO A, ISO B and JIS B,
North American, Print shop, Posters and banners, Envelopes, Cards and
postcards, Photo, Screen and social media, Web ads), a line, then
Custom... and Edit This List.... No two presets share a size in either
orientation. The list shows the
preset the page matches (either way round) or Custom when it matches
none, and Custom... is then the entry ticked. A preset keeps the page's
orientation. Custom... opens the Page Size dialog with the page's width
and height, to type any size (#650, #1092); the page width and height fields
next to the list also take any size at once. Both resize the current
page or every page as the All Pages / Current Page buttons say.
Edit This List... opens Document Options > Page Size, where the
save button next to Size keeps a custom size under a typed name and the
delete button forgets the saved size shown; saved sizes are kept in the
preferences and listed in every page size list (the property bar, the
Page Size dialog, Document Options and Create a New Document).
Checked by `ui::propbar::tests::the_page_size_list_offers_a_custom_size`,
`ui::dialogs::tests::page_size_presets_are_distinct_and_keep_their_names`,
`ui::dialogs::tests::a_page_size_finds_its_name`.

## Fit Page to Selection, Fit Page to Drawing

Layout > Fit Page to Selection (pale with nothing selected) and Fit
Page to Drawing (pale on an empty page), also as buttons on the
Document Options > Page Size page and, Fit Page to Drawing, on the
property bar with nothing selected (after the page width and height):
the current page takes the size of
the selection's bounds (effects included), or of everything on the page,
and the page's objects move so that box sits on the page from its
bottom-left corner. One undo step (`App::fit_page_to_selection`,
`fit_page_to_drawing`); a box under 0.1 mm does nothing and the status
bar says so. The property bar's button opens a popup instead of fitting
at once: a Margin distance field (in the ruler unit; kept in the
preferences, `settings.fit_page_margin`, 0 at first) and an Autofit
Page button, which fits the page to the drawing with that margin left
free on every side (`App::fit_page_to_drawing_with_margin`; a margin
that is not a number counts as none). Checked by
`ops2::fit_page_tests::the_page_fits_the_selection_or_the_drawing` and
`ui::propbar::tests::the_page_bar_draws_with_its_fit_page_button`.
While the popup is open the page itself shows at the size Autofit Page
would give it, around the drawing, following the margin as it is typed;
nothing else is drawn, and closing the popup without OK puts it back
(#594, #604, `ops2::tests::the_autofit_preview_is_the_drawing_and_its_margin`).
A Live preview check box on the margin's row, right of the field (on at
first, kept in the preferences, `settings.fit_page_preview`), turns that
off: the page then keeps its size until Autofit Page is clicked (#659,
`ui::propbar::tests::the_autofit_popup_previews_only_with_live_preview_on`).
Clicks inside the popup (the box, the margin field and its arrows) keep
it open, so the preview can be switched and watched; a click outside or
Autofit Page closes it
(`pick_drag_tests::the_autofit_popup_stays_open_when_its_box_is_clicked`).

## Delete Page

Layout > Delete Page... and the Delete Page... button of the Pages panel
open the Delete Page dialog (#500): Delete page (the current page to start
with) and a Through to page check box with the last page of a range, off
to start with (the range may be given in either order; numbers are kept to
the drawing's pages). OK deletes the pages in one undo step; the page after
them, else the one before, becomes the current page. A range of every page
deletes nothing and the dialog says a drawing keeps at least one page. The
page tab menu's Delete Page deletes that page at once
(`app::tests::delete_page_takes_a_range_of_pages`).
