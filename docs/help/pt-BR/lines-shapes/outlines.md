# Definir contornos

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

O contorno é a linha desenhada ao longo da borda de um objeto. Você define a
cor, a largura, o estilo, os cantos, as pontas e as setas na seção
[[docker.outline]] do painel [[docker.properties]].

## Abrir as configurações de contorno {#open}

- A ferramenta [[tool.outline_pen]] (<kbd>F12</kbd>) abre a caixa de diálogo
  [[outline_pen.title]] com todas as configurações do contorno: cor, largura e
  sua unidade, estilo, limite de esquadria, tracejados, cantos, terminações,
  posição, pontas de seta, caligrafia, [[docker.behind_fill]],
  [[docker.scale_with_object]], os nós de largura do contorno variável e
  [[outline_pen.overprint]]. Os objetos selecionados mostram cada mudança ao
  vivo enquanto a caixa de diálogo está aberta. **OK** guarda só o que você
  mudou, em todos os objetos selecionados, em um passo de desfazer;
  **Cancelar** ou <kbd>Esc</kbd> devolve os contornos. Sem nada selecionado,
  define o contorno dos objetos novos. A lista de larguras oferece as mesmas
  larguras redondas da barra de propriedades, na unidade do contorno (0,1,
  0,2, 0,25 mm e assim por diante).

![A caixa de diálogo Caneta de contorno](../../img/lines-shapes-outlines-outline-pen.png)

- Clique duas vezes na parte do contorno da barra de status.
- Clique com o botão direito em uma cor da paleta para colorir o contorno da
  seleção.

## Configurações da caneta de contorno {#outline-pen}

- **Largura**: 0,2 mm para novos objetos. [[docker.hairline]] desenha uma
  linha com um ponto de tela ou de impressora de largura em qualquer zoom.
- [[docker.outline_fill]]: o contorno pode ser pintado como um preenchimento:
  nenhum, cor uniforme, gradiente, padrão, textura ou malha. Veja
  [Preenchimentos](../color/fills.md). Linhas finíssimas e setas mantêm a cor
  simples do contorno.
- [[docker.caps]], as pontas das linhas abertas: [[docker.cap_butt]] (de
  início), [[docker.cap_round]] ou [[docker.cap_square]].
- [[docker.corners]]: [[docker.join_miter]] (de início),
  [[docker.join_round]] ou [[docker.join_bevel]]. O [[docker.miter_limit]] (45
  graus para contornos novos) chanfra cantos mais agudos que ele, então um
  contorno grosso em texto ou em curvas estreitas não cria pontas compridas.
- [[docker.dash]]: [[docker.dash_solid]], [[docker.dash_dashed]],
  [[docker.dash_dotted]] ou [[docker.dash_dash_dot]]. Os tracejados crescem com
  a largura.
- Setas, no [[docker.arrow_start]] e no [[docker.arrow_end]] das linhas
  abertas, preenchidas com a cor do contorno. Para criar a sua, desenhe-a,
  selecione-a e escolha
  [[menu.object > menu.object.create > menu.object.create_arrowhead]].
- O [[docker.nib_stretch]] (0,1 a 1) e o [[docker.nib_angle_lc]] fazem um
  contorno caligráfico: mais fino de um lado da ponta que do outro, como uma
  caneta chata. No painel [[docker.properties]] eles ficam com o
  [[docker.miter_limit]] sob o título [[docker.outline_advanced]], depois das
  configurações comuns, fechado de início a não ser que um deles esteja
  definido.
- [[docker.behind_fill]] desenha o contorno por baixo do preenchimento, então
  só a metade externa aparece.
- [[docker.scale_with_object]]: a largura cresce e diminui quando você
  redimensiona o objeto.

Para transformar um contorno em um objeto que pode ser preenchido, escolha
[[menu.object > menu.object.convert_outline_to_object]] (<kbd>Ctrl+Shift+Q</kbd>).
As configurações dos novos objetos são explicadas em
[Estilos de objeto e padrões](../templates-styles/object-styles.md#document-defaults).

## Cor do contorno {#outline-color}

A ferramenta [[tool.outline_color]] (<kbd>Shift+F12</kbd>) abre o painel
[[docker.color]] para escolher a cor do contorno da seleção. Veja
[O painel Cor](../color/color-docker.md).

## Contorno variável {#variable-outline}

A ferramenta [[tool.variable_outline]], no submenu Forma, torna um contorno mais
grosso ou mais fino ao longo do comprimento com nós de largura.

1. Selecione um objeto com contorno, como uma curva, um retângulo ou uma
   elipse. Um objeto sem contorno recebe um fino.
2. Escolha a ferramenta [[tool.variable_outline]].
3. Arraste através do contorno: um nó de largura é acrescentado onde você
   começa, e o contorno chega até o ponteiro dos dois lados. Mantenha
   <kbd>Shift</kbd> (ou <kbd>Alt</kbd>) pressionado ao começar a arrastar para
   mudar só o lado onde está o ponteiro. <kbd>Shift</kbd>+clique escolhe uma
   sequência de nós e <kbd>Ctrl</kbd>+clique acrescenta um; um arraste então
   muda todos os nós escolhidos.
4. Acrescente mais nós do mesmo jeito. Entre os nós, a largura muda suavemente.

Para mudar um nó, clique nele e digite a **Largura esquerda** e a **Largura
direita** na barra de propriedades (a esquerda fica à esquerda da direção do
caminho). **Excluir nó** ou <kbd>Delete</kbd> remove o nó selecionado, e
**Limpar contorno variável** devolve o contorno simples.
