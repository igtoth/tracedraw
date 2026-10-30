# Preenchimentos

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Um preenchimento pinta o interior de um objeto fechado. O TraceDraw tem seis
tipos de preenchimento: [[docker.fill_none]], [[docker.fill_uniform]] (uma cor
lisa), [[docker.fill_fountain]] (cores que se misturam),
[[docker.fill_pattern]], [[docker.fill_texture]] e [[docker.fill_mesh]] (cores
que fluem entre os nós de uma grade).

## Formas de preencher um objeto {#ways}

- Clique em uma cor da paleta para dar à seleção um preenchimento uniforme.
- Pressione <kbd>F11</kbd> para a caixa de diálogo [[edit_fill.title]] (veja
  [abaixo](#edit-fill)), ou use a seção [[docker.fill]] do painel
  [[docker.properties]]: escolha o tipo de preenchimento na fileira de imagens
  e depois as configurações.
- Use a ferramenta [[tool.interactive_fill]] (<kbd>G</kbd>) para definir
  gradientes no próprio objeto.
- Use a ferramenta [[tool.area_fill]] para preencher uma área cercada por vários
  objetos.
- Use a ferramenta [[tool.mesh_fill]] (<kbd>M</kbd>) para um preenchimento de
  malha.

Sem nada selecionado, o preenchimento escolhido passa a ser o dos novos
objetos.

## Preenchimentos uniformes {#uniform}

Um preenchimento uniforme é uma cor, em qualquer modelo de cor. Escolha-a na
paleta, no painel [[docker.color]] (veja [O painel Cor](color-docker.md)) ou na
seção de preenchimento do painel [[docker.properties]].

## Preenchimentos em gradiente {#gradient}

Um gradiente mistura duas ou mais cores. As configurações, na seção de
preenchimento do painel [[docker.properties]]:

- O tipo: [[fill.linear]] (ao longo de uma linha), [[fill.radial]] (do centro
  para fora), [[fill.conical]] (em volta do centro), [[fill.square]] (em
  quadrados crescentes) ou [[fill.shaped]] (seguindo o contorno do objeto, da
  borda para dentro).
- [[docker.gradient_presets]]: gradientes prontos para começar.
- As **paradas**: as cores e onde ficam no gradiente, de 2 a 64. Selecione uma
  parada para mudar a cor ou a posição; [[docker.add_stop]] e
  [[docker.remove_stop]] acrescentam e removem paradas.
  [[docker.import_gradient]] pega as cores de um arquivo de gradiente `.ggr`.
- O **ângulo** dos gradientes linear, cônico e quadrado, o [[docker.edge_pad]]
  (quanto de cada ponta é cor lisa) e o deslocamento do centro dos gradientes
  radial, cônico e quadrado.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]] (as
  cores recomeçam do início) ou [[docker.repeat_mirror]] (vão e voltam), com o
  número de [[docker.repeat_count]].
- [[docker.bilinear]]: as cores vão da primeira à última e voltam.
- [[docker.invert_colours]]: as cores correm na ordem inversa.

Uma faixa abaixo do tipo mostra toda a sequência de cores como será desenhada.

## A caixa de diálogo Editar preenchimento {#edit-fill}

<kbd>F11</kbd> abre a caixa de diálogo [[edit_fill.title]] no preenchimento da
seleção (ou dos objetos novos, sem nada selecionado).

- A fileira de imagens no alto escolhe o tipo de preenchimento.
- Um preenchimento uniforme mostra o visualizador de cores.
- Um gradiente mostra à esquerda uma prévia do preenchimento e o
  [[edit_fill.fill_picker]] com os gradientes prontos. À direita: o tipo, como
  ele se repete e a faixa de cores com seus nós. Clique num nó para escolhê-lo
  e arraste para movê-lo; clique duas vezes na faixa para acrescentar um nó e
  num nó para removê-lo. A cor, a transparência, a posição e o
  [[edit_fill.midpoint]] do nó escolhido ficam sob a faixa, com
  [[edit_fill.reverse]]. [[edit_fill.transform]] tem a rotação, o centro (para
  gradientes radiais, cônicos e quadrados), a margem da borda, o espelho e a
  inversão. A prévia tem as proporções do objeto e mostra os nós de início, de
  fim e de inclinação: arraste-os para mover, girar e esticar o gradiente como
  na tela (<kbd>Ctrl</kbd> gira em passos de 15 graus).
- **OK** muda só o que você mudou em todos os objetos selecionados, com
  [[edit_fill.overprint]].
- Enquanto a caixa de diálogo está aberta, a seleção mostra o preenchimento conforme você o altera; **Cancelar** ou <kbd>Esc</kbd> devolve o preenchimento anterior.

![A caixa de diálogo Editar preenchimento](../../img/color-fills-edit-fill.png)

## A ferramenta Preenchimento interativo {#interactive-fill}

A ferramenta [[tool.interactive_fill]] (<kbd>G</kbd>) edita preenchimentos no
próprio objeto.

1. Escolha a ferramenta e arraste através de um objeto: o gradiente começa
   onde você pressiona e termina onde solta, mantendo as cores de um
   gradiente que o objeto já tenha. Mantenha <kbd>Ctrl</kbd> pressionado para
   limitar o ângulo a passos de 15 graus. Um clique num objeto sem gradiente
   dá o preenchimento padrão.
2. Arraste os nós para mudar o gradiente: o nó inicial o move, o nó final
   define o comprimento e a direção, e o nó redondo de inclinação inclina as
   faixas (ou faz de um gradiente radial uma elipse).
3. Clique duas vezes na linha entre os nós para acrescentar um nó de cor ali;
   clique duas vezes num nó de cor para removê-lo. Arraste um nó de cor ao
   longo da linha, ou a barrinha entre dois nós para mover o ponto onde as
   cores deles se encontram no meio.
4. Clique num nó para escolhê-lo: abre-se embaixo dele uma pequena barra com
   a cor do nó (clique na amostra para o seletor de cores) e a transparência.
   Uma cor da paleta clicada com um nó escolhido vai para esse nó, e a barra
   de propriedades mostra a cor, a transparência e a posição do nó.
5. Arraste uma cor da paleta até um nó para mudar a cor dele, ou até a linha
   entre os nós para acrescentar uma cor ali.

A barra de propriedades tem o tipo de preenchimento e as configurações: a cor
de um preenchimento uniforme; para um gradiente, o tipo, as cores inicial e
final, o ângulo e a margem.

## A ferramenta Preenchimento de área {#area-fill}

A ferramenta [[tool.area_fill]] preenche qualquer área fechada, mesmo uma
formada por vários objetos sobrepostos, criando um novo objeto com essa área. Os
objetos em volta ficam como estão.

1. Escolha a ferramenta [[tool.area_fill]].
2. Escolha o preenchimento e o contorno na barra de propriedades:
   **Especificar** (a cor, e para o contorno a largura, ao lado), **Usar
   padrão** (o preenchimento e o contorno dos novos objetos; uma cor quando o
   padrão é sem preenchimento, para a área aparecer) ou **Nenhum**.
3. Clique dentro da área.

O novo objeto vai para o topo da camada ativa e fica selecionado. Um clique em
um contorno, ou em uma área que nada cerca, não cria nada.

## Preenchimentos de malha {#mesh-fill}

Um preenchimento de malha é uma grade de nós sobre um objeto, cada nó com cor e
transparência próprias; as cores fluem suavemente entre os nós. Serve para
sombreamentos realistas.

1. Escolha a ferramenta [[tool.mesh_fill]] (<kbd>M</kbd>) e defina as linhas e
   colunas na barra de propriedades (2 por 2 de início, até 50).
2. Clique em um objeto. Ele recebe uma malha na cor do preenchimento.
3. Clique em um nó para selecioná-lo e depois em uma cor da paleta para
   colori-lo. Arraste nós para curvar o fluxo das cores.
4. Clique duas vezes dentro da malha para acrescentar um nó exatamente ali,
   numa nova linha e coluna por esse ponto; selecione um nó e pressione <kbd>Delete</kbd> para remover
   a linha e a coluna dele.

A barra de propriedades também tem **Cor suave**, que mistura as cores com mais
suavidade, a **transparência** do nó selecionado e **Limpar malha**, que remove
o preenchimento de malha. Mudar as linhas e colunas de uma malha mantém as
cores.

## Preenchimentos de padrão e textura {#pattern-texture}

- Um preenchimento [[docker.fill_pattern]] repete um ladrilho: duas cores, um
  padrão vetorial ou um bitmap. Veja [Preenchimentos de padrão](patterns.md).
- Um preenchimento [[docker.fill_texture]] é uma textura gerada, de aparência
  natural: [[texture.clouds]], [[texture.marble]], [[texture.noise]] ou
  [[texture.wood]], com cores e escala.

## Vários preenchimentos em um objeto {#several-fills}

Na seção de preenchimento do painel [[docker.properties]], [[docker.add_fill]]
pinta outro preenchimento sobre o do objeto, começando como uma cópia para
você mudar. Os preenchimentos extras aparecem como "Preenchimento 2",
"Preenchimento 3" e assim por diante, cada um com seu editor e um botão para
removê-lo. A seção de contorno pode acrescentar contornos extras da mesma forma.

Os preenchimentos também podem ser pintados em contornos; veja
[Definir contornos](../lines-shapes/outlines.md#outline-pen).
