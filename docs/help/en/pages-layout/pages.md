# Working with pages

## Moving between pages {#go-to-page}

- Click a page tab under the drawing window, or use the page navigator beside
  them (first, previous, next and last page).
- Press <kbd>PgUp</kbd> or <kbd>PgDn</kbd>.
- Choose [[menu.layout > menu.layout.go_to_page]], type the page number in the
  [[dialog.go_to_page]] dialog box and click [[dialog.ok]].

![The Go To Page dialog box](../../img/pages-layout-pages-go-to-page.png)

## The Pages panel {#pages-docker}

The [[docker.pages]] panel lists the pages of the drawing. Click a page to go
to it. Its buttons [[menu.layout.insert_page]], [[menu.layout.delete_page]] and
[[menu.layout.duplicate_page]] act on the current page, and
[[menu.view.page_sorter]] shows every page as a thumbnail. Open it from
[[menu.window > menu.window.dockers]].

![The Pages panel](../../img/pages-layout-pages-pages-docker.png)

## Inserting pages {#insert-page}

1. Choose [[menu.layout > menu.layout.insert_page]], or click one of the insert
   buttons of the page navigator (they add one page of the current size
   before or after the current page at once).
2. In the [[dialog.insert_page]] dialog box, set the [[dialog.number_of_pages]]
   and choose [[dialog.before]] or [[dialog.after]] the current page.
3. Click [[dialog.ok]].

![The Insert Page dialog box](../../img/pages-layout-pages-insert-page.png)

## Deleting pages {#delete-page}

[[menu.layout.delete_page]] asks which pages to delete: the page number in
[[dialog.delete_page_from]] (the current page to start with), and to delete a
range, [[dialog.delete_through]] with the last page of it. All of them go in
one step, which [[menu.edit.undo]] brings back. A drawing keeps at least one
page. [[page_tab.delete]] on a page tab's menu deletes that page at once.

## Duplicating a page {#duplicate-page}

1. Choose [[menu.layout > menu.layout.duplicate_page]], or right-click the
   page tab and choose [[menu.layout.duplicate_page]].
2. Under [[dialog.insert_new_page]], choose
   [[dialog.before_selected_page]] or [[dialog.after_selected_page]].
3. Choose [[dialog.copy_layers_only]] for a page with the same layers and
   nothing on them, or [[dialog.copy_layers_and_contents]] to copy the
   objects too.
4. Click [[dialog.ok]]. The copy becomes the current page;
   [[menu.edit > menu.edit.undo]] removes it.

The page tab menu also has [[page_tab.insert_after]],
[[page_tab.insert_before]], [[page_tab.delete]] and
[[menu.layout.switch_orientation]].

## Renaming a page {#rename-page}

Choose [[menu.layout > menu.layout.rename_page]], or double-click the page
tab. Type the [[dialog.page_name]] and click [[dialog.ok]].

![The Rename Page dialog box](../../img/pages-layout-pages-rename-page.png)

## Page size and orientation {#page-size}

[[menu.layout > menu.layout.page_size]] opens the [[dialog.page_size]] dialog
box:

1. Type the [[dialog.width]] and [[dialog.height]]; [[dialog.swap]] exchanges
   them.
2. Tick [[dialog.apply_all_pages]] to change every page, or leave it off to
   change the current page only.
3. Click [[dialog.ok]].

![The Page Size dialog box](../../img/pages-layout-pages-page-size.png)

[[menu.layout.switch_orientation]] turns the page between portrait and
landscape. With nothing selected, the property bar of the [[tool.pick]] tool
also sets the page size and orientation. Its page size list has A4, Letter,
A5, A3 and Legal first, then the sizes you saved, then about 130 sizes in
groups (ISO A, ISO B and JIS B, North American, print shop, posters and
banners, envelopes, cards and postcards, photo, screen and social media, web
ads). **Custom...** opens the Page Size dialog box, and **Edit This List...**
opens the document options at the page size, where you can save a size under
a name. The full page settings are in
[Document options](document-options.md#page-size).

Pages you add, and their first layer, are named in the interface language.

## Page numbers {#page-numbers}

[[menu.layout > menu.layout.insert_page_number]] puts a page number
[[menu.layout.page_number_active]], [[menu.layout.page_number_all]],
[[menu.layout.page_number_odd]] or [[menu.layout.page_number_even]]. The
numbers on master layers show the right number on every page.
[[menu.object > menu.object.insert > menu.object.insert_page_number]] places
one in the text you are editing.

[[menu.layout > menu.layout.page_number_settings]] sets how they look:

- [[dialog.start_at]]: the first number;
- [[dialog.prefix]] and [[dialog.suffix]]: text around the number, such as
  "Page ";
- [[dialog.number_style]]: [[dialog.style_arabic]],
  [[dialog.style_roman_upper]], [[dialog.style_roman_lower]],
  [[dialog.style_alpha_upper]] or [[dialog.style_alpha_lower]];
- [[dialog.page_number_position]]: [[dialog.pos_bottom_center]],
  [[dialog.pos_bottom_outer]], [[dialog.pos_top_center]] or
  [[dialog.pos_top_outer]].

![The Page Number Settings dialog box](../../img/pages-layout-pages-page-numbers.png)

## Page background {#background}

[[menu.layout > menu.layout.page_background]] sets the color behind every
page; see [Document options](document-options.md#background).
