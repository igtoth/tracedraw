# Opcje globalne

> *Tłumaczenie maszynowe: ta strona może być mniej aktualna niż wersja angielska.*

[[menu.tools > menu.tools.options > menu.tools.options_global]] zawiera
ustawienia dotyczące całego programu: język, ustawienia domyślne drukowania
i eksportu, wektoryzację, komunikaty oraz miejsca przechowywania plików.

## Ogólne {#general}

![Strona Ogólne opcji globalnych](../../img/customize-global-options-general.png)

- [[options.language]]: język interfejsu. Ta Pomoc otwiera się w tym samym
  języku.
- [[options.check_updates]] (włączone): zobacz
  [Sprawdzanie aktualizacji](../welcome/about-and-updates.md#updates).
- [[options.agents]]: [[options.mcp_window]] pozwala asystentom AI na tym
  komputerze edytować otwarty rysunek; zobacz [Skrypty](scripts.md#mcp).
- [[options.reset_all]]: zobacz [Resetowanie ustawień](reset-settings.md).

## Drukowanie {#printing}

![Strona Drukowanie opcji globalnych](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]: [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (wszystkie wyłączone), [[print.sheet]],
[[print.landscape]] oraz [[print.gutter]] (5 mm). [[options.print_remember]]
zachowuje wybory z ostatniego drukowania. W sekcji [[options.print_checks]]:
[[options.print_preflight]] (włączone). Zobacz
[Drukowanie rysunku](../printing/print.md).

## Formaty plików {#file-formats}

![Strona Formaty plików opcji globalnych](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]: [[dialog.format]] (SVG),
  [[dialog.resolution]] formatów bitmapowych (300 dpi), [[dialog.quality]] JPEG
  i WebP (90), [[dialog.transparent_background]] (włączone),
  [[dialog.pdf_standard]], [[dialog.output_condition]] oraz [[dialog.bleed]].
  [[options.export_remember]] zachowuje wybory z ostatniego eksportu.
- [[options.import_section]]: [[options.import_place]]
  [[options.import_as_in_file]] (domyślnie) lub [[options.import_view_center]].
- [[options.cdr_save_section]]: [[dialog.cdr_version]], w której nowe rysunki
  są zapisywane jako `.cdr`.

## Wektoryzacja bitmap {#bitmap-tracing}

![Strona Wektoryzacja bitmap opcji globalnych](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] jednego z ustawień wstępnych (domyślnie Clipart).
[[options.trace_starts_with]]: [[dialog.remove_background]] (włączone)
i [[dialog.delete_original]] (wyłączone). Zobacz
[Wektoryzacja bitmap](../bitmaps/tracing.md).

## Ostrzeżenia {#warnings}

![Strona Ostrzeżenia opcji globalnych](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]] i [[options.warn_shelf_remove]] (wszystkie
włączone). Każdy komunikat ma też pole, które go wyłącza; zaznacz tutaj opcję,
aby znów go widzieć.

[[options.ask_missing_fonts]] (włączone) pokazuje okno Brakujące czcionki,
gdy otwiera się rysunek z czcionkami, które nie są zainstalowane; zobacz
[Brakujące czcionki](../text/missing-fonts.md). Jest to opcja niezależna od
zwykłego komunikatu powyżej.

## Lokalizacje plików {#file-locations}

![Strona Lokalizacje plików opcji globalnych](../../img/customize-global-options-file-locations.png)

Pokazuje, gdzie są przechowywane preferencje i automatyczne kopie zapasowe.
Preferencje znajdują się w folderze `tracedraw-settings` obok programu, jeśli
taki istnieje (kopia przenośna), a w przeciwnym razie w miejscu, którego system
używa na ustawienia programów; wersja przeglądarkowa przechowuje je
w przeglądarce.
