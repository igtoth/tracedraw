# Desfazer, zoom e deslocamento

Você pode desfazer qualquer alteração e olhar o desenho em qualquer nível de
zoom e de qualquer posição. Zoom e deslocamento nunca alteram o desenho.

## Desfazer e refazer {#undo}

- [[menu.edit > menu.edit.undo]] (<kbd>Ctrl+Z</kbd>) desfaz a última
  alteração; [[menu.edit.redo]] (<kbd>Ctrl+Shift+Z</kbd>) a refaz.
- As setas ao lado de Desfazer e Refazer na barra de ferramentas padrão listam
  os passos. Escolha um passo para desfazer ou refazer todas as alterações até
  ele.
- [[menu.edit.repeat]] aplica o último comando de novo, por exemplo ao próximo
  objeto que você selecionar. Um deslocamento, redimensionamento ou rotação é
  repetido em torno do centro, lado ou canto do próprio objeto selecionado, que
  gira ou cresce onde está em vez de sair do lugar.

### O painel Desfazer {#undo-docker}

O painel [[docker.undo]] lista todas as alterações do desenho, da mais antiga
para a mais nova. Clique em um passo para levar o desenho de volta a como
estava depois dele; clique em um passo posterior para avançar de novo. Abra-o
em [[menu.window > menu.window.dockers]].

![O painel Desfazer](../../img/get-started-undo-and-views-undo-docker.png)

## Zoom {#zoom}

- Gire a roda do mouse para aproximar ou afastar em torno do ponteiro.
- Use a caixa de zoom na barra de ferramentas padrão: digite um nível ou
  escolha um na lista (10% a 1600%), ou ajuste à seleção, a todos os objetos, à
  página, à largura ou à altura da página.
- [[menu.view > menu.view.zoom_in]] e [[menu.view.zoom_out]] mudam um passo;
  [[menu.view.zoom_to_fit]] (<kbd>F4</kbd>) mostra todos os objetos,
  [[menu.view.zoom_to_page]] (<kbd>Shift+F4</kbd>) a página e
  [[menu.view.zoom_to_selected]] (<kbd>Shift+F2</kbd>) a seleção.
- O gesto de pinça no trackpad muda o zoom, e numa tela sensível ao toque dois
  dedos deslocam e ampliam com qualquer ferramenta.

### A ferramenta Zoom {#zoom-tool}

Escolha a ferramenta [[tool.zoom]] (<kbd>Z</kbd> ou <kbd>F2</kbd>) e:

- clique para aproximar duas vezes naquele ponto;
- <kbd>Shift</kbd>+clique ou clique com o botão direito para afastar duas
  vezes;
- arraste um retângulo para que ele preencha a janela.

Clique duas vezes no botão da ferramenta para ajustar o zoom a todos os
objetos.

## Deslocamento {#pan}

- Mantenha o botão do meio do mouse pressionado e arraste, com qualquer
  ferramenta.
- Escolha a ferramenta [[tool.pan]] (<kbd>H</kbd>) e arraste.
- <kbd>Ctrl</kbd>+roda rola para cima e para baixo, <kbd>Alt</kbd>+roda para os
  lados.
- Use as barras de rolagem, que cobrem a página e o tamanho de uma página ao
  redor.
- Mantenha pressionado o botão Navegador, no canto entre as barras de rolagem,
  e mova o ponteiro sobre a miniatura da página.
- <kbd>PgUp</kbd> e <kbd>PgDn</kbd> vão para a página anterior e a próxima.

Para que a roda role em vez de mudar o zoom, mude a ação da roda do mouse na
página [[options.zoom_pan]] de
[[menu.tools > menu.tools.options > menu.tools.options_tools]]. Então a roda
rola, <kbd>Shift</kbd>+roda rola para os lados e <kbd>Ctrl</kbd>+roda muda o
zoom. Veja [Opções das ferramentas](../customize/tool-options.md#zoom-pan).

## Modos de exibição {#views}

O menu [[menu.view]] escolhe como os objetos são desenhados:
[[menu.view.wireframe]] mostra só os contornos, [[menu.view.normal]] e
[[menu.view.enhanced]] desenham preenchimentos e efeitos, e
[[menu.view.pixels]] mostra o desenho como os pixels de uma exportação.
[[menu.view.fullscreen_preview]] mostra só a página na tela; pressione qualquer
tecla para voltar. [[menu.view.preview_selected_only]] limita essa
visualização à seleção. O menu [[menu.view]] também mostra ou oculta as
réguas, as grades, as linhas-guia, o sangramento e a área imprimível; veja
[Réguas, grades e linhas-guia](../pages-layout/guidelines-and-grid.md).
