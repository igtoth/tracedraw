# Možnosti TraceDraw

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
nastavuje, jak program pracuje. Stránku zvolte ze seznamu vlevo.

## Obecné {#general}

![Stránka Obecné v možnostech TraceDraw](../../img/customize-options-app-general.png)

- [[options.on_startup]]: zobrazit úvodní obrazovku, začít nový dokument,
  nebo otevřít naposledy upravovaný dokument.
- [[options.show_new_document_dialog]] (zapnuto): ptát se na nastavení nové
  kresby; viz [Vytvoření kresby](../get-started/new-document.md).
- [[options.show_splash]] (zapnuto).
- [[options.hardware_acceleration]] (zapnuto): okno se kreslí grafickou kartou vybranou vedle ([[options.gpu_automatic]] na začátku); vypnuto, softwarovým vykreslováním počítače. Platí od dalšího spuštění. Pokud se okno takto neotevře, TraceDraw se spustí znovu se softwarovým vykreslováním, pak s OpenGL, a pamatuje si, co fungovalo.
- [[options.undo_levels]]: kolik kroků můžete vrátit zpět (výchozí 150).

## Zobrazení {#display}

![Stránka Zobrazení v možnostech TraceDraw](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: zobrazovat barvy tak, jak se vytisknou.
- [[options.show_tooltips]] (zapnuto), [[options.hide_bbox_curve_tools]] a
  [[options.node_tracking]] (vypnuto: po zapnutí může nástroj [[tool.pick]]
  táhnout uzly vybrané křivky).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (výchozí) nebo
  [[options.wheel_scroll]].
- [[options.ruler_calibration]]: přiložte k obrazovce pravítko a měňte
  [[options.screen_resolution]], dokud se značky neshodují, aby zvětšení
  100 % ukazovalo skutečné velikosti. [[options.reset]] vrátí hodnotu 96.

## Úpravy {#edit}

![Stránka Úpravy v možnostech TraceDraw](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: krok s klávesou <kbd>Ctrl</kbd> při otáčení a
  kreslení (výchozí 15 stupňů).
- [[options.drawing_precision]]: počet desetinných míst na liště vlastností a
  ve stavovém řádku (výchozí 3).
- [[options.focus_inactive]] (výchozí 20 %) a
  [[options.focus_double_click]]; viz [Režim soustředění](../objects/focus-mode.md).

## Uzly a úchyty {#nodes-and-handles}

![Stránka Uzly a úchyty v možnostech TraceDraw](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: malý, střední nebo velký.
- [[options.node_shape]] pro uzly typu [[options.node_cusp]],
  [[options.node_smooth]] a [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: [[options.main_color]] vybraných uzlů a úchytů a
  [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Ořezový rámeček {#clip-frame}

![Stránka Ořezový rámeček v možnostech TraceDraw](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (výchozí),
  [[options.auto_center_always]] nebo [[options.auto_center_never]].
- [[options.empty_frame_lines]] (zapnuto).

Viz [Umístění objektů do rámečku](../objects/clip-frames.md).

## Přichytávání {#snapping}

![Stránka Přichytávání v možnostech TraceDraw](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], mřížka účaří a
  [[options.snap_to_pixels]].
- [[options.snapping_radius]] (výchozí 10 pixelů).
- [[options.show_snap_marks]] a [[options.snap_screen_tip]].
- [[options.snap_modes]]: body, ke kterým se objekty přichytávají, s
  tlačítky [[options.select_all]] a [[options.deselect_all]].

Viz [Přichytávání](../pages-layout/snapping.md).

## Ukládání {#save}

![Stránka Ukládání v možnostech TraceDraw](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (zapnuto) uchová
  předchozí soubor jako `backup_of_<name>`, [[options.backup_to]]
  [[options.same_folder]] nebo zvolené místo.
- [[options.auto_backup]] (zapnuto): [[options.backup_every]] 20
  [[options.minutes]], do dočasné složky nebo na zvolené místo. Po pádu
  programu se zálohy nabídnou; viz
  [Automatické zálohování a obnova](../get-started/open-and-save.md#recovery).

## Text {#text}

![Stránka Text v možnostech TraceDraw](../../img/customize-options-app-text.png)

Stránka má pět karet.

- [[options.display]]: [[options.keyboard_text_increment]] (o kolik
  <kbd>Ctrl+8</kbd> a <kbd>Ctrl+2</kbd> zvětší a zmenší vybraný text, 1 bod),
  [[options.default_font]] a [[options.default_size]] nového textu a
  [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] a [[options.hyphenation_patterns]]
  (vestavěné, nebo slovník dělení slov pro váš jazyk).
- [[docker.fonts]]: [[options.show_recent_fonts]] nahoře v seznamech písem,
  [[options.recent_font_count]] (5, nejvýše 20) a
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  a [[options.add_to_autocorrect]], aby se slovo opravené v kontrole pravopisu
  příště opravilo při psaní; a [[options.dictionaries]], které používá, viz
  [Slovníky](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: co se opravuje při psaní; viz
  [Automatické opravy](../text/writing-tools.md#autocorrect).
