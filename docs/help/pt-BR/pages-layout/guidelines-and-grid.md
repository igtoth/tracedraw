# Réguas, grades e linhas-guia

Réguas, grades e linhas-guia ajudam a posicionar objetos com precisão. Nenhuma
delas é impressa.

## Réguas {#rulers}

As réguas ficam no alto e à esquerda da janela de desenho, nas unidades de
desenho; [[menu.view > menu.view.rulers]] as mostra ou oculta. Uma linha
tracejada em cada régua acompanha o ponteiro.

- Arraste a partir do canto onde as réguas se encontram para mover a **origem**,
  o ponto a partir do qual as réguas contam. As posições na barra de
  propriedades, na barra de status e nos painéis também contam a partir dela.
- Arraste a partir de uma régua para acrescentar uma linha-guia.
- Clique duas vezes em uma régua para abrir as configurações; veja
  [Opções do documento](document-options.md#rulers), onde você também define as
  unidades e a escala do desenho.

## A grade do documento {#grid}

[[menu.view > menu.view.grid > menu.view.document_grid]] mostra uma grade de
linhas ou pontos sobre a página, atrás dos objetos;
[[menu.view.snap_document_grid]] em [[menu.view.snap_to]] faz os objetos se
encaixarem nela. O espaçamento (10 mm de início) é definido em
[Opções do documento](document-options.md#grid). Quando as linhas ficariam
próximas demais, só cada segunda, quinta ou décima linha é desenhada.

A **grade de linha de base** ([[menu.view.baseline_grid]]) desenha linhas pela
página como um caderno pautado, para alinhar texto; a **grade de pixels**
([[menu.view.pixel_grid]]) mostra os pixels do desenho na vista
[[menu.view.pixels]] a partir de 800% de zoom.

## Linhas-guia {#guidelines}

As linhas-guia são linhas que atravessam a janela de desenho e às quais os
objetos se encaixam.

- Arraste a partir da régua de cima para uma linha-guia horizontal, da régua da
  esquerda para uma vertical.
- Com a ferramenta [[tool.pick]], clique em uma linha-guia para selecioná-la
  (<kbd>Shift</kbd>+clique acrescenta) e arraste-a para movê-la. Solte-a fora
  da janela de desenho, ou pressione <kbd>Delete</kbd>, para excluí-la.
- Clique de novo na linha-guia selecionada para mostrar as alças de rotação, e
  arraste-as para girá-la.
- Clique com o botão direito em uma linha-guia para excluí-la, bloqueá-la ou
  desbloqueá-la, ou abrir as propriedades. Linhas-guia bloqueadas podem ser
  selecionadas mas não movidas nem excluídas.
- [[menu.edit > menu.edit.select_all > menu.edit.select_all_guidelines]]
  seleciona todas as linhas-guia da página, e [[menu.view.guidelines]] as
  mostra ou oculta.
- Com linhas-guia selecionadas (e nenhum objeto), [[menu.edit.copy]] e
  [[menu.edit.paste]] fazem cópias no mesmo lugar, e [[menu.edit.duplicate]]
  faz cópias deslocadas pelo deslocamento de duplicação (definido em
  [[menu.layout > menu.layout.document_options]], Geral). O painel
  Transformações move, gira e redimensiona linhas-guia selecionadas como faz
  com objetos.
- As linhas-guia são salvas com o desenho em arquivos `.tdraw`. Arquivos
  `.cdr` não as guardam: ao salvar um, a barra de status avisa.

## O painel Linhas-guia {#guidelines-docker}

O painel [[docker.guidelines]] coloca linhas-guia por coordenadas. Abra-o em
[[menu.window > menu.window.dockers]].

![O painel Linhas-guia](../../img/pages-layout-guidelines-and-grid-guidelines-docker.png)

1. [[guides.show]] e [[guides.snap]] ativam as linhas-guia e o encaixe nelas.
2. Escolha o [[guides.type]]: horizontal, vertical ou inclinada.
3. Digite a posição ([[guides.x]], [[guides.y]], e o [[guides.angle]] para uma
   inclinada), contada a partir da origem da régua.
4. Clique em [[guides.add]] para acrescentar uma linha-guia, ou selecione uma na
   lista e clique em [[guides.modify]] para movê-la.

A lista mostra as linhas-guia da página desse tipo (<kbd>Ctrl</kbd>+clique para
selecionar várias). [[guides.delete]], [[guides.lock]] e [[guides.unlock]]
agem sobre as selecionadas, e [[guides.color]] e [[guides.style]]
([[guides.style_solid]], [[guides.style_dashed]], [[guides.style_dotted]] ou
[[guides.style_dash_dot]]) mudam a aparência delas.

Conjuntos prontos de linhas-guia (margens, colunas, grades) ficam na página
[[options.guidelines]] das [opções do documento](document-options.md#guidelines).
