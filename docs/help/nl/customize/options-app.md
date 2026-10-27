# TraceDraw-opties

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
stelt in hoe het programma werkt. Kies een pagina in de lijst links.

## Algemeen {#general}

![De pagina Algemeen van de TraceDraw-opties](../../img/customize-options-app-general.png)

- [[options.on_startup]]: het welkomstscherm tonen, een nieuw document
  beginnen, of het laatst bewerkte document openen.
- [[options.show_new_document_dialog]] (aan): om de instellingen van een
  nieuwe tekening vragen; zie [Een tekening maken](../get-started/new-document.md).
- [[options.show_splash]] (aan).
- [[options.hardware_acceleration]] (aan): tekent het venster met de grafische kaart die ernaast is gekozen ([[options.gpu_automatic]] in het begin); uit, met de softwarerenderer van de computer. Het geldt vanaf de volgende start. Als het venster zo niet opent, start TraceDraw opnieuw met de softwarerenderer, daarna met OpenGL, en onthoudt wat werkte.
- [[options.undo_levels]]: hoeveel stappen u ongedaan kunt maken (in het begin
  150).

## Weergave {#display}

![De pagina Weergave van de TraceDraw-opties](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: kleuren tonen zoals ze worden afgedrukt.
- [[options.show_tooltips]] (aan), [[options.hide_bbox_curve_tools]] en
  [[options.node_tracking]] (uit: als dit aan staat, kan het gereedschap
  [[tool.pick]] de knooppunten van een geselecteerde curve slepen).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (in het begin) of
  [[options.wheel_scroll]].
- [[options.ruler_calibration]]: houd een liniaal tegen het scherm en wijzig de
  [[options.screen_resolution]] totdat de streepjes overeenkomen, zodat zoom
  100% echte maten toont. [[options.reset]] zet 96 terug.

## Bewerken {#edit}

![De pagina Bewerken van de TraceDraw-opties](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: de stap van <kbd>Ctrl</kbd> bij draaien en
  tekenen (in het begin 15 graden).
- [[options.drawing_precision]]: het aantal decimalen op de eigenschappenbalk
  en de statusbalk (in het begin 3).
- [[options.focus_inactive]] (in het begin 20%) en
  [[options.focus_double_click]]; zie [Focusmodus](../objects/focus-mode.md).

## Knooppunten en grepen {#nodes-and-handles}

![De pagina Knooppunten en grepen van de TraceDraw-opties](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: klein, middel of groot.
- [[options.node_shape]] voor knooppunten van het type [[options.node_cusp]],
  [[options.node_smooth]] en [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: de [[options.main_color]] van geselecteerde knooppunten
  en grepen en de [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Uitsnijkader {#clip-frame}

![De pagina Uitsnijkader van de TraceDraw-opties](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (in het begin),
  [[options.auto_center_always]] of [[options.auto_center_never]].
- [[options.empty_frame_lines]] (aan).

Zie [Objecten in een uitsnijkader plaatsen](../objects/clip-frames.md).

## Kleven {#snapping}

![De pagina Kleven van de TraceDraw-opties](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], het
  basislijnraster en [[options.snap_to_pixels]].
- [[options.snapping_radius]] (in het begin 10 pixels).
- [[options.show_snap_marks]] en de [[options.snap_screen_tip]].
- [[options.snap_modes]]: de punten waaraan objecten kleven, met
  [[options.select_all]] en [[options.deselect_all]].

Zie [Kleven](../pages-layout/snapping.md).

## Opslaan {#save}

![De pagina Opslaan van de TraceDraw-opties](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (aan) bewaart het vorige
  bestand als `backup_of_<name>`, [[options.backup_to]] de
  [[options.same_folder]] of een gekozen plaats.
- [[options.auto_backup]] (aan): [[options.backup_every]] 20
  [[options.minutes]], naar een tijdelijke map of een gekozen plaats. Na een
  crash worden de back-ups aangeboden; zie
  [Automatische back-up en herstel](../get-started/open-and-save.md#recovery).

## Tekst {#text}

![De pagina Tekst van de TraceDraw-opties](../../img/customize-options-app-text.png)

De pagina heeft vijf tabbladen.

- [[options.display]]: [[options.keyboard_text_increment]] (hoeveel
  <kbd>Ctrl+8</kbd> en <kbd>Ctrl+2</kbd> de geselecteerde tekst groter en kleiner
  maken, 1 punt), [[options.default_font]] en [[options.default_size]] van
  nieuwe tekst, en [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] en de [[options.hyphenation_patterns]]
  (ingebouwd, of een afbreekwoordenboek voor uw taal).
- [[docker.fonts]]: [[options.show_recent_fonts]] bovenaan de
  lettertypelijsten, het [[options.recent_font_count]] (5, tot 20) en de
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  en [[options.add_to_autocorrect]], zodat een woord dat u in de
  spellingcontrole verbetert de volgende keer tijdens het typen wordt verbeterd;
  en de [[options.dictionaries]] die ze gebruikt, zie
  [Woordenboeken](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: wat tijdens het typen wordt verbeterd; zie
  [AutoCorrectie](../text/writing-tools.md#autocorrect).
