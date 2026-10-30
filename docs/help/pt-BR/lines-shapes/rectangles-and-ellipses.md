# Desenhar retângulos e elipses

Retângulos e elipses continuam sendo retângulos e elipses depois de
desenhados: você pode mudar os cantos, transformar uma elipse em fatia ou arco
e ainda redimensioná-los livremente. Para editá-los nó por nó, converta-os em
curvas com [[menu.object > menu.object.convert_to_curves]].

## Desenhar com uma caixa {#drawing}

As ferramentas [[tool.rectangle]], [[tool.ellipse]], polígono, estrela,
espiral, papel milimetrado, formas comuns e linhas de ação desenham arrastando
uma caixa. Enquanto você arrasta, o próprio objeto segue o ponteiro, com as
configurações da barra de propriedades.

- Mantenha <kbd>Ctrl</kbd> pressionado para desenhar um quadrado, um círculo ou
  uma forma regular.
- Mantenha <kbd>Shift</kbd> pressionado para desenhar a partir do centro. Você
  pode pressioná-lo ou soltá-lo enquanto arrasta.
- Os pontos de início e fim se encaixam no que o encaixe estiver configurado.

O novo objeto recebe o preenchimento e o contorno padrão (sem preenchimento e
um contorno preto fino de início) e fica selecionado. Para mudar esses padrões,
veja [Estilos de objeto e padrões](../templates-styles/object-styles.md#document-defaults).

## Retângulo {#rectangle}

Escolha a ferramenta [[tool.rectangle]] (<kbd>F6</kbd>) e arraste de um canto
ao canto oposto. Clique duas vezes no botão da ferramenta para desenhar um
retângulo do tamanho da página; mantenha <kbd>Shift</kbd> pressionado ao clicar
duas vezes para emoldurar a seleção.

### Cantos {#rectangle-corners}

A barra de propriedades define os cantos dos retângulos selecionados, ou dos
novos quando nada está selecionado:

- O estilo do canto: **Arredondado** (um quarto de círculo), **Recortado** (um
  quarto de círculo cortado para dentro) ou **Chanfrado** (um corte reto).
- Os quatro tamanhos: superior esquerdo, superior direito, inferior esquerdo e
  inferior direito. O tamanho é o raio, ou, para o chanfro, a que distância do
  canto o corte começa. Um canto tem no máximo metade do lado menor.
- **Editar cantos juntos** (o cadeado, ativado de início) dá aos quatro o
  tamanho que você digitar em um.
- **Escala relativa dos cantos** (ativada de início): os cantos crescem e
  diminuem com o retângulo. Desativada, eles mantêm o tamanho quando você
  redimensiona o retângulo e continuam redondos quando você o estica.

Você também pode arrastar um nó de canto com a ferramenta [[tool.shape]] para
arredondar todos os cantos de uma vez.

## Retângulo de 3 pontos {#three-point-rectangle}

A ferramenta [[tool.three_point_rectangle]] desenha um retângulo inclinado:

1. Arraste para desenhar a linha da base, em qualquer ângulo.
2. Mova o ponteiro para definir a altura e clique.

## Elipse {#ellipse}

Escolha a ferramenta [[tool.ellipse]] (<kbd>F7</kbd>) e arraste uma caixa.
Mantenha <kbd>Ctrl</kbd> pressionado para um círculo.

Na barra de propriedades, escolha **Elipse**, **Fatia** ou **Arco**. Para uma
fatia ou um arco, defina os ângulos inicial e final (em graus, no sentido
anti-horário a partir da direita) e a direção. Você também pode arrastar o nó
de uma elipse com a ferramenta [[tool.shape]] para fazer uma fatia (arrastando
para dentro) ou um arco (para fora).

## Elipse de 3 pontos {#three-point-ellipse}

A ferramenta [[tool.three_point_ellipse]] desenha uma elipse inclinada:

1. Arraste para desenhar a linha central da elipse, em qualquer ângulo.
2. Mova o ponteiro para definir a altura e clique.

## O painel Cantos {#corners}

O painel [[docker.corners_docker]] arredonda, recorta ou chanfra os cantos de
curvas, o que as configurações do retângulo não fazem. Abra-o em
[[menu.window > menu.window.dockers]].

![O painel Cantos](../../img/lines-shapes-rectangles-and-ellipses-corners.png)

1. Selecione um ou mais objetos. Retângulos, elipses, polígonos e texto viram
   curvas quando os cantos são cortados.
2. Escolha a [[docker.corners_operation]]: [[docker.corners_fillet]] (um arco
   redondo que toca os dois lados), [[docker.corners_scallop]] (um entalhe, um
   quarto de círculo em volta do canto) ou [[docker.corners_chamfer]] (um corte
   reto).
3. Defina o [[docker.corners_radius]] para filete ou recorte (2 mm de início),
   ou a [[docker.corners_chamfer_distance]] para chanfro: a primeira distância
   ao longo do lado por onde o caminho chega e a segunda ao longo do lado por
   onde sai. [[docker.corners_lock]] as mantém iguais.
4. O resultado aparece como um contorno tracejado no desenho. Clique em
   [[docker.apply]].

Um canto é um nó onde dois segmentos se encontram em ângulo; nós suaves não
são cantos. Com a ferramenta [[tool.shape]] e alguns nós selecionados, só esses
cantos são cortados. Um canto cujos lados são curtos demais para o corte é
ignorado.
