# Desenhar linhas e curvas

No TraceDraw, uma linha é uma **curva**: um caminho feito de **segmentos** que
se unem em **nós**. Os segmentos podem ser retos ou curvos. Uma curva pode ser
aberta, com duas pontas, ou fechada, quando o fim encontra o início, e então
pode ser preenchida.

As ferramentas de curva ficam no submenu Curva da caixa de ferramentas. Todas
desenham uma curva com o preenchimento e o contorno padrão dos novos objetos;
mude-os depois no painel [[docker.properties]] ou com a paleta de cores.

## Mão livre {#freehand}

A ferramenta [[tool.freehand]] (<kbd>F5</kbd>) desenha uma linha por onde você
arrasta, como um lápis.

1. Escolha a ferramenta [[tool.freehand]].
2. Arraste na página. Solte o botão para terminar.

O TraceDraw segue o ponteiro e suaviza a linha quando você solta. A suavização
(0 a 100, 50 de início) na barra de propriedades define o quanto: um valor maior
dá menos nós e uma linha mais suave. Com o zoom aproximado, a linha guarda mais
detalhes.

Para desenhar uma linha reta com esta ferramenta, clique onde a linha começa e
clique de novo onde ela termina. Mantenha <kbd>Ctrl</kbd> pressionado para
limitar a linha a passos de 15 graus. <kbd>Esc</kbd> descarta o primeiro
clique.

## Linha de 2 pontos {#two-point-line}

A ferramenta [[tool.two_point_line]] desenha um segmento reto: pressione onde
ele começa e solte onde termina, ou clique duas vezes. Mantenha
<kbd>Ctrl</kbd> pressionado para limitá-lo a passos de 15 graus.

## Bézier {#bezier}

A ferramenta [[tool.bezier]] desenha curvas precisas, nó por nó.

1. Escolha a ferramenta [[tool.bezier]].
2. Clique para colocar um nó de canto (cúspide), ou arraste para colocar um nó
   suave e puxar suas alças, que definem a direção e a curvatura.
3. Continue colocando nós.
4. Clique duas vezes ou pressione <kbd>Enter</kbd> para terminar. Para fechar a
   curva, clique no primeiro nó. <kbd>Esc</kbd> descarta a curva que você está
   desenhando.

Mantenha <kbd>Ctrl</kbd> pressionado para colocar o próximo nó em passos de 15
graus a partir do último, ou para girar uma alça nesses passos. Cada nó se
encaixa no que o encaixe estiver configurado; veja
[Encaixe](../pages-layout/snapping.md).

## Caneta {#pen}

A ferramenta [[tool.pen]] coloca nós como a ferramenta [[tool.bezier]]: clique
para um nó de canto, arraste para um nó suave, clique duas vezes ou
<kbd>Enter</kbd> para terminar, clique no primeiro nó para fechar. Enquanto o
botão está pressionado, a alça segue o ponteiro, para você ver a curva tomar
forma.

## B-Spline {#b-spline}

A ferramenta [[tool.bspline]] desenha curvas suaves a partir de pontos de
controle, sem alças.

1. Clique para colocar cada ponto de controle. A curva segue os pontos,
   começando no primeiro e terminando no último, e sempre fica dentro deles.
   Enquanto você desenha, os pontos de controle aparecem como uma linha
   tracejada.
2. Clique duas vezes ou pressione <kbd>Enter</kbd> para terminar. Clique no
   primeiro ponto (quando houver três) para fechar a curva em um laço suave.

O resultado é uma curva comum; os pontos de controle não são guardados.

## Polilinha {#polyline}

A ferramenta [[tool.polyline]] desenha segmentos retos um após o outro: clique
para acrescentar cada canto e clique duas vezes ou pressione <kbd>Enter</kbd>
para terminar. Mantenha <kbd>Ctrl</kbd> pressionado para limitar cada segmento
a passos de 15 graus.

## Curva de 3 pontos {#three-point-curve}

A ferramenta [[tool.three_point_curve]] desenha um arco por três pontos:

1. Arraste de onde a curva começa até onde termina.
2. Mova o ponteiro para curvá-la e clique no ponto por onde ela passa.

## Continuar uma curva {#continue}

Para acrescentar a uma curva aberta, selecione-a e desenhe com a ferramenta
[[tool.freehand]], [[tool.two_point_line]], [[tool.bezier]], [[tool.pen]],
[[tool.bspline]] ou [[tool.polyline]] começando (ou terminando) em um dos nós
das pontas. A nova linha passa a fazer parte da curva:

- ela continua a partir daquela ponta;
- quando termina na outra ponta da curva, a curva se fecha;
- quando termina na ponta de outra parte da mesma curva, as duas partes se
  unem.

O ponteiro mostra um nó com uma linha saindo dele quando você está sobre um nó
de ponta, e um anel quando um clique fecharia a curva. A curva mantém a
aparência e continua selecionada, e a alteração é um único passo de desfazer.
Curvas fechadas, vários objetos selecionados e objetos dentro de grupos nunca
são continuados. A distância que conta como "sobre o nó" é definida na página
[[options.curve_tools]] das opções de ferramentas; veja
[Opções das ferramentas](../customize/tool-options.md#curves).

## A barra de propriedades {#property-bar}

Com uma ferramenta de curva ativa, a barra de propriedades mostra a posição, o
tamanho, a escala, o ângulo e os botões de espelhar da seleção, a largura do
contorno, o estilo de linha e as setas de início e fim, e um botão que fecha
uma curva aberta. A ferramenta [[tool.freehand]] acrescenta a suavização. Sem
nada selecionado, ela lembra como terminar uma curva. Veja
[Contornos](outlines.md) para as configurações de contorno.

## Editar curvas depois {#edit}

Use a ferramenta [[tool.shape]] para mover, acrescentar e excluir nós, mudar o
tipo deles e curvar segmentos; veja
[Moldar curvas com a ferramenta Forma](shape-tool.md).
