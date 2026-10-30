# Linhas de cota e balões

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

As linhas de cota mostram o tamanho das coisas em um desenho técnico: um
comprimento, uma distância horizontal ou vertical, um ângulo ou o comprimento
de um segmento. Os balões apontam para algo com uma linha de chamada e um
rótulo. Os dois são vivos: quando você os move, escalona ou gira, eles são
medidos e desenhados de novo.

## Colocar uma cota {#placing}

Toda cota é colocada com cliques, e a barra de propriedades diz onde clicar em
seguida. Enquanto o último ponto é esperado, a cota segue o ponteiro.
<kbd>Esc</kbd> recomeça. Os pontos se encaixam como no desenho, então encaixar
em nós ou objetos dá medidas exatas; veja [Encaixe](../pages-layout/snapping.md).
A nova cota fica selecionada.

### Paralela {#parallel}

A ferramenta [[tool.parallel_dimension]] mede a distância em linha reta entre
dois pontos, em qualquer ângulo:

1. Arraste do primeiro ponto ao segundo, ou clique nos dois.
2. Mova o ponteiro e clique onde vai a linha de cota.

### Horizontal ou vertical {#horizontal-vertical}

A ferramenta [[tool.hv_dimension]] mede só a distância horizontal ou a vertical
entre dois pontos, a que for maior. Coloque-a como uma cota paralela.

### Angular {#angular}

A ferramenta [[tool.angular_dimension]] mede um ângulo:

1. Clique no vértice do ângulo.
2. Clique em um ponto do primeiro lado, depois em um ponto do segundo lado.
3. Clique onde vai o arco.

O ângulo é medido no sentido anti-horário a partir do primeiro lado.

### Segmento {#segment}

A ferramenta [[tool.segment_dimension]] mede um segmento de um objeto: clique
no segmento para pegar suas duas pontas e depois clique onde vai a linha de
cota.

## Estilo da cota {#dimension-style}

A barra de propriedades mostra estas configurações com as ferramentas de cota e
quando a seleção é toda de cotas. As alterações valem para as cotas
selecionadas e para as próximas que você fizer.

- **Precisão**: 0 a 6 casas decimais (2 de início).
- **Unidades**: as unidades de desenho (de início) ou qualquer outra. Os
  comprimentos seguem a escala do desenho, então em 1:100 uma linha de 10 mm
  mostra 1000 mm; veja [Opções do documento](../pages-layout/document-options.md#rulers).
- **Mostrar unidades**: a unidade depois do número (o símbolo de grau nos
  ângulos).
- **Prefixo** e **Sufixo**: texto antes e depois do número.
- **Texto**: acima da linha (de início), na linha ou abaixo da linha.
- **Tamanho do rótulo**: 1 a 500 pontos (10 de início).
- **Fonte** do rótulo (a fonte do texto novo de início) e a **cor do rótulo**
  (preto).
- **Cor da linha** (preto) e **largura da linha** (linha finíssima de início,
  até 50 mm), para as linhas e as setas.
- **Setas**: seta (de início), seta aberta, ponto, traço ou nenhuma.
- **Pontas redondas**: terminações e junções redondas nas linhas (desligado).
- O menu **Opções**: o **espaço da linha de extensão** entre os pontos medidos
  e as linhas de extensão (0), quanto as linhas de extensão passam **além da
  linha** (2 mm), **texto horizontal** para cotas lineares (desligado),
  **unir linhas nos cantos** (desligado), que faz das linhas de extensão e da
  linha de cota (ou do arco de um ângulo) uma linha só que dobra o canto em
  vez de passar dele, e um **raio do canto** (0 a 100 mm) que arredonda esses
  cantos. O menu fica aberto enquanto você digita nele.

O rótulo sempre se lê da esquerda para a direita e fica em pé. Clique duas
vezes nele com a ferramenta [[tool.pick]] para editá-lo no lugar: um texto que
ainda contém o valor medido o mantém vivo (o texto antes e depois dele vira o
prefixo e o sufixo); um texto sem ele aparece como digitado até você apagá-lo.

## Cotas que acompanham objetos {#follow}

Uma cota cujas pontas você colocou em um objeto fica presa a ele: nos nós (os
cantos, por exemplo) ou nas bordas (os lados de um retângulo, não só os
cantos). Quando você move, redimensiona, gira ou remodela o objeto, a cota
vai junto, cada ponta mantendo o lugar no nó ou ao longo da borda, e o valor
muda com ele. Mover só a cota a solta, assim como excluir o objeto ou o nó a
que ela estava presa.

Quando você move, dimensiona ou gira uma cota, ela é medida e desenhada de
novo: uma cota dimensionada por 2 mostra o dobro do comprimento, e o rótulo
fica em pé.

[[menu.object.break_apart]] (<kbd>Ctrl+K</kbd>) solta as linhas, as setas e
o rótulo como objetos próprios, e [[menu.object.convert_to_curves]]
(<kbd>Ctrl+Q</kbd>) faz da cota um grupo de curvas simples. Nos dois casos ela
deixa de ser uma cota.

## Balão {#callout}

A ferramenta [[tool.callout]] desenha um rótulo com uma linha de chamada em
dois segmentos.

1. Arraste da ponta (o que o balão aponta) até o cotovelo, ou clique nos dois.
2. Clique onde vai o texto. O texto fica desse lado do cotovelo.
3. Digite o texto. O texto de exemplo fica selecionado, então digitar o
   substitui.

A linha de chamada sempre termina no texto: quando você digita mais, muda o
tamanho ou move só o texto (<kbd>Ctrl</kbd>+clique nele), a linha acompanha.
Mova a ponta ou o cotovelo com a ferramenta [[tool.shape]].

A barra de propriedades, mostrada com a ferramenta e quando a seleção é toda de
balões, define:

- a **largura do contorno** da linha e da forma (finíssima de início);
- o **estilo de linha**;
- a **ponta** (uma seta de início, ou nenhuma);
- a **forma do balão** em volta do texto: nenhuma (de início), sublinhado,
  retângulo, retângulo arredondado ou elipse;
- o **espaço** entre o texto e a linha ou a forma (1 mm de início).
