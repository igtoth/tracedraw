# Opções do TraceDraw

[[menu.tools > menu.tools.options > menu.tools.options_app]] (<kbd>Ctrl+J</kbd>)
define como o programa funciona. Escolha uma página na lista à esquerda.

## Geral {#general}

![A página Geral das opções do TraceDraw](../../img/customize-options-app-general.png)

- [[options.on_startup]]: mostrar a Tela de boas-vindas, começar um documento
  novo ou abrir o último documento editado.
- [[options.show_new_document_dialog]] (ligado): pedir as configurações de um
  desenho novo; veja [Criar um desenho](../get-started/new-document.md).
- [[options.show_splash]] (ligado).
- [[options.hardware_acceleration]] (ligado): desenha a janela com a placa de vídeo escolhida ao lado ([[options.gpu_automatic]] de início); desligado, com o renderizador por software do computador. Vale a partir da próxima abertura. Se a janela não abrir assim, o TraceDraw abre de novo com o renderizador por software, depois com OpenGL, e guarda o que funcionou.
- [[options.undo_levels]]: quantos passos você pode desfazer (150 de início).

## Exibição {#display}

![A página Exibição das opções do TraceDraw](../../img/customize-options-app-display.png)

- [[menu.view.proof_colors]]: mostra as cores como serão impressas.
- [[options.show_tooltips]] (ligado), [[options.hide_bbox_curve_tools]] e
  [[options.node_tracking]] (desligado: quando ligado, a ferramenta
  [[tool.pick]] pode arrastar os nós de uma curva selecionada).
- [[options.full_screen_preview]]: [[options.show_page_border]].
- [[options.wheel_default_action]]: [[options.wheel_zoom]] (de início) ou
  [[options.wheel_scroll]].
- [[options.open_view]]: [[options.open_view_page]] (de início),
  [[options.open_view_objects]], [[options.open_view_width]],
  [[options.open_view_height]], [[options.open_view_actual]] ou
  [[options.open_view_saved]] (a página, o lugar e o zoom guardados a cada
  vez que um arquivo do TraceDraw é salvo).
- [[options.ruler_calibration]]: segure uma régua contra a tela e mude a
  [[options.screen_resolution]] até as marcas coincidirem, para que o zoom de
  100% mostre tamanhos reais. [[options.reset]] devolve 96.

## Edição {#edit}

![A página Edição das opções do TraceDraw](../../img/customize-options-app-edit.png)

- [[options.constrain_angle]]: o passo do <kbd>Ctrl</kbd> ao girar e desenhar
  (15 graus de início).
- [[options.drawing_precision]]: as casas decimais na barra de propriedades e
  na barra de status (3 de início).
- [[options.focus_inactive]] (20% de início) e
  [[options.focus_double_click]]; veja [Modo de foco](../objects/focus-mode.md).

## Nós e alças {#nodes-and-handles}

![A página Nós e alças das opções do TraceDraw](../../img/customize-options-app-nodes-and-handles.png)

- [[options.node_size]]: pequeno, médio ou grande.
- [[options.node_shape]] para nós [[options.node_cusp]],
  [[options.node_smooth]] e [[options.node_symmetrical]].
- [[options.show_curve_direction]].
- [[options.corner_widgets]].
- [[options.colors]]: a [[options.main_color]] de nós e alças selecionados e a
  [[options.secondary_color]].
- [[options.unselected_nodes_filled]].

## Quadro de recorte {#clip-frame}

![A página Quadro de recorte das opções do TraceDraw](../../img/customize-options-app-clip-frame.png)

- [[options.auto_center]]: [[options.auto_center_outside]] (de início),
  [[options.auto_center_always]] ou [[options.auto_center_never]].
- [[options.empty_frame_lines]] (ligado).

Veja [Colocar objetos dentro de um quadro](../objects/clip-frames.md).

## Encaixe {#snapping}

![A página Encaixe das opções do TraceDraw](../../img/customize-options-app-snapping.png)

- [[options.snap_to_objects]], [[options.snap_to_page]],
  [[options.snap_to_guidelines]], [[options.snap_to_grid]], a grade de linha de
  base e [[options.snap_to_pixels]].
- [[options.snapping_radius]] (10 pixels de início).
- [[options.show_snap_marks]] e a [[options.snap_screen_tip]].
- [[options.snap_modes]]: os pontos aos quais os objetos se encaixam, com
  [[options.select_all]] e [[options.deselect_all]].

Veja [Encaixe](../pages-layout/snapping.md).

## Salvar {#save}

![A página Salvar das opções do TraceDraw](../../img/customize-options-app-save.png)

- [[options.backup]]: [[options.backup_before_save]] (ligado) guarda o arquivo
  anterior como `backup_of_<name>`, [[options.backup_to]] a
  [[options.same_folder]] ou um lugar escolhido.
- [[options.auto_backup]] (ligado): [[options.backup_every]] 20
  [[options.minutes]], em uma pasta temporária ou um lugar escolhido. Depois de
  uma falha as cópias são oferecidas; veja
  [Cópia automática e recuperação](../get-started/open-and-save.md#recovery).

## Texto {#text}

![A página Texto das opções do TraceDraw](../../img/customize-options-app-text.png)

A página tem cinco abas.

- [[options.display]]: [[options.keyboard_text_increment]] (quanto <kbd>Ctrl+8</kbd>
  e <kbd>Ctrl+2</kbd> aumentam e diminuem o texto selecionado, 1 ponto), a
  [[options.default_font]] e o [[options.default_size]] do texto novo, e
  [[menu.text.show_non_printing]].
  - [[options.text_units]] pt, mm, cm, in, px…
- [[options.paragraph_text]]: [[menu.text.show_text_frames]],
  [[menu.text.use_hyphenation]] e os [[options.hyphenation_patterns]]
  (incluídos, ou um dicionário de hifenização do seu idioma).
- [[docker.fonts]]: [[options.show_recent_fonts]] no topo das listas de fontes,
  o [[options.recent_font_count]] (5, até 20) e o
  [[options.font_sample_size]].
- [[options.spelling]]: [[writing.ignore_digits]], [[writing.ignore_capitals]]
  e [[options.add_to_autocorrect]], para que uma palavra corrigida no corretor
  ortográfico seja corrigida ao digitar da próxima vez; e os
  [[options.dictionaries]] que ele usa, veja
  [Dicionários](../text/writing-tools.md#dictionaries).
- [[dialog.autocorrect]]: o que é corrigido ao digitar; veja
  [Correção automática](../text/writing-tools.md#autocorrect).
