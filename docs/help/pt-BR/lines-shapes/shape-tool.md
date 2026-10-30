# Moldar curvas com a ferramenta Forma

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

A ferramenta [[tool.shape]] (<kbd>F10</kbd>) edita os nós e segmentos de uma
curva: você move nós, curva segmentos, acrescenta e exclui nós e muda como cada
nó une seus segmentos.

## Nós e segmentos {#nodes}

Selecione uma curva com a ferramenta [[tool.shape]]. O contorno aparece na cor
dos nós, os nós como pequenos marcadores e o primeiro nó de cada parte da curva
como um triângulo um pouco maior. Um nó selecionado mostra suas alças de
controle: linhas tracejadas que terminam em uma seta no ponto de controle.

Um nó é de um de três tipos:

- **Cúspide**: os segmentos se encontram em ângulo; cada alça se move sozinha.
- **Suave**: a curva passa sem canto; as duas alças ficam alinhadas.
- **Simétrico**: suave, com as duas alças do mesmo comprimento.

O ponteiro mostra o que um clique pegaria: um pequeno quadrado sobre um nó, um
ponto sobre uma alça de controle, uma onda sobre um segmento.

## Selecionar nós {#selecting-nodes}

- Clique em um nó para selecioná-lo sozinho; <kbd>Ctrl</kbd>+clique o
  acrescenta ou o tira da seleção.
- <kbd>Shift</kbd>+clique seleciona todos os nós desde o último selecionado até
  o que você clicar.
- Arraste uma área em volta dos nós. O modo de seleção da barra de
  propriedades escolhe uma área retangular ou à mão livre; <kbd>Alt</kbd>+arraste
  sempre desenha uma à mão livre, e <kbd>Alt+Shift</kbd>+arraste soma os nós dela
  aos selecionados.
- <kbd>Tab</kbd> e <kbd>Shift+Tab</kbd> selecionam o próximo ou o anterior;
  <kbd>Home</kbd> e <kbd>End</kbd> o primeiro ou o último.
- <kbd>Ctrl+A</kbd>, o botão de selecionar todos os nós ou um clique duplo no
  botão da ferramenta seleciona todos os nós.

## Mover nós e curvar segmentos {#editing}

- Arraste um nó para mover os nós selecionados. Mantenha <kbd>Ctrl</kbd>
  pressionado para mover só na horizontal ou na vertical. Os nós se encaixam
  como pontos de desenho, também nos outros nós e segmentos da própria curva.
- Arraste um segmento para curvá-lo: o ponto que você pega segue o ponteiro
  enquanto os nós das pontas ficam parados. Um segmento reto fica curvo. Nós
  cúspide nas pontas continuam cúspide, e as outras alças deles ficam onde
  estão.
- Arraste uma alça de controle para mudar a curvatura.
- As setas do teclado deslocam os nós selecionados (<kbd>Shift</kbd> para um
  passo maior, <kbd>Ctrl</kbd> para um menor).
- Clique duas vezes em um segmento para acrescentar um nó ali, ou em um nó para
  excluí-lo. Clique em um segmento para marcar um ponto e pressione
  <kbd>+</kbd> para acrescentar um nó ali.
- <kbd>Delete</kbd> ou <kbd>-</kbd> exclui os nós selecionados; <kbd>+</kbd>
  acrescenta um nó no meio de cada segmento que termina neles. Nós vizinhos
  excluídos juntos deixam uma curva ajustada ao contorno antigo, então a forma
  mantém o desenho geral.
- Com nós selecionados, <kbd>C</kbd> alterna entre cúspide e suave,
  <kbd>S</kbd> entre suave e simétrico, e <kbd>Y</kbd> os torna simétricos.
- Com nós selecionados, <kbd>Ctrl+C</kbd> copia os segmentos entre eles como
  novas curvas, <kbd>Ctrl+X</kbd> os recorta e <kbd>Ctrl+D</kbd> os duplica.

Clique com o botão direito em um nó ou segmento para um menu com os comandos
abaixo.

Com vários objetos selecionados, o objeto escolhido por último mostra o
contorno na cor dos nós (azul) e os outros na cor secundária dos nós
(vermelho), também enquanto um nó é arrastado, para você distingui-los. Nós de
ponta de dois objetos diferentes não se unem, nem com **Unir nós** nem
soltando um sobre o outro: combine os objetos primeiro
([[menu.object > menu.object.combine]], <kbd>Ctrl+L</kbd>) e depois una as
pontas.

## Arredondar cantos de curvas {#corner-rings}

Anéis dentro dos cantos de uma curva permitem arredondar um canto arrastando.
Eles vêm desligados, para não atrapalhar a edição de nós; ligue
[[options.corner_widgets]] na página [[options.nodes_and_handles]] de
[[menu.tools > menu.tools.options > menu.tools.options_app]] para usá-los.

- Arraste o anel de um canto entre dois lados retos para arredondá-lo. Um
  canto arredondado mantém o anel: arraste-o de novo para mudar o raio, mais
  para dentro ou de volta a um canto vivo. Isso também funciona em cantos
  arredondados em desenhos salvos e com o painel [[docker.corners]].
- Numa curva esticada, os cantos ficam redondos na página, não ovais.
  Retângulos arredondam os cantos pelos próprios nós, seja qual for esta
  opção.

## A barra de propriedades {#property-bar}

- **Modo de seleção**: área retangular ou à mão livre.
- **Acrescentar nó** e **Excluir nó**.
- **Unir nós** junta dois nós de ponta em um; **Quebrar curva** divide a curva
  no nó selecionado.
- **Converter em linha** e **Converter em curva** tornam reto ou curvo o
  segmento que termina no nó.
- **Cúspide**, **Suave** e **Simétrico** definem o tipo do nó.
- **Inverter direção** vira a curva; **Estender curva para fechar** une dois
  nós de ponta selecionados com um segmento reto; **Extrair subcaminho** torna a
  parte do nó selecionado uma curva própria; **Fechar curva** fecha uma curva
  aberta.
- **Esticar e escalonar nós** e **Girar e inclinar nós** emolduram dois ou mais
  nós selecionados com alças: as alças dos cantos escalonam (ou giram), as dos
  lados esticam (ou inclinam) só esses nós.
- **Alinhar nós** abre a caixa de diálogo [[dialog.node_align]] (veja abaixo).
- **Refletir na horizontal** e **Refletir na vertical**: mover um nó move do
  modo espelhado os nós selecionados do outro lado, para os dois lados de uma
  forma mudarem juntos.
- **Modo elástico**: os nós selecionados se movem menos quanto mais longe
  estiverem do que você arrasta, até metade para o mais distante.
- **Selecionar todos os nós**.
- **Reduzir nós** remove nós desnecessários, dos nós selecionados ou da curva
  inteira. Arraste o controle **suavidade da curva** para remover mais nós aos
  poucos.

## Alinhar nós {#align-nodes}

Para alinhar nós:

1. Selecione dois ou mais nós com a ferramenta [[tool.shape]].
2. Clique em **Alinhar nós** na barra de propriedades.
3. Na caixa de diálogo [[dialog.node_align]], marque
   [[dialog.align_horizontal]], [[dialog.align_vertical]] ou os dois, e clique
   em [[dialog.ok]].

Os nós selecionados vão para uma linha horizontal ou vertical comum, ou para um
único ponto quando os dois estão marcados.

As teclas de alinhamento também agem sobre os nós selecionados, dois ou mais:
<kbd>L</kbd> e <kbd>R</kbd> os alinham com o nó mais à esquerda ou mais à
direita, <kbd>T</kbd> e <kbd>B</kbd> com o nó de cima ou de baixo, e
<kbd>E</kbd> os põe no meio da altura deles. Os pontos de controle vão junto.
(<kbd>C</kbd> continua tornando o nó cúspide aqui.)

![A caixa de diálogo Alinhar nós](../../img/lines-shapes-shape-tool-align-nodes.png)

## Retângulos, elipses, polígonos e outros objetos {#shapes}

- Arraste um nó de canto de um retângulo para arredondar todos os cantos;
  <kbd>Ctrl</kbd>+arraste (ou clique nele antes) para arredondar só esse canto.
- Arraste o nó de uma elipse para dentro para fazer uma fatia, para fora para
  fazer um arco.
- Arraste um nó de um polígono ou estrela: os nós correspondentes acompanham,
  então a forma continua simétrica. Arrastar os pontos entre os cantos afia ou
  torce uma estrela.
- Clique duas vezes em um retângulo, elipse ou polígono para transformá-lo em
  curva.
- Arraste um canto de uma imagem para recortá-la: a imagem vai para uma moldura
  cujos nós você edita como uma curva.
- Com texto, a ferramenta [[tool.shape]] move caracteres individuais; veja
  [Adicionar e editar texto](../text/adding-text.md).

A aparência dos nós e das alças é definida na página
[[options.nodes_and_handles]] das opções; veja
[Opções do TraceDraw](../customize/options-app.md#nodes-and-handles).
