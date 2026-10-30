# Opciones globales

> *Traducción automática: esta página puede ir por detrás de la versión en inglés.*

[[menu.tools > menu.tools.options > menu.tools.options_global]] contiene los
ajustes que se aplican a todo el programa: idioma, valores predeterminados de
impresión y exportación, calco, mensajes y dónde se guardan los archivos.

## General {#general}

![La página General de las opciones globales](../../img/customize-global-options-general.png)

- [[options.language]]: el idioma de la interfaz. Esta Ayuda se abre en el
  mismo idioma.
- [[options.check_updates]] (activado): vea
  [Buscar actualizaciones](../welcome/about-and-updates.md#updates).
- [[options.agents]]: [[options.mcp_window]] permite que los asistentes de IA
  de este equipo editen el dibujo abierto; vea [Scripts](scripts.md#mcp).
- [[options.reset_all]]: vea [Restablecer los ajustes](reset-settings.md).

## Impresión {#printing}

![La página Impresión de las opciones globales](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]: [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (todos desactivados), la [[print.sheet]],
[[print.landscape]] y el [[print.gutter]] (5 mm).
[[options.print_remember]] conserva las opciones de la última impresión. En
[[options.print_checks]], [[options.print_preflight]] (activado). Vea
[Imprimir un dibujo](../printing/print.md).

## Formatos de archivo {#file-formats}

![La página Formatos de archivo de las opciones globales](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]: el [[dialog.format]] (SVG), la
  [[dialog.resolution]] de los formatos de mapa de bits (300 ppp), la
  [[dialog.quality]] de JPEG y WebP (90), [[dialog.transparent_background]]
  (activado), el [[dialog.pdf_standard]], la [[dialog.output_condition]] y el
  [[dialog.bleed]]. [[options.export_remember]] conserva las opciones de la
  última exportación.
- [[options.import_section]]: [[options.import_place]]
  [[options.import_as_in_file]] (opción predeterminada) o
  [[options.import_view_center]].
- [[options.cdr_save_section]]: la [[dialog.cdr_version]] en la que se
  guardan como `.cdr` los dibujos nuevos.

## Calco de mapas de bits {#bitmap-tracing}

![La página Calco de mapas de bits de las opciones globales](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] uno de los ajustes preestablecidos (Imágenes
prediseñadas de forma predeterminada). [[options.trace_starts_with]]
[[dialog.remove_background]] (activado) y [[dialog.delete_original]]
(desactivado). Vea [Calcar mapas de bits](../bitmaps/tracing.md).

## Avisos {#warnings}

![La página Avisos de las opciones globales](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]] y [[options.warn_shelf_remove]] (todos
activados). Cada mensaje tiene además una casilla que lo desactiva; marque
aquí la opción para volver a verlo.

[[options.ask_missing_fonts]] (activada) muestra el cuadro de diálogo Fuentes
que faltan cuando un dibujo se abre con fuentes que no están instaladas;
consulte [Fuentes que faltan](../text/missing-fonts.md). Es independiente del
mensaje simple de arriba.

## Ubicaciones de archivos {#file-locations}

![La página Ubicaciones de archivos de las opciones globales](../../img/customize-global-options-file-locations.png)

Muestra dónde se guardan las preferencias y las copias de seguridad
automáticas. Las preferencias están en una carpeta `tracedraw-settings` junto
al programa cuando existe (una copia portátil); si no, en el lugar que su
sistema destina a los ajustes de los programas. La versión para navegador las
guarda en el navegador.
