# Общие параметры

> *Машинный перевод: эта страница может отставать от английской версии.*

[[menu.tools > menu.tools.options > menu.tools.options_global]] содержит
параметры, которые действуют на всю программу: язык, значения по умолчанию
для печати и экспорта, трассировку, сообщения и места хранения файлов.

## Общие {#general}

![Страница «Общие» общих параметров](../../img/customize-global-options-general.png)

- [[options.language]]: язык интерфейса. Эта справка открывается на том же
  языке.
- [[options.check_updates]] (включено): см.
  [Проверка обновлений](../welcome/about-and-updates.md#updates).
- [[options.agents]]: [[options.mcp_window]] позволяет ИИ-ассистентам на этом
  компьютере редактировать открытый рисунок; см. [Сценарии](scripts.md#mcp).
- [[options.reset_all]]: см. [Сброс настроек](reset-settings.md).

## Печать {#printing}

![Страница «Печать» общих параметров](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]: [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (все выключены), [[print.sheet]], [[print.landscape]]
и [[print.gutter]] (5 мм). [[options.print_remember]] сохраняет выбор
последней печати. В разделе [[options.print_checks]]
[[options.print_preflight]] (включено). См. [Печать рисунка](../printing/print.md).

## Форматы файлов {#file-formats}

![Страница «Форматы файлов» общих параметров](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]: [[dialog.format]] (SVG),
  [[dialog.resolution]] растровых форматов (300 dpi), [[dialog.quality]] JPEG
  и WebP (90), [[dialog.transparent_background]] (включено),
  [[dialog.pdf_standard]], [[dialog.output_condition]] и [[dialog.bleed]].
  [[options.export_remember]] сохраняет выбор последнего экспорта.
- [[options.import_section]]: [[options.import_place]]
  [[options.import_as_in_file]] (по умолчанию) или
  [[options.import_view_center]].
- [[options.cdr_save_section]]: [[dialog.cdr_version]], в которой новые
  рисунки сохраняются как `.cdr`.

## Трассировка растровых изображений {#bitmap-tracing}

![Страница «Трассировка растровых изображений» общих параметров](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] один из наборов параметров (по умолчанию
«Клипарт»). [[options.trace_starts_with]] [[dialog.remove_background]]
(включено) и [[dialog.delete_original]] (выключено). См.
[Трассировка растровых изображений](../bitmaps/tracing.md).

## Предупреждения {#warnings}

![Страница «Предупреждения» общих параметров](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]] и [[options.warn_shelf_remove]] (все
включены). В каждом сообщении также есть флажок, который отключает его;
отметьте параметр здесь, чтобы снова его видеть.

[[options.ask_missing_fonts]] (включено) показывает диалог «Отсутствующие
шрифты», когда открывается рисунок со шрифтами, которые не установлены; см.
[Отсутствующие шрифты](../text/missing-fonts.md). Этот параметр не зависит от
простого сообщения выше.

## Расположение файлов {#file-locations}

![Страница «Расположение файлов» общих параметров](../../img/customize-global-options-file-locations.png)

Показывает, где хранятся настройки и автоматические резервные копии.
Настройки находятся в папке `tracedraw-settings` рядом с программой, если она
есть (переносная копия), иначе в системном месте для настроек программ; версия
для браузера хранит их в браузере.
