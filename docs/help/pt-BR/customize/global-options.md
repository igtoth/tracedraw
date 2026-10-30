# Opções globais

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

[[menu.tools > menu.tools.options > menu.tools.options_global]] reúne as
configurações que valem para o programa inteiro: idioma, padrões de impressão e
exportação, vetorização, mensagens e onde os arquivos ficam.

## Geral {#general}

![A página Geral das opções globais](../../img/customize-global-options-general.png)

- [[options.language]]: o idioma da interface. Esta Ajuda abre no mesmo idioma.
- [[options.check_updates]] (ligado): veja
  [Procurar atualizações](../welcome/about-and-updates.md#updates).
- [[options.agents]]: [[options.mcp_window]] deixa assistentes de IA neste
  computador editarem o desenho aberto; veja [Scripts](scripts.md#mcp).
- [[options.reset_all]]: veja [Redefinir configurações](reset-settings.md).

## Impressão {#printing}

![A página Impressão das opções globais](../../img/customize-global-options-printing.png)

[[options.print_starts_with]]: [[dialog.fit_to_page]],
[[dialog.separations]], [[dialog.print_bleed]], [[dialog.crop_marks]],
[[dialog.registration_marks]], [[print.color_bars]], [[print.densitometer]],
[[print.page_info_mark]] (todos desligados), a [[print.sheet]],
[[print.landscape]] e o [[print.gutter]] (5 mm). [[options.print_remember]]
guarda as escolhas da última impressão. Em [[options.print_checks]],
[[options.print_preflight]] (ligado). Veja
[Imprimir um desenho](../printing/print.md).

## Formatos de arquivo {#file-formats}

![A página Formatos de arquivo das opções globais](../../img/customize-global-options-file-formats.png)

- [[options.export_starts_with]]: o [[dialog.format]] (SVG), a
  [[dialog.resolution]] dos formatos de bitmap (300 dpi), a
  [[dialog.quality]] de JPEG e WebP (90), [[dialog.transparent_background]]
  (ligado), o [[dialog.pdf_standard]], a [[dialog.output_condition]] e a
  [[dialog.bleed]]. [[options.export_remember]] guarda as escolhas da última
  exportação.
- [[options.import_section]]: [[options.import_place]]
  [[options.import_as_in_file]] (de início) ou [[options.import_view_center]].
- [[options.cdr_save_section]]: a [[dialog.cdr_version]] em que os desenhos
  novos são salvos como `.cdr`.

## Vetorização de bitmaps {#bitmap-tracing}

![A página Vetorização de bitmaps das opções globais](../../img/customize-global-options-bitmap-tracing.png)

[[options.quick_trace_with]] uma das predefinições (Clipart de início).
[[options.trace_starts_with]] [[dialog.remove_background]] (ligado) e
[[dialog.delete_original]] (desligado). Veja
[Vetorizar bitmaps](../bitmaps/tracing.md).

## Avisos {#warnings}

![A página Avisos das opções globais](../../img/customize-global-options-warnings.png)

[[options.warnings_show]] [[options.warn_unread_objects]],
[[options.warn_missing_fonts]] e [[options.warn_shelf_remove]] (todos ligados).
Cada mensagem também tem uma caixa que a desativa; marque a opção aqui para
voltar a vê-la.

[[options.ask_missing_fonts]] (ligado) mostra a caixa de diálogo Fontes
ausentes quando um desenho abre com fontes que não estão instaladas; veja
[Fontes ausentes](../text/missing-fonts.md). É separado da mensagem simples
acima.

## Locais de arquivos {#file-locations}

![A página Locais de arquivos das opções globais](../../img/customize-global-options-file-locations.png)

Mostra onde ficam as preferências e as cópias de segurança automáticas. As
preferências ficam em uma pasta `tracedraw-settings` ao lado do programa quando
ela existe (uma cópia portátil), senão no lugar do sistema para configurações de
programas; a versão para navegador as guarda no navegador.
