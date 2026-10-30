# Mover, escalonar, girar e inclinar

Você pode transformar objetos arrastando-os com a ferramenta [[tool.pick]]
(veja [Selecionar objetos](selecting.md#transform-with-pick)), digitando
valores na barra de propriedades, com a ferramenta [[tool.free_transform]] ou,
com precisão, com o painel [[docker.transformations]].

## O painel Transformações {#transformations-docker}

Abra o painel com [[menu.object > menu.object.transformations]], ou com
<kbd>Alt+F7</kbd> ([[docker.position]]), <kbd>Alt+F8</kbd>
([[docker.rotate]]), <kbd>Alt+F9</kbd> ([[docker.scale_and_mirror]]) e
<kbd>Alt+F10</kbd> ([[docker.size]]). Os botões no alto escolhem uma de cinco
páginas.

![O painel Transformações](../../img/objects-transforming-transformations-docker.png)

Toda página termina com o **ponto de referência**, uma grade 3 por 3 que
escolhe o ponto da seleção que é posicionado, que serve de eixo de rotação ou
que fica parado (o centro de início), [[dialog.copies]] e [[docker.apply]]. As
distâncias ficam nas unidades de desenho, os ângulos em graus (sentido
anti-horário), as escalas em porcentagem.

Para transformar a seleção:

1. Escolha uma página e digite os valores.
2. Escolha o ponto de referência.
3. Defina [[dialog.copies]]: 0 altera a própria seleção; um número acima de 0
   deixa a seleção como está e acrescenta esse número de cópias, cada uma
   transformada uma vez mais que a anterior. As cópias são um passo de
   desfazer, e a última cópia passa a ser a seleção, então outro
   [[docker.apply]] continua a partir dali.
4. Clique em [[docker.apply]].

### Posição {#position}

[[docker.x]] e [[docker.y]] movem a seleção. Com [[docker.relative_position]]
ativada (de início), são uma distância a deslocar; desativada, são para onde
vai o ponto de referência, a partir da origem da régua. Com
[[docker.relative_position]] ativada, a grade escolhe uma direção: um lado ou
um canto define a distância como o tamanho da seleção, então cada cópia fica
logo ao lado da anterior.

### Girar {#rotate}

Digite o ângulo e o [[docker.center_colon]] em torno do qual girar. Com
[[docker.relative_center]] ativado (de início), o centro conta a partir do
ponto de referência; desativado, a partir da origem da régua.

### Escala e espelho {#scale-and-mirror}

[[docker.x]] e [[docker.y]] escalonam em porcentagem (100% de início).
[[toolbar.mirror_h]] e [[toolbar.mirror_v]] viram a seleção.
[[docker.proportional]] mantém as duas escalas iguais.

### Tamanho {#size}

[[docker.w]] e [[docker.h]] definem a largura e a altura da seleção.
[[docker.proportional]] mantém as proporções.

### Inclinar {#skew}

[[docker.x]] e [[docker.y]] inclinam a seleção em ângulos. Com
[[docker.use_anchor_point]] ativado, o ponto de referência fica parado;
desativado, o centro fica parado.

## A ferramenta Transformação livre {#free-transform}

A ferramenta [[tool.free_transform]], no submenu Seleção, transforma a seleção
em torno do ponto onde você pressiona. Escolha um modo na barra de
propriedades:

- **Rotação livre** (de início): gira a seleção em torno do ponto pressionado.
- **Reflexão em ângulo livre**: espelha a seleção pela linha que vai do ponto
  pressionado ao ponteiro; mover o ponteiro gira essa linha.
- **Escala livre**: escalona em torno do ponto pressionado.
- **Inclinação livre**: o movimento horizontal inclina na horizontal, o
  vertical na vertical.

Mantenha <kbd>Alt</kbd> pressionado para escalonar, <kbd>Ctrl</kbd> para
inclinar ou <kbd>Shift</kbd> para refletir em um arraste, qualquer que seja o
modo. Com **Aplicar à duplicata** ativado, o clique faz uma cópia e o arraste
transforma a cópia, deixando o original como estava. Um arraste é um único
passo: <kbd>Ctrl+Z</kbd> devolve a seleção ao que era antes do arraste (e
remove a cópia). A barra de propriedades
também tem os campos de posição, tamanho, escala, ângulo e espelho.

## Desfazer transformações {#clear}

[[menu.object > menu.object.clear_transformations]] devolve os objetos
selecionados a como eram antes de serem escalonados, girados ou inclinados.

## O painel Coordenadas {#coordinates}

O painel [[docker.coordinates]] desenha um objeto a partir de coordenadas
digitadas, ou remodela o selecionado, com uma prévia tracejada no desenho.
Abra-o em [[menu.window > menu.window.dockers]].

![O painel Coordenadas](../../img/objects-transforming-coordinates.png)

1. Escolha o tipo de objeto com os botões no alto: retângulo ou quadrado,
   elipse ou círculo, polígono ou polígono regular, estrela, estrela complexa,
   linha de 2 pontos ou curva de vários pontos.
2. Digite os valores:
   - retângulos e elipses: a posição do ponto de origem (escolhido em uma
     grade 3 por 3), a largura e a altura, o ângulo de rotação e os cantos da
     caixa delimitadora enquanto o ângulo for 0;
   - quadrados e círculos: a posição, o lado ou o diâmetro e o ângulo;
   - polígonos e estrelas: o número de pontas ou lados, a nitidez, o ângulo e o
     círculo onde as pontas ficam;
   - polígonos regulares: o número de lados, o comprimento de um lado, o ângulo
     e o centro;
   - linhas de 2 pontos: o ponto inicial e o final, ou o comprimento e o
     ângulo;
   - curvas de vários pontos: a lista de pontos, com botões para acrescentar e
     excluir pontos e para fechar a curva.
3. Em vez de digitar, clique em um botão com ponteiro e alvo e clique ou
   arraste no desenho para definir esse valor. Um clique com o botão direito
   cancela.
4. Clique em **Criar objeto** para acrescentá-lo, ou em **Substituir objeto**
   para remodelar o retângulo, a elipse, o polígono ou a curva selecionada,
   mantendo o preenchimento, o contorno, o nome e o lugar.

As posições contam a partir da origem da régua, nas unidades de desenho.
