# Opções do documento

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

[[menu.layout > menu.layout.document_options]] abre as configurações do desenho
ativo: tamanho da página, layout, fundo, sangramento, réguas, grade e
linhas-guia. Clicar duas vezes em uma régua ou na borda da página também a abre.
Escolha uma página na lista à esquerda. As alterações aparecem na hora;
[[dialog.ok]] as mantém e [[dialog.cancel]] desfaz tudo. **Salvar como padrão**
guarda as configurações de grade, réguas e linhas-guia para novos desenhos.

## Geral {#general}

![A página Geral das opções do documento](../../img/pages-layout-document-options-general.png)

- [[options.rendering_resolution]]: a resolução dos efeitos que viram pixels,
  como sombras e efeitos de bitmap.
- [[options.fill_open_curves]] (desativado de início): ativado, as curvas
  abertas mostram o preenchimento como se fossem fechadas.
- [[options.auto_inflate_bitmaps]] (ativado de início): os bitmaps crescem para
  dar espaço a efeitos que passam das bordas.
- [[options.duplicate_offset]]: a que distância [[menu.edit.duplicate]] coloca a
  cópia.

## Tamanho da página {#page-size}

![A página Tamanho da página das opções do documento](../../img/pages-layout-document-options-page-size.png)

- Escolha [[options.page_size_radio]] para um tamanho de papel, ou
  [[options.label_presets]] para folhas de etiquetas, que definem a página com
  o tamanho da folha e acrescentam linhas-guia em volta de cada etiqueta.
- [[options.size]], [[dialog.width]] e [[dialog.height]], e
  [[options.portrait]] ou [[options.landscape]]. O botão de salvar ao lado de
  [[options.size]] guarda um tamanho personalizado com um nome que você
  digitar, e o botão de excluir esquece o tamanho salvo mostrado. Tamanhos
  salvos aparecem em todas as listas de tamanho de página: a barra de
  propriedades, a caixa de diálogo Tamanho da página, estas opções e a caixa
  de diálogo de novo documento.
- [[options.apply_current_only]]: muda só a página atual.
- [[options.show_page_border]], e [[options.add_page_frame]], que acrescenta um
  retângulo do tamanho da página.
- [[options.rendering_resolution]], [[options.bleed]] e
  [[options.show_bleed]] (veja abaixo).

## Layout {#layout}

![A página Layout das opções do documento](../../img/pages-layout-document-options-layout.png)

A página mostra quantas páginas o desenho tem, com [[menu.view.page_sorter]] e
[[menu.layout.switch_orientation]], e lista as [[options.master_layers]], cada
uma mostrada em [[options.scope_all]], [[options.scope_odd]] ou
[[options.scope_even]]. Os botões acrescentam uma nova camada mestra para
todas as páginas, ímpares ou pares. Veja
[Camadas mestras](../objects/objects-and-layers.md#master-layers).

## Fundo {#background}

![A página Fundo das opções do documento](../../img/pages-layout-document-options-background.png)

Escolha [[options.no_background]], uma cor [[options.solid]] ou um
[[options.bitmap]] ([[options.choose_bitmap]]) para mostrar atrás de todas as
páginas.

## Sangramento {#bleed}

![A página Sangramento das opções do documento](../../img/pages-layout-document-options-bleed.png)

O sangramento é a parte do desenho que passa da borda da página, para que a
impressão não tenha borda branca depois de refilada. Defina a distância do
[[options.bleed]] e marque [[options.show_bleed]] para vê-lo. A impressão e a
exportação em PDF podem incluir o sangramento; veja
[Impressão](../printing/print.md).

## Réguas {#rulers}

![A página Réguas das opções do documento](../../img/pages-layout-document-options-rulers.png)

- [[options.units]]: as unidades de desenho (polegadas, milímetros, paicas,
  pontos, pixels, cíceros, didots, pés, jardas, milhas, centímetros, metros ou
  quilômetros).
- [[options.drawing_scale]]: faz uma distância na página representar uma
  distância no mundo, como 1:100. Escolha uma escala comum, ou digite a
  distância na página e a distância no mundo. Réguas, posições, tamanhos e novas
  linhas de cota passam a mostrar distâncias do mundo; tamanhos de página,
  larguras de contorno e tamanhos de texto não.
- [[options.origin]]: de onde as réguas contam ([[options.horizontal_origin]] e
  [[options.vertical_origin]]); [[options.reset_origin]] a devolve. Você também
  pode arrastar a origem a partir do canto onde as réguas se encontram.
- [[options.tick_divisions]]: o número de marcas entre os números da régua.
- [[options.nudge_section]]: a distância de [[options.nudge_label]] das setas
  do teclado (2,54 mm), o [[options.super_nudge]] com <kbd>Shift</kbd> (5,08 mm)
  e o [[options.micro_nudge]] com <kbd>Ctrl</kbd> (0,254 mm).
- [[options.show_rulers]].

## Grade {#grid}

![A página Grade das opções do documento](../../img/pages-layout-document-options-grid.png)

- [[options.document_grid]]: [[options.show_grid]], [[options.snap_to_grid]], o
  espaçamento [[options.horizontal]] e [[options.vertical]] (10 mm de início),
  como distância ou como linhas por unidade, e [[options.show_grid_as]]
  [[options.grid_lines]] ou [[options.grid_dots]].
- [[options.baseline_grid]]: linhas pela página para alinhar texto, com o
  [[options.baseline_spacing]] (14 pontos), o [[options.baseline_start]] (12,7
  mm) e a [[options.line_color]].
- [[options.pixel_grid]]: a [[options.color]] e a [[options.opacity]] da grade
  de pixels, [[options.pixel_grid_800]] e [[options.snap_to_pixels]].

## Linhas-guia {#guidelines}

![A página Linhas-guia das opções do documento](../../img/pages-layout-document-options-guidelines.png)

- [[options.show_guidelines]] e [[options.snap_to_guidelines]].
- [[options.default_guide_color]] e [[options.default_preset_guide_color]].
- [[guides.presets]]: margens de uma polegada, área de sangramento, bordas da
  página, área imprimível, um boletim de três colunas, uma grade básica e uma
  grade no canto superior esquerdo.
- [[guides.user_defined]]: [[guides.margins]] ([[guides.top]],
  [[guides.bottom]], [[guides.left]], [[guides.right]]), [[guides.columns]]
  ([[guides.number_of_columns]] e [[guides.distance_between]]) e uma
  [[guides.grid]] ([[guides.rows]] e [[guides.columns_count]]).
  [[guides.apply_presets]] as acrescenta; uma linha que já existe não é
  acrescentada duas vezes.

Veja [Réguas, grades e linhas-guia](guidelines-and-grid.md) para trabalhar com
linhas-guia na página.
