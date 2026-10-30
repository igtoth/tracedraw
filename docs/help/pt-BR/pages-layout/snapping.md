# Encaixe

O encaixe puxa o ponteiro, e os objetos que você move, para pontos exatos: nós,
o meio de segmentos, o centro de formas, linhas-guia, a grade ou a página. Assim
fica fácil unir linhas e alinhar objetos sem dar zoom.

## Ativar e desativar o encaixe {#on-off}

- [[menu.view > menu.view.snap_to]], ou a lista Encaixar em da barra de
  ferramentas padrão, ativa e desativa o encaixe em [[menu.view.snap_pixels]],
  [[menu.view.snap_document_grid]] (<kbd>Ctrl+Y</kbd>),
  [[menu.view.snap_baseline_grid]], [[menu.view.snap_guidelines]],
  [[menu.view.snap_objects]] (<kbd>Alt+Z</kbd>) e [[menu.view.snap_page]]. De
  início, objetos, linhas-guia e a página estão ativados.
- [[menu.view.snap_off]] (<kbd>Alt+Q</kbd>) desativa todo o encaixe e o
  reativa.

## Onde os objetos se encaixam {#modes}

Com o encaixe em objetos ativado, estes pontos atraem o ponteiro dentro do raio
de encaixe (10 pixels de tela de início). Cada um mostra sua marca e, com as
dicas de tela ativadas, o nome.

| Modo | Pontos |
|---|---|
| Nó | os nós dos contornos, os cantos dos retângulos, as pontas dos polígonos |
| Interseção | onde dois contornos se cruzam |
| Ponto médio | o meio de cada segmento |
| Quadrante | o alto, a base, a esquerda e a direita das elipses |
| Tangente | onde uma linha que você desenha toca uma elipse ou curva |
| Perpendicular | onde uma linha que você desenha encontra um segmento em ângulo reto |
| Borda | o ponto mais próximo de um contorno |
| Centro | o centro de formas, bitmaps e tabelas |
| Linha de base do texto | a linha de base de uma linha de texto |

Os pontos têm preferência sobre as linhas-guia, as linhas-guia sobre as grades,
e as bordas vêm por último. Os modos, o raio e as marcas são definidos na página
[[options.snapping]] das opções; veja
[Opções do TraceDraw](../customize/options-app.md#snapping).

## Encaixe ao mover {#moving}

Quando você começa a mover uma seleção com a ferramenta [[tool.pick]] a partir
de um dos pontos dela (um nó, um ponto médio ou o centro), esse ponto é levado
até o ponto ou a borda do outro objeto. Senão, as bordas e o centro da caixa da
seleção se encaixam em linhas-guia, grades, na página e em outros objetos. As
alças se encaixam enquanto você redimensiona.

## Guias dinâmicas e de alinhamento {#dynamic-guides}

- [[menu.view > menu.view.dynamic_guides]] (<kbd>Alt+Shift+D</kbd>) mostra guias
  temporárias a 0, 45, 90 e 135 graus pelos pontos por onde você passa, para
  você desenhar ao longo delas.
- [[menu.view.alignment_guides]] (<kbd>Alt+Shift+A</kbd>) mostra guias quando o
  objeto que você move se alinha com outros.
