# Pinceladas e pincéis pictóricos

As pinceladas transformam uma linha que você desenha em uma forma preenchida
de largura variável, como a tinta de uma caneta ou de um pincel. Os pincéis
pictóricos pintam o contorno de um objeto com marcas de giz, tinta, aquarela e
outros materiais.

## Pinceladas {#brush-strokes}

A ferramenta [[tool.brush_strokes]] (<kbd>I</kbd>) desenha traços em um de
cinco modos. O traço é um objeto preenchido: recebe a cor de preenchimento
atual (preto quando não há) e nenhum contorno, e você o edita como qualquer
curva.

Para desenhar uma pincelada:

1. Escolha a ferramenta [[tool.brush_strokes]].
2. Escolha um modo e as configurações na barra de propriedades ou no painel
   [[docker.brush_strokes]].
3. Arraste na página.

Para transformar curvas existentes em pinceladas, selecione-as e clique em
[[docker.apply_to_curve]] no painel [[docker.brush_strokes]].

![O painel Pinceladas](../../img/lines-shapes-artistic-media-brush-strokes.png)

### Modos {#modes}

- **Caligráfico**: uma ponta chata da largura dada, segura no ângulo dado (45
  graus de início). O traço fica mais fino quando você se move ao longo da
  ponta e mais largo quando se move através dela.
- **Predefinido**: um traço cuja largura segue um perfil: afinar as duas
  pontas (de início), afinar o início, afinar o fim, bojo, onda ou plano. A
  largura é o ponto mais largo.
- **Pincel**: um perfil predefinido com borda áspera, de pincel seco.
- **Expressão**: a largura segue a sua velocidade como se fosse a pressão da
  caneta: traços lentos são largos. A configuração de pressão ajusta o efeito.
- **Pulverizador**: formas (círculos, quadrados, estrelas, corações ou folhas)
  colocadas ao longo da linha a cada espaçamento (8 mm de início), cada uma
  entre 70% e 100% da largura.

### Configurações {#brush-settings}

- **Largura**: 3 mm de início.
- **Ângulo**: o ângulo da ponta, para Caligráfico.
- **Suavização**: 0 a 100 (25 de início). Valores maiores eliminam tremores.
- **Pressão**: para Expressão (0,5 de início).
- **Espaçamento**: para Pulverizador.

## Pincel pictórico {#painterly-brush}

A ferramenta [[tool.painterly_brush]], no mesmo submenu, pinta uma linha feita
de marcas de pincel. A pincelada é uma curva comum sem preenchimento: a
ferramenta [[tool.shape]] a remodela, a cor do contorno a colore e as marcas
acompanham.

Para pintar um traço:

1. Escolha a ferramenta [[tool.painterly_brush]].
2. Escolha um pincel, um tamanho e uma transparência na barra de propriedades.
3. Arraste na página.

Para pintar o contorno de objetos existentes, clique neles para selecioná-los e
escolha um pincel: os contornos são pintados com ele. Mudar o tamanho ou a
transparência muda as pinceladas deles. **Limpar pincelada** devolve aos objetos
selecionados o contorno simples.

### Pincéis e configurações {#painterly-settings}

- **Pincel**: materiais secos (Giz, o primeiro, Carvão, Pastel macio),
  materiais úmidos (Aguada, Aquarela, Pincel seco) e texturas (Cerdas, Pelo,
  Spray, Salpicos, Nuvem, Fita de seda).
- **Tamanho do pincel**: 0,1 a 200 mm, 5 mm de início.
- **Transparência**: 0 a 100%, 20% de início.

A maioria dos pincéis afina nas duas pontas. As marcas são colocadas do mesmo
jeito toda vez que o traço é desenhado, exportado ou impresso. Com "Escalonar
com o objeto" ativado, as marcas crescem e diminuem com o objeto. Em
exportações SVG e PDF, as marcas viram formas preenchidas com sua
transparência.
