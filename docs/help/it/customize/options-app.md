# Opzioni di TraceDraw

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
imposta il funzionamento del programma. Scegli una pagina dall'elenco a
sinistra.

## Generale {#general}

![La pagina Generale delle opzioni di TraceDraw](../../img/customize-options-app-general.png)

- [[options.on_startup]]: mostra la schermata di benvenuto, avvia un nuovo
  documento o apri l'ultimo documento modificato.
- [[options.show_new_document_dialog]] (attivo): chiede le impostazioni di un
  nuovo disegno; vedi [Creare un disegno](../get-started/new-document.md).
- [[options.show_splash]] (attivo).
- [[options.hardware_acceleration]] (attivo): disegna la finestra con la scheda grafica scelta accanto ([[options.gpu_automatic]] all'inizio); se disattivato, con il renderer software del computer. Vale dal prossimo avvio. Se la finestra non si apre così, TraceDraw si riavvia con il renderer software, poi con OpenGL, e tiene quello che ha funzionato.
- [[options.undo_levels]]: quanti passaggi puoi annullare (150 in partenza).

## Visualizzazione {#display}

![La pagina Visualizzazione delle opzioni di TraceDraw](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: mostra i colori come verranno stampati.
- [[options.show_tooltips]] (attivo), [[options.hide_bbox_curve_tools]] e
  [[options.node_tracking]] (disattivato: se attivo, lo strumento
  [[tool.pick]] può trascinare i nodi di una curva selezionata).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (in partenza) o
  [[options.wheel_scroll]].
- [[options.open_view]]: [[options.open_view_page]] (in partenza),
  [[options.open_view_objects]], [[options.open_view_width]],
  [[options.open_view_height]], [[options.open_view_actual]] o
  [[options.open_view_saved]] (la pagina, il punto e lo zoom tenuti a ogni
  salvataggio di un file TraceDraw).
- [[options.ruler_calibration]]: tieni un righello contro lo schermo e cambia
  la [[options.screen_resolution]] finché le tacche coincidono, così lo zoom
  al 100% mostra le dimensioni reali. [[options.reset]] rimette 96.

## Modifica {#edit}

![La pagina Modifica delle opzioni di TraceDraw](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: il passo di <kbd>Ctrl</kbd> quando ruoti e
  disegni (15 gradi in partenza).
- [[options.drawing_precision]]: le cifre decimali nella barra delle
  proprietà e nella barra di stato (3 in partenza).
- [[options.focus_inactive]] (20% in partenza) e
  [[options.focus_double_click]]; vedi
  [Modalità focus](../objects/focus-mode.md).

## Nodi e maniglie {#nodes-and-handles}

![La pagina Nodi e maniglie delle opzioni di TraceDraw](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: piccola, media o grande.
- [[options.node_shape]] per i nodi [[options.node_cusp]],
  [[options.node_smooth]] e [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: il [[options.main_color]] dei nodi e delle maniglie
  selezionati e il [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Cornice di ritaglio {#clip-frame}

![La pagina Cornice di ritaglio delle opzioni di TraceDraw](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (in partenza),
  [[options.auto_center_always]] o [[options.auto_center_never]].
- [[options.empty_frame_lines]] (attivo).

Vedi [Inserire oggetti in una cornice](../objects/clip-frames.md).

## Aggancio {#snapping}

![La pagina Aggancio delle opzioni di TraceDraw](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], la griglia della
  linea di base e [[options.snap_to_pixels]].
- [[options.snapping_radius]] (10 pixel in partenza).
- [[options.show_snap_marks]] e il [[options.snap_screen_tip]].
- [[options.snap_modes]]: i punti a cui gli oggetti si agganciano, con
  [[options.select_all]] e [[options.deselect_all]].

Vedi [Aggancio](../pages-layout/snapping.md).

## Salvataggio {#save}

![La pagina Salvataggio delle opzioni di TraceDraw](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (attivo) conserva il file
  precedente come `backup_of_<name>`, [[options.backup_to]] la
  [[options.same_folder]] o una posizione scelta.
- [[options.auto_backup]] (attivo): [[options.backup_every]] 20
  [[options.minutes]], in una cartella temporanea o in una posizione scelta.
  Dopo un arresto anomalo vengono proposti i backup; vedi
  [Backup automatico e recupero](../get-started/open-and-save.md#recovery).

## Testo {#text}

![La pagina Testo delle opzioni di TraceDraw](../../img/customize-options-app-text.png)

La pagina ha cinque schede.

- [[options.display]]: [[options.keyboard_text_increment]] (di quanto
  <kbd>Ctrl+8</kbd> e <kbd>Ctrl+2</kbd> ingrandiscono e riducono il testo
  selezionato, 1 punto), [[options.default_font]] e [[options.default_size]] del
  nuovo testo, e [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] e gli [[options.hyphenation_patterns]]
  (integrati, o un dizionario di sillabazione per la tua lingua).
- [[docker.fonts]]: [[options.show_recent_fonts]] in cima agli elenchi dei
  font, il [[options.recent_font_count]] (5, fino a 20) e la
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  e [[options.add_to_autocorrect]], così una parola corretta nel controllo
  ortografico viene corretta durante la digitazione la volta successiva; e i
  [[options.dictionaries]] che usa, vedere
  [Dizionari](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: cosa viene corretto durante la digitazione; vedere
  [Correzione automatica](../text/writing-tools.md#autocorrect).
