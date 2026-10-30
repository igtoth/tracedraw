# Suavizar, borrar e tornar áspero o contorno

Os pincéis de modelagem do submenu Forma remodelam o contorno de um objeto
onde você arrasta sobre ele, como uma ferramenta de escultor: [[tool.smooth]],
[[tool.smear]], [[tool.twirl]], [[tool.attract_repel]], [[tool.smudge]] e
[[tool.roughen]].

## Usar um pincel de modelagem {#using}

1. Selecione uma curva. Outros objetos são convertidos em curvas antes; sem
   nada selecionado, é usado o objeto sob o ponteiro.
2. Escolha a ferramenta e defina o **tamanho da ponta** (o raio do pincel, 10
   mm de início) e as outras configurações na barra de propriedades.
3. Arraste sobre o contorno.

O efeito é mais forte no centro da ponta e some na borda. Só as partes do
contorno ao alcance mudam; curvas abertas continuam abertas e as fechadas,
fechadas. Cada arraste é um passo de desfazer. As configurações são lembradas
para a próxima vez.

## Suavizar {#smooth}

A ferramenta [[tool.smooth]] remove tremores e nós a mais onde você arrasta.
**Intensidade** (1 a 100%, 20% de início) define a força.

## Borrar {#smear}

A ferramenta [[tool.smear]] (<kbd>W</kbd>) puxa o contorno junto com o
ponteiro, como se você borrasse tinta fresca. **Pressão** (1 a 100%, 100% de
início) define quanto ele se move.

## Redemoinho {#twirl}

A ferramenta [[tool.twirl]] gira o contorno em volta do ponteiro, formando um
redemoinho. **Velocidade** (1 a 100%, 50% de início) define a rapidez, e os
botões de direção escolhem anti-horário ou horário; mantenha <kbd>Alt</kbd> ou
<kbd>Shift</kbd> pressionado ao arrastar para girar ao contrário. Manter o
ponteiro parado continua girando.

## Atrair e repelir {#attract-repel}

A ferramenta [[tool.attract_repel]] puxa os nós dentro da ponta em direção ao
ponteiro, ou os empurra para longe. **Velocidade** (1 a 100%, 50% de início)
define a rapidez; escolha Atrair ou Repelir na barra de propriedades, ou
mantenha <kbd>Alt</kbd> ou <kbd>Shift</kbd> pressionado ao arrastar para
alternar. Manter o ponteiro parado continua o efeito.

## Esfumar {#smudge}

A ferramenta [[tool.smudge]] (<kbd>V</kbd>) suaviza um pouco o contorno e
depois o borra, então ele é puxado sem dobras. **Pressão** (1 a 100%, 100% de
início) define quanto.

## Tornar áspero {#roughen}

A ferramenta [[tool.roughen]] (<kbd>E</kbd>) acrescenta pontas serrilhadas ao
longo do contorno. **Frequência** (1 a 10 pontas a cada 10 mm, 5 de início)
define quantas, e **Tamanho da ponta** (2 mm de início) a altura. As pontas são
retas, então ficam afiadas.
