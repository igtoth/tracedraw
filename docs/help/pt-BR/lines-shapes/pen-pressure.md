# Desenhar com uma mesa digitalizadora

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

A caneta de uma mesa digitalizadora move o ponteiro e clica como um mouse,
então toda ferramenta funciona com ela. Onde o sistema também informa ao
TraceDraw a força com que a caneta pressiona, as ferramentas Mão livre e
Pinceladas deixam essa pressão definir a largura do traço: pressione de leve
para uma linha fina, com mais força para uma larga. O mouse desenha com
largura uniforme, como antes.

## Desenhar com pressão {#draw}

1. Escolha a ferramenta [[tool.freehand]], ou a ferramenta
   [[tool.brush_strokes]] no modo Predefinido, Pincel ou Expressão.
2. Desenhe com a caneta, pressionando mais onde o traço deve ser mais largo.

- **Mão livre:** a nova linha ganha um contorno de largura variável que segue
  a pressão ao longo dela. Um toque levíssimo dá uma linha finíssima, a
  pressão total a largura total de contorno das linhas novas. Você pode editar
  a linha com a ferramenta Forma e o contorno dela no painel
  [[docker.properties]], como qualquer linha de largura variável (veja
  [Contornos](outlines.md#variable-outline)).
- **Pinceladas:** a caneta ajusta a largura dos traços Predefinido, Pincel e
  Expressão. A configuração [[docker.pressure]] do painel Pinceladas (0,5 de
  início) diz quanto a largura segue a caneta: em 0,5 um traço sem pressão
  tem metade da largura de um com pressão total; em 1 vai de nada à largura
  total. Predefinido e Pincel mantêm o formato ao longo do traço (um
  afinamento, uma onda) e o ajustam pela caneta; Expressão usa a pressão da
  caneta no lugar da velocidade do desenho. Traços caligráficos e os objetos
  do pulverizador mantêm o tamanho.

O traço aparece como uma linha fina enquanto você desenha e ganha as larguras
quando você levanta a caneta. [[docker.apply]] a uma curva no painel
Pinceladas não tem caneta, então monta o traço sem pressão.

## Desligar a pressão {#option}

A pressão vem ligada. Para desenhar com a caneta em largura uniforme, desligue
[[options.pen_pressure]] em [[options.pen_tablet]] na página
[[options.curve_tools]] de
[[menu.tools > menu.tools.options > menu.tools.options_tools]].

## Quais sistemas informam a pressão {#systems}

| Sistema | A caneta move o ponteiro | Pressão |
|---|---|---|
| Windows 8 e posteriores | sim | sim |
| Linux com X11 | sim | sim |
| Linux com Wayland | não (veja abaixo) | não |
| macOS | sim | sim |
| Versão para navegador | sim | não |

Num ambiente Wayland (o padrão de vários sistemas Linux) a caneta ainda não
chega ao TraceDraw. Inicie o TraceDraw com `tracedraw --x11` num terminal: a
janela então abre pelo XWayland, onde a caneta move o ponteiro, clica e
informa a pressão.

> **Dica:** Para mais controle sobre o traço depois, desenhe com pressão e
> então ajuste os nós de largura com a ferramenta [[tool.variable_outline]].
