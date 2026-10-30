# Opzioni globali

[[menu.tools > menu.tools.options > menu.tools.options_global]] contiene le
impostazioni che valgono per l'intero programma: lingua, valori predefiniti
di stampa ed esportazione, ricalco, messaggi e posizione dei file.

## Generale {#general}

![La pagina Generale delle opzioni globali](../../img/customize-global-options-general.png)

- [[options.language]]: la lingua dell'interfaccia. Questa Guida si apre nella
  stessa lingua.
- [[options.check_updates]] (attivo): vedi
  [Verificare gli aggiornamenti](../welcome/about-and-updates.md#updates).
- [[options.agents]]: [[options.mcp_window]] permette agli assistenti IA di
  questo computer di modificare il disegno aperto; vedi
  [Script](scripts.md#mcp).
- [[options.reset_all]]: vedi
  [Ripristinare le impostazioni](reset-settings.md).

## Stampa {#printing}

![La pagina Stampa delle opzioni globali](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]: [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (tutti disattivati), il [[print.sheet]],
[[print.landscape]] e il [[print.gutter]] (5 mm). [[options.print_remember]]
conserva le scelte dell'ultima stampa. Sotto [[options.print_checks]],
[[options.print_preflight]] (attivo). Vedi
[Stampare un disegno](../printing/print.md).

## Formati di file {#file-formats}

![La pagina Formati di file delle opzioni globali](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]: il [[dialog.format]] (SVG), la
  [[dialog.resolution]] dei formati bitmap (300 dpi), la [[dialog.quality]] di
  JPEG e WebP (90), [[dialog.transparent_background]] (attivo), lo
  [[dialog.pdf_standard]], la [[dialog.output_condition]] e la
  [[dialog.bleed]]. [[options.export_remember]] conserva le scelte
  dell'ultima esportazione.
- [[options.import_section]]: [[options.import_place]]
  [[options.import_as_in_file]] (in partenza) o
  [[options.import_view_center]].
- [[options.cdr_save_section]]: la [[dialog.cdr_version]] in cui i nuovi
  disegni vengono salvati come `.cdr`.

## Ricalco bitmap {#bitmap-tracing}

![La pagina Ricalco bitmap delle opzioni globali](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] uno dei preset (Clipart in partenza).
[[options.trace_starts_with]] [[dialog.remove_background]] (attivo) e
[[dialog.delete_original]] (disattivato). Vedi
[Ricalcare le bitmap](../bitmaps/tracing.md).

## Avvisi {#warnings}

![La pagina Avvisi delle opzioni globali](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]] e [[options.warn_shelf_remove]] (tutti
attivi). Ogni messaggio ha anche una casella che lo disattiva; spunta qui
l'opzione per vederlo di nuovo.

## Posizioni dei file {#file-locations}

![La pagina Posizioni dei file delle opzioni globali](../../img/customize-global-options-file-locations.png)

Mostra dove vengono conservati le preferenze e i backup automatici. Le
preferenze si trovano in una cartella `tracedraw-settings` accanto al
programma, se esiste (una copia portatile), altrimenti nella posizione del
sistema dedicata alle impostazioni dei programmi; la versione per browser le
conserva nel browser.
