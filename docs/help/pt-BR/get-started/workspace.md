# O espaço de trabalho

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

A janela do TraceDraw contém seus desenhos e tudo o que é preciso para
trabalhar neles. Esta página dá nome às suas partes, para que as outras páginas
desta Ajuda façam sentido.

## As partes da janela {#parts}

De cima para baixo e da esquerda para a direita:

- A **barra de menus** contém todos os comandos, nos menus [[menu.file]],
  [[menu.edit]], [[menu.view]], [[menu.layout]], [[menu.object]],
  [[menu.effects]], [[menu.bitmaps]], [[menu.text]], [[menu.table]],
  [[menu.tools]], [[menu.window]] e [[menu.help]]. Cada comando mostra seu
  atalho ao lado. Sem nenhum desenho aberto, só aparecem [[menu.file]],
  [[menu.tools]], [[menu.window]] e [[menu.help]]. A página
  [Comandos de menu](../reference/menus.md) lista todos os comandos.
- A **barra de ferramentas padrão** contém os comandos mais usados (veja
  abaixo).
- A **barra de propriedades** mostra as configurações do que você está
  fazendo agora.
- As **guias de documentos** mostram uma guia por desenho aberto.
- A **caixa de ferramentas**, à esquerda, contém as ferramentas.
- As **réguas** ficam no alto e à esquerda da janela de desenho.
- A **janela de desenho** mostra a página sobre a área de trabalho ao redor.
- Os **painéis** ficam à direita.
- A **paleta de cores** fica embaixo.
- A **barra de status**, bem embaixo, descreve a ferramenta ativa e a seleção.

## A barra de ferramentas padrão {#standard-toolbar}

A barra de ferramentas padrão contém, em grupos:

1. Novo, Abrir (a seta lista os desenhos usados por último), Salvar e
   Imprimir.
2. Recortar, Copiar e Colar.
3. Desfazer e Refazer. As setas listam os passos, para você desfazer ou
   refazer vários de uma vez escolhendo um passo.
4. Importar, Exportar e Publicar em PDF.
5. A caixa de zoom: digite um nível de zoom e pressione <kbd>Enter</kbd>, ou
   abra a lista para ajustar o zoom à seleção, a todos os objetos, à página, à
   largura ou à altura da página, e níveis de 10% a 1600%. Depois, a
   visualização em tela cheia.
6. Mostrar réguas, mostrar grade e mostrar linhas-guia, que aparecem
   pressionados quando ativos.
7. Desativar o encaixe (<kbd>Alt+Q</kbd>) e a lista do que encaixar.
8. Opções, que abre a lista de páginas de opções.
9. Iniciar: Gerenciador de fontes, Tela de boas-vindas e Gerenciamento de
   cores.

Passe o ponteiro sobre um botão para ver o nome, o atalho e o que ele faz.

## A barra de propriedades {#property-bar}

A barra de propriedades muda conforme o que você faz. Sem nada selecionado e
com a ferramenta [[tool.pick]] ativa, ela mostra o tamanho e a orientação da
página, as unidades de desenho e a distância de deslocamento. Com objetos
selecionados, mostra a posição, o tamanho, a escala, o ângulo de rotação e os
botões de espelhar, depois as configurações daquele tipo de objeto. Cada
ferramenta acrescenta as suas próprias configurações, explicadas na página da
ferramenta.

Nos campos numéricos você pode digitar uma conta como `10+5`, ou um valor em
outra unidade como `1 in`, que é convertido para as unidades de desenho.

## A caixa de ferramentas e os submenus {#toolbox}

A caixa de ferramentas tem um botão por grupo de ferramentas. Um botão com uma
pequena seta no canto contém um **submenu** de ferramentas relacionadas. Para
abri-lo, clique na seta, clique no botão com o botão direito ou mantenha-o
pressionado. O botão passa a mostrar a última ferramenta que você usou do
grupo. Passe o ponteiro sobre uma ferramenta para ver o nome, o atalho e o que
ela faz.

Clicar duas vezes em alguns botões tem uma ação rápida: [[tool.pick]]
seleciona todos os objetos, [[tool.zoom]] ajusta o zoom a todos os objetos e
[[tool.rectangle]] acrescenta um retângulo do tamanho da página.

O botão **+** no fim da caixa de ferramentas lista os grupos com uma caixa de
seleção para mostrar ou ocultar cada um, e restaura a caixa de ferramentas. A
página [Caixa de ferramentas](../reference/toolbox.md) lista todas as
ferramentas.

## Painéis {#panels}

Os painéis guardam configurações que ficam abertas enquanto você trabalha,
como [[docker.properties]], [[docker.objects]] e [[docker.hints]], que vêm
abertos. Os painéis abertos são guias à direita; clique em uma guia para trazer
o painel para a frente, ou na guia da frente para recolher os painéis. Abra
outros em [[menu.window > menu.window.dockers]] ou pelos atalhos; o botão de
três pontos abaixo das guias lista todos os painéis, cada um com a imagem,
para mostrar ou ocultar. Quando há mais painéis abertos do que cabem na coluna
de guias, a coluna desliza: use as setas no topo e no pé dela ou a roda do
mouse sobre ela. Cada painel tem botões de recolher e fechar na
barra de título. A página [Painéis](../reference/dockers.md) lista todos os
painéis.

## A janela de desenho {#drawing-window}

A página fica sobre a área de trabalho ao redor, onde você também pode deixar
objetos. Abaixo do desenho ficam:

- o **navegador de páginas**: inserir página antes, primeira página, página
  anterior, o número da página, próxima página, última página e inserir página
  depois;
- uma **guia por página** (clique para ir à página, clique duas vezes para
  renomeá-la);
- a barra de rolagem horizontal. Arraste o divisor pontilhado entre as guias
  das páginas e a barra de rolagem para dividir o espaço entre elas.

O botão **Navegador**, no canto entre as barras de rolagem, mostra uma
miniatura da página enquanto você o mantém pressionado; mova o ponteiro sobre a
miniatura para mover a vista. Clicar duas vezes na borda da página com a
ferramenta [[tool.pick]] abre as opções de tamanho da página.

## A paleta de cores {#color-palette}

Clique em uma cor da paleta para preencher os objetos selecionados; clique com
o botão direito para colorir o contorno deles. A paleta do documento, abaixo da
janela de desenho, reúne as cores que o desenho usa. Veja
[Paletas de cores](../color/palettes.md).

## A barra de status {#status-bar}

A barra de status mostra uma dica da ferramenta ativa, o que está selecionado e
em qual camada, e o preenchimento e o contorno da seleção. Clique duas vezes na
parte do preenchimento ou do contorno para alterá-los. A página
[Barra de status](../reference/status-bar.md) explica cada parte.

## Organizar as barras {#arranging}

Você pode mover a caixa de ferramentas, a paleta de cores, a barra de
ferramentas padrão, a barra de propriedades e a barra de status: arraste uma
barra pela alça pontilhada até outra borda da janela, ou para longe das bordas
para que ela flutue. Clique com o botão direito em uma alça para travar as
barras ou restaurar suas posições. Para mostrar ou ocultar uma barra, use
[[menu.window > menu.window.toolbars]]. Você também pode salvar toda a
disposição como um espaço de trabalho; veja
[Espaços de trabalho](../customize/workspaces.md).

A barra de ferramentas padrão, a barra de propriedades, as barras Texto, Zoom
e Transformar e o controle de tempo podem dividir uma linha quando encaixados
no alto ou embaixo: arraste a alça de uma barra até a linha de outra para
pô-la ao lado dela, ou até a divisa entre duas linhas para lhe dar uma linha
própria. As linhas ficam guardadas nas suas preferências. Um clique com o
botão direito em qualquer parte de uma barra encaixada abre o menu dela.
Menus mais altos que a janela rolam com a roda do mouse.

Todo controle deslizante das barras, painéis e caixas de diálogo funciona do
mesmo jeito: o rótulo acima, um sulco com marcas, e o valor numa caixa à
direita em que você pode digitar ou avançar com as setas. Clicar no sulco
leva o cursor até ali sem tomar o teclado, então os atalhos da tela continuam
funcionando; depois que <kbd>Tab</kbd> lhe dá o foco, as setas o avançam
(<kbd>Page Up</kbd> e <kbd>Page Down</kbd> de dez em dez, <kbd>Home</kbd> e
<kbd>End</kbd> até as pontas). Todo controle mostra um título e uma descrição
quando você passa o ponteiro.

## Trabalhar com vários desenhos {#several-drawings}

Cada desenho aberto tem sua guia acima das réguas, com o nome e um `*` enquanto
houver alterações não salvas. A guia depois da última cria um desenho novo.
Cada desenho guarda suas páginas, histórico de desfazer, seleção, zoom e
unidades.

[[menu.window > menu.window.cascade]], [[menu.window.tile_horizontally]] e
[[menu.window.tile_vertically]] mostram cada desenho em uma janela própria;
[[menu.window.combine_windows]] traz as guias de volta.
[[menu.window.new_window]] abre uma segunda janela do mesmo desenho, por
exemplo para ver duas páginas ou dois níveis de zoom ao mesmo tempo; uma edição
em uma janela aparece em todas.

## O tamanho da interface {#interface-size}

Se os menus e botões parecem pequenos ou grandes demais, escolha
[[menu.tools > menu.tools.options > menu.tools.options_customization]] e ajuste o
tamanho da interface na página [[options.appearance]]. O tamanho da interface
nunca muda o tamanho do desenho. Veja
[Opções de personalização](../customize/customization.md#appearance).
