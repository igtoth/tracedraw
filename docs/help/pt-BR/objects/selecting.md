# Selecionar objetos

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Antes de alterar um objeto, você o seleciona. A ferramenta [[tool.pick]]
(<kbd>Space</kbd> volta para ela a partir da maioria das ferramentas, e
<kbd>Space</kbd> de novo devolve a ferramenta que você usava) seleciona, move e redimensiona objetos; a ferramenta [[tool.freehand_pick]] faz
o mesmo, mas seleciona com um laço à mão livre.

## A ferramenta Seleção {#pick}

- **Clique** em um objeto para selecioná-lo. Um objeto preenchido conta onde
  está preenchido; um sem preenchimento, no contorno. Texto, grupos, tabelas e
  imagens contam em qualquer lugar dentro da caixa.
- <kbd>Shift</kbd>+clique acrescenta um objeto à seleção ou o tira dela.
- **Arraste** em volta de objetos, começando em um lugar vazio: os objetos
  totalmente dentro da moldura tracejada são selecionados. Mantenha
  <kbd>Alt</kbd> pressionado ao terminar o arraste para selecionar todos os
  objetos que a moldura toca; pressionado desde o início, a moldura pode
  começar também sobre um objeto (um fundo). Mantenha <kbd>Shift</kbd> para inverter os
  objetos que a moldura pega: os selecionados saem da seleção e os outros
  entram. Mantenha <kbd>Ctrl</kbd> para manter a moldura quadrada.
- <kbd>Ctrl</kbd>+clique seleciona um objeto dentro de um grupo, um nível mais
  para dentro a cada clique. As alças ficam redondas, e as alterações valem só
  para ele; arraste-o de qualquer ponto para movê-lo sozinho.
  <kbd>Ctrl</kbd>+clique em uma moldura de recorte começa a editar o conteúdo
  dela (veja [Molduras de recorte](clip-frames.md#edit-contents)).
- <kbd>Alt</kbd>+clique seleciona o objeto que está embaixo do selecionado,
  para alcançar objetos cobertos.
- <kbd>Tab</kbd> e <kbd>Shift+Tab</kbd> selecionam o próximo ou o anterior.
- <kbd>Esc</kbd> desfaz a seleção. [[menu.edit > menu.edit.select_all]] e um
  clique duplo no botão da ferramenta selecionam todos os objetos;
  [[menu.edit.select_all]] também seleciona todo o texto, as linhas-guia ou os
  nós.

Os objetos selecionados mostram oito alças quadradas em volta e uma pequena
cruz no meio. A barra de status diz o que está selecionado e em qual camada.

O ponteiro mostra as setas de mover só sobre o que está selecionado. Sobre
qualquer outro objeto ele continua a seta simples, porque pressionar ali
escolhe esse objeto primeiro; arrastar um objeto não selecionado ainda o
escolhe e move. Um clique com o botão direito enquanto você arrasta um quadro
de seleção é ignorado: o quadro continua e seleciona quando você solta o botão
esquerdo.

Ferramentas de desenho e de modelagem cujo clique não tem função própria (as
ferramentas de retângulo, elipse, polígono e outras formas, a Faca, os
conectores, Mistura e outras) também selecionam: clique em um objeto para
selecioná-lo, <kbd>Shift</kbd>+clique para acrescentá-lo ou tirá-lo,
<kbd>Ctrl</kbd>+clique para entrar em grupos, e clique no vazio para não
selecionar nada. Arrastar continua desenhando. As ferramentas de linha e a
ferramenta Texto mantêm os cliques delas.

### Mover, redimensionar e girar {#transform-with-pick}

- Arraste um objeto selecionado, ou a cruz no meio dele, para movê-lo.
  Mantenha <kbd>Ctrl</kbd> pressionado para mover só na horizontal ou na
  vertical. O objeto se encaixa no que o encaixe estiver configurado.
- Arraste uma alça de canto para redimensionar em proporção, uma alça lateral
  para esticar em uma direção. Mantenha <kbd>Shift</kbd> para redimensionar a
  partir do centro, <kbd>Ctrl</kbd> para redimensionar em passos de 100%
  (passar do outro lado espelha o objeto), <kbd>Alt</kbd> para redimensionar
  livremente.
- Clique de novo em um objeto selecionado para mostrar as alças de rotação:
  arraste uma seta curva de canto para girar, uma seta lateral para inclinar.
  Mantenha <kbd>Ctrl</kbd> para passos de 15 graus. Arraste o anel do meio para
  girar em torno de outro ponto; clique duas vezes nele para devolvê-lo ao
  meio. Nesse modo de rotação, um nó da seleção (ou o meio de um segmento)
  arrastado gira a seleção em torno do anel; solte perto da borda de outro
  objeto e o nó cai exatamente nela, então um arco girado em torno de uma
  ponta pode ser deitado sobre uma linha. A alça de rotação ainda se encaixa
  nos nós, pontos médios e centros de outros objetos para mirar o giro.
- Durante o arraste, pressione <kbd>Space</kbd> ou clique com o botão direito
  para deixar uma cópia onde os objetos estavam; <kbd>Esc</kbd> cancela o
  arraste.
- As setas do teclado deslocam a seleção pela distância de deslocamento (2,54
  mm de início), com <kbd>Shift</kbd> pela distância maior e com
  <kbd>Ctrl</kbd> pela menor. Defina-as na página [[options.rulers]] das opções
  do documento.
- <kbd>+</kbd> duplica a seleção no mesmo lugar.

### Arrastar com o botão direito {#right-drag}

Arraste um objeto selecionado com o botão direito do mouse. Ao soltar, um menu
oferece movê-lo para ali, copiá-lo para ali, copiar o preenchimento, o contorno
ou todas as propriedades para o objeto sob o ponteiro, ou colocá-lo dentro
desse objeto como [moldura de recorte](clip-frames.md).

### A barra de propriedades {#pick-property-bar}

Sem nada selecionado, a barra de propriedades mostra o tamanho e a orientação
da página, as unidades de desenho e a distância de deslocamento. Com uma
seleção, mostra a posição (X e Y), o tamanho (L e A), a escala, o ângulo de
rotação, os botões de espelhar, a largura do contorno, trazer para a frente e
enviar para trás, e converter em curvas. Texto acrescenta as configurações de
texto, e bitmaps os comandos de bitmap. Com vários objetos, um grupo ou uma
curva de várias partes, ela acrescenta agrupar, desagrupar, combinar, separar,
os comandos de modelagem e o painel [[docker.align_distribute]].

### Grupos de seleção {#selection-groups}

Pressione <kbd>Ctrl</kbd> e um número de 0 a 9 para guardar a seleção com esse
número. Pressione só o número para selecionar esses objetos de novo, e outra
vez para dar zoom neles. <kbd>Alt</kbd> e o número acrescentam a seleção ao
grupo. Os grupos são salvos com o desenho.

## A ferramenta Seleção à mão livre {#freehand-pick}

A ferramenta [[tool.freehand_pick]] funciona como a ferramenta [[tool.pick]],
mas um arraste a partir de um lugar vazio desenha um laço. Ao soltar, todos os
objetos totalmente dentro do laço são selecionados; mantenha <kbd>Alt</kbd>
pressionado ao soltar para selecionar todos os que o laço toca, ou
<kbd>Shift</kbd> para acrescentar à seleção. Mantenha <kbd>Ctrl</kbd>
pressionado para desenhar um retângulo.

As opções da ferramenta [[tool.pick]], como se uma moldura seleciona os objetos
que toca, ficam na página [[tool.pick]] das opções de ferramentas; veja
[Opções das ferramentas](../customize/tool-options.md#pick).
