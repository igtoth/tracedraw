# Globální možnosti

> *Strojový překlad: tato stránka může zaostávat za anglickou verzí.*

[[menu.tools > menu.tools.options > menu.tools.options_global]] obsahuje
nastavení platná pro celý program: jazyk, výchozí hodnoty tisku a exportu,
vektorizaci, zprávy a umístění souborů.

## Obecné {#general}

![Stránka Obecné v globálních možnostech](../../img/customize-global-options-general.png)

- [[options.language]]: jazyk rozhraní. Tato nápověda se otevírá ve stejném
  jazyce.
- [[options.check_updates]] (zapnuto): viz
  [Hledání aktualizací](../welcome/about-and-updates.md#updates).
- [[options.agents]]: [[options.mcp_window]] umožní asistentům AI v tomto
  počítači upravovat otevřenou kresbu; viz [Skripty](scripts.md#mcp).
- [[options.reset_all]]: viz [Obnovení nastavení](reset-settings.md).

## Tisk {#printing}

![Stránka Tisk v globálních možnostech](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]: [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (vše vypnuto), [[print.sheet]], [[print.landscape]]
a [[print.gutter]] (5 mm). [[options.print_remember]] zachová volby
posledního tisku. V části [[options.print_checks]]
[[options.print_preflight]] (zapnuto). Viz [Tisk kresby](../printing/print.md).

## Formáty souborů {#file-formats}

![Stránka Formáty souborů v globálních možnostech](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]: [[dialog.format]] (SVG),
  [[dialog.resolution]] bitmapových formátů (300 dpi), [[dialog.quality]]
  JPEG a WebP (90), [[dialog.transparent_background]] (zapnuto),
  [[dialog.pdf_standard]], [[dialog.output_condition]] a [[dialog.bleed]].
  [[options.export_remember]] zachová volby posledního exportu.
- [[options.import_section]]: [[options.import_place]]
  [[options.import_as_in_file]] (výchozí) nebo [[options.import_view_center]].
- [[options.cdr_save_section]]: [[dialog.cdr_version]], ve které se nové
  kresby ukládají jako `.cdr`.

## Vektorizace bitmap {#bitmap-tracing}

![Stránka Vektorizace bitmap v globálních možnostech](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] jednu z předvoleb (výchozí Clipart).
[[options.trace_starts_with]] [[dialog.remove_background]] (zapnuto) a
[[dialog.delete_original]] (vypnuto). Viz [Vektorizace bitmap](../bitmaps/tracing.md).

## Upozornění {#warnings}

![Stránka Upozornění v globálních možnostech](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]] a [[options.warn_shelf_remove]] (vše zapnuto).
Každá zpráva má také políčko, které ji vypne; zaškrtnutím volby zde ji
uvidíte znovu.

[[options.ask_missing_fonts]] (zapnuto) zobrazí dialog Chybějící písma, když
se otevře výkres s písmy, která nejsou nainstalována; viz
[Chybějící písma](../text/missing-fonts.md). Je to samostatná volba, nezávislá
na prosté zprávě výše.

## Umístění souborů {#file-locations}

![Stránka Umístění souborů v globálních možnostech](../../img/customize-global-options-file-locations.png)

Ukazuje, kde jsou uloženy předvolby a automatické zálohy. Předvolby jsou ve
složce `tracedraw-settings` vedle programu, pokud existuje (přenosná kopie),
jinak v místě, které váš systém určuje pro nastavení programů; verze pro
prohlížeč je uchovává v prohlížeči.
