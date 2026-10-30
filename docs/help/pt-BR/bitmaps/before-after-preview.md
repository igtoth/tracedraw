# A prévia de antes e depois

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Toda caixa de diálogo que muda uma imagem mostra a imagem antes e depois da
mudança na mesma prévia: Traçar bitmap, Remover fundo, as caixas de diálogo de
modo de cor (Preto e branco, Duotom, Paleta), todo efeito de bitmap e
[[fx.image_adjustments]]. Esta página explica como olhar a imagem ali.

## A caixa de diálogo {#layout}

Essas caixas de diálogo são janelas redimensionáveis: arraste uma borda ou um
canto, ou clique no botão de maximizar na barra de título. A prévia ocupa a
esquerda da janela e cresce com ela; as configurações da própria caixa de
diálogo ficam numa coluna à direita, com [[dialog.ok]] e [[dialog.cancel]]
embaixo.

A prévia acompanha sozinha cada mudança das configurações. Enquanto você
arrasta um controle deslizante, ela mostra na hora uma versão rápida e menor,
e a vista completa quando você solta. Nada é recalculado enquanto as
configurações ficam iguais, então a caixa de diálogo continua rápida com
imagens grandes.

## Escolher a vista {#views}

A lista [[dialog.trace_preview_label]] acima da imagem escolhe o que a prévia
mostra:

- [[dialog.trace_view_before_after]] (o padrão): a imagem em cima e o
  resultado embaixo.
- [[dialog.view_side_by_side]]: a imagem à esquerda e o resultado à direita.
- [[dialog.before]]: só a imagem como está.
- [[dialog.after]]: só o resultado.
- [[dialog.trace_view_wireframe]] (só em Traçar bitmap): as curvas do traçado
  desenhadas sobre a imagem, para ver quanto elas a seguem.
- [[dialog.view_split]] (só em [[fx.image_adjustments]]): uma imagem, o
  original à esquerda de uma linha tracejada e o resultado à direita. Arraste
  a linha para movê-la. [[fx.image_adjustments]] começa em [[dialog.after]].

Os rótulos Antes e Depois ficam nos cantos dos painéis. Partes transparentes
da imagem mostram um quadriculado.

## Aproximar e mover {#zoom-pan}

Os botões ao lado da lista:

- [[dialog.lab_zoom_fit]] (pressionado de início) mostra a imagem inteira num
  painel.
- [[dialog.lab_zoom_out]] e [[dialog.lab_zoom_in]] afastam e aproximam um
  quarto a cada clique.
- O botão de mover (pressionado de início) faz um arraste num painel mover a
  imagem.

A roda do mouse sobre um painel aproxima em torno do ponteiro, então o ponto
sob ele fica no lugar. Os dois painéis sempre compartilham o mesmo zoom e a
mesma posição: mover ou aproximar um move o outro do mesmo jeito, para você
comparar o mesmo ponto.

[[fx.image_adjustments]] também tem [[dialog.lab_rotate_left]] e
[[dialog.lab_rotate_right]], que giram a vista um quarto de volta (a imagem em
si não gira), e 100 %, que mostra um pixel da imagem por ponto da tela.

Uma marca girando depois do botão de mover quer dizer que a caixa de diálogo
ainda está calculando o resultado, como o traçado de uma imagem grande; a
janela continua respondendo enquanto isso.

## Pegar cores na prévia {#eyedropper}

Algumas caixas de diálogo têm um conta-gotas que pega uma cor da imagem, como
a cor de fundo a remover. Enquanto ele está ligado, um clique num painel pega
a cor sob o ponteiro e o botão de mover fica solto. Clique no botão de mover
para parar de pegar cores.

Veja também [Traçar bitmaps](tracing.md),
[Remover o fundo](remove-background.md),
[Modos de cor de bitmaps](color-modes.md) e
[Efeitos de bitmap](../effects/bitmap-effects.md).
