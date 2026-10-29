# Opcje TraceDraw

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
określa sposób działania programu. Wybierz stronę z listy po lewej stronie.

## Ogólne {#general}

![Strona Ogólne opcji TraceDraw](../../img/customize-options-app-general.png)

- [[options.on_startup]]: wyświetl ekran powitalny, utwórz nowy dokument lub
  otwórz ostatnio edytowany dokument.
- [[options.show_new_document_dialog]] (włączone): pytaj o ustawienia nowego
  rysunku; zobacz [Tworzenie rysunku](../get-started/new-document.md).
- [[options.show_splash]] (włączone).
- [[options.hardware_acceleration]] (włączone): okno jest rysowane kartą graficzną wybraną obok (na początku [[options.gpu_automatic]]); wyłączone, programowym rendererem komputera. Działa od następnego uruchomienia. Jeśli okno nie może się tak otworzyć, TraceDraw uruchamia się ponownie z rendererem programowym, potem z OpenGL, i zapamiętuje ten, który zadziałał.
- [[options.undo_levels]]: ile kroków można cofnąć (domyślnie 150).

## Wyświetlanie {#display}

![Strona Wyświetlanie opcji TraceDraw](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: pokazuj kolory tak, jak zostaną wydrukowane.
- [[options.show_tooltips]] (włączone), [[options.hide_bbox_curve_tools]]
  i [[options.node_tracking]] (wyłączone: po włączeniu narzędzie [[tool.pick]]
  może przeciągać węzły zaznaczonej krzywej).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (domyślnie) lub
  [[options.wheel_scroll]].
- [[options.open_view]]: [[options.open_view_page]] (domyślnie),
  [[options.open_view_objects]], [[options.open_view_width]],
  [[options.open_view_height]], [[options.open_view_actual]] lub
  [[options.open_view_saved]] (strona, miejsce i powiększenie zapamiętane
  przy każdym zapisie pliku TraceDraw).
- [[options.ruler_calibration]]: przyłóż linijkę do ekranu i zmieniaj wartość
  [[options.screen_resolution]], aż podziałki się pokryją, aby powiększenie 100%
  pokazywało rzeczywiste rozmiary. [[options.reset]] przywraca 96.

## Edycja {#edit}

![Strona Edycja opcji TraceDraw](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: krok kąta z <kbd>Ctrl</kbd> podczas obracania
  i rysowania (domyślnie 15 stopni).
- [[options.drawing_precision]]: liczba miejsc po przecinku na pasku
  właściwości i pasku stanu (domyślnie 3).
- [[options.focus_inactive]] (domyślnie 20%) i [[options.focus_double_click]];
  zobacz [Tryb skupienia](../objects/focus-mode.md).

## Węzły i uchwyty {#nodes-and-handles}

![Strona Węzły i uchwyty opcji TraceDraw](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: mały, średni lub duży.
- [[options.node_shape]] dla węzłów [[options.node_cusp]],
  [[options.node_smooth]] i [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: [[options.main_color]] zaznaczonych węzłów i uchwytów
  oraz [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Ramka przycinania {#clip-frame}

![Strona Ramka przycinania opcji TraceDraw](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (domyślnie),
  [[options.auto_center_always]] lub [[options.auto_center_never]].
- [[options.empty_frame_lines]] (włączone).

Zobacz [Umieszczanie obiektów w ramce](../objects/clip-frames.md).

## Przyciąganie {#snapping}

![Strona Przyciąganie opcji TraceDraw](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], siatka linii
  bazowych i [[options.snap_to_pixels]].
- [[options.snapping_radius]] (domyślnie 10 pikseli).
- [[options.show_snap_marks]] i [[options.snap_screen_tip]].
- [[options.snap_modes]]: punkty, do których przyciągają obiekty, z przyciskami
  [[options.select_all]] i [[options.deselect_all]].

Zobacz [Przyciąganie](../pages-layout/snapping.md).

## Zapisywanie {#save}

![Strona Zapisywanie opcji TraceDraw](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (włączone) zachowuje
  poprzedni plik jako `backup_of_<name>`, [[options.backup_to]]
  [[options.same_folder]] lub w wybranym miejscu.
- [[options.auto_backup]] (włączone): [[options.backup_every]] 20
  [[options.minutes]], do folderu tymczasowego lub w wybranym miejscu. Po awarii
  program proponuje kopie zapasowe; zobacz
  [Automatyczna kopia zapasowa i odzyskiwanie](../get-started/open-and-save.md#recovery).

## Tekst {#text}

![Strona Tekst opcji TraceDraw](../../img/customize-options-app-text.png)

Strona ma pięć kart.

- [[options.display]]: [[options.keyboard_text_increment]] (o ile
  <kbd>Ctrl+8</kbd> i <kbd>Ctrl+2</kbd> powiększają i zmniejszają zaznaczony
  tekst, 1 punkt), [[options.default_font]] i [[options.default_size]] nowego
  tekstu oraz [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] i [[options.hyphenation_patterns]]
  (wbudowane lub słownik dzielenia wyrazów dla Twojego języka).
- [[docker.fonts]]: [[options.show_recent_fonts]] na górze list czcionek,
  [[options.recent_font_count]] (5, do 20) i
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  i [[options.add_to_autocorrect]], aby słowo poprawione w sprawdzaniu pisowni
  następnym razem poprawiało się podczas pisania; oraz
  [[options.dictionaries]], których używa, zobacz
  [Słowniki](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: co jest poprawiane podczas pisania; zobacz
  [Autokorekta](../text/writing-tools.md#autocorrect).
