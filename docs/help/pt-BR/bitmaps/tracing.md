# Vetorizar bitmaps

A vetorização transforma um bitmap em formas vetoriais que você pode editar,
recolorir e redimensionar sem perder qualidade: um logotipo digitalizado, um
esboço, uma foto.

## Vetorização rápida {#quick-trace}

Selecione um bitmap e escolha [[menu.bitmaps > menu.bitmaps.quick_trace]].
Ela usa a predefinição escolhida na página [[options.bitmap_tracing]] das
opções globais (Clipart no início).

[[menu.bitmaps.repeat_last_trace]] vetoriza o bitmap selecionado de novo como a última vetorização, sem caixa de diálogo, e [[menu.bitmaps.last_trace_settings]] abre a caixa de diálogo com as configurações dela. O menu de contexto de um bitmap também tem esses comandos, junto com [[menu.bitmaps.centerline_trace]] e [[menu.bitmaps.outline_trace]].

## A caixa de diálogo Vetorizar bitmap {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] e
[[menu.bitmaps.centerline_trace]] listam predefinições; escolha uma para
abrir a caixa de diálogo.

- **Vetorização de contorno** preenche áreas de cor com formas:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] e [[menu.bitmaps.trace_high_quality]].
- **Vetorização de linha central** segue as linhas com curvas abertas:
  [[menu.bitmaps.trace_technical]] e [[menu.bitmaps.trace_line_drawing]].

![A caixa de diálogo Vetorizar bitmap](../../img/bitmaps-tracing.png)

A caixa de diálogo é uma janela que você pode redimensionar e maximizar. A
visualização ocupa o lado esquerdo, as configurações ficam em um painel à
direita, e a vetorização é refeita sempre que você muda uma configuração:
não há nada a apertar para ver o resultado. Abaixo da visualização, uma
linha conta as **curvas**, os **nós** e as **cores** do resultado. Clique
em [[dialog.ok]] para colocar as formas sobre o bitmap, as mais escuras
primeiro, ou em [[dialog.cancel]] para deixar o bitmap como está.

## A visualização {#preview}

A lista [[dialog.trace_preview_label]] escolhe o que a visualização mostra:

- [[dialog.trace_view_before_after]]: o bitmap em cima e sua vetorização
  embaixo, ou [[dialog.view_side_by_side]]; [[dialog.before]] mostra só o
  bitmap.
- [[dialog.trace_view_large]]: só a vetorização, do tamanho da
  visualização.
- [[dialog.trace_view_wireframe]]: as curvas do resultado desenhadas como
  linhas finas sobre o bitmap. Ao lado da lista aparece o controle
  [[dialog.trace_overlay]]: ele define o quanto o bitmap aparece sob as
  linhas, de 0 (só as linhas) a 100 (o bitmap como é), 80 no início.

Os botões depois da lista ajustam a imagem à janela, reduzem, ampliam e
deslocam. A roda do mouse amplia ao redor do ponteiro e um arrasto move a
imagem; os dois painéis se movem juntos. As partes transparentes mostram um
tabuleiro de xadrez.

## Configurações {#settings}

A guia [[dialog.trace_tab_settings]] contém:

- [[dialog.trace_type]] Contorno ou Linha central, e
  [[dialog.trace_image_type]] a predefinição. Escolher uma coloca os
  controles abaixo nos valores que convêm a esse tipo de imagem.
- [[dialog.detail]] (0 a 100): mais detalhe mantém áreas menores e linhas
  mais finas; menos detalhe suaviza manchas e granulação.
- [[dialog.smoothing]] (0 a 100, 25 no início): curvas mais suaves com
  menos nós.
- [[dialog.corner_smoothness]] (0 a 100): cantos mais arredondados.
- [[dialog.trace_remove]]: o que fica fora do resultado. **Cor de fundo**
  remove o fundo onde ele alcança a borda da imagem, **Cor inteira da
  imagem** remove cada área dessa cor, **Nenhum** vetoriza tudo.
  [[dialog.trace_remove_auto]] encontra a cor de fundo ao longo da borda da
  imagem; [[dialog.trace_remove_specify]] usa a cor da amostra, que você
  pode mudar ou pegar da visualização com o conta-gotas ao lado.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (ligado no
  início) faz uma só curva de cada área de uma cor; desligado, cada mancha
  separada é uma curva própria. [[dialog.trace_remove_overlap]] mantém cada
  forma nos próprios pixels; desligado, as formas avançam um pouco sob as
  de cima, para que não apareçam frestas entre vizinhas.
  [[dialog.trace_group_by_color]] põe as curvas de cada cor em um grupo
  próprio; fica disponível quando as áreas de uma cor ainda não estão
  mescladas.
- [[dialog.delete_original]]: o bitmap some quando o resultado é colocado.

As configurações que não se aplicam ao tipo de vetorização atual ficam
acinzentadas: uma vetorização de linha central lê só o detalhe e a
suavização.

## Cores {#colors}

A guia [[dialog.trace_tab_colors]] define o [[dialog.color_mode]] (RGB,
tons de cinza, ou preto e branco) e o [[dialog.trace_number_of_colors]] (2 a
256), e lista as cores do resultado com o número de curvas de cada uma.
Marque duas ou mais cores e clique em [[dialog.trace_merge_colors]] para
juntá-las na primeira; clique na amostra de uma cor para desenhar suas
curvas em outra cor. Mudar uma configuração que vetoriza a imagem de novo
esquece essas edições, que pertenciam ao resultado anterior.

## Redefinir, desfazer e refazer {#steps}

Abaixo do painel de configurações, [[dialog.reset]] devolve as
configurações da predefinição, e as duas setas voltam e avançam por cada
mudança feita na caixa de diálogo: cada controle solto, escolha em lista,
caixa de seleção, edição de cor e redefinição é um passo. Uma mudança feita
depois de desfazer descarta os passos desfeitos.

[[dialog.trace_options]] abre a página [[options.bitmap_tracing]] das
opções globais, onde você escolhe a predefinição da vetorização rápida e as
escolhas com que cada vetorização começa, e depois volta à caixa de
diálogo. O botão [[dialog.help_button]] abre esta página.
