# Fazer cópias com Duplicar e repetir

O painel [[docker.step_and_repeat]] faz várias cópias da seleção, cada uma
deslocada da mesma distância em relação à anterior: uma fileira de botões, uma
coluna de linhas, uma grade de peças. Abra-o com
[[menu.edit > menu.edit.step_and_repeat]] (<kbd>Ctrl+Shift+D</kbd>).

![O painel Duplicar e repetir](../../img/objects-step-and-repeat.png)

## Para fazer cópias {#copies}

1. Selecione os objetos a copiar.
2. Defina o [[docker.copies]] (1 a 999).
3. Em [[docker.horizontal_settings]], escolha como as cópias se espaçam na
   horizontal:
   - [[docker.offset]]: a distância de uma cópia à seguinte, medida a partir do
     mesmo ponto de cada uma;
   - [[docker.spacing]]: o espaço entre uma cópia e a seguinte, de modo que as
     cópias nunca se sobrepõem, seja qual for o tamanho.

   Depois digite a distância. 0 mantém as cópias alinhadas.
4. Faça o mesmo em [[docker.vertical_settings]].
5. Clique em [[docker.apply]].

Enquanto você trabalha no painel (o ponteiro está sobre ele, você está
digitando num dos campos ou seu último clique foi nele), as cópias que
[[docker.apply]] faria aparecem contornadas na página, tracejadas em azul, e
acompanham cada valor que você muda. Nada é adicionado até você clicar em
[[docker.apply]], e um [[menu.edit.undo]] tira todas as cópias de novo. Um
clique fora, ou fechar ou esconder o painel, tira os contornos. Muitas cópias
de um objeto detalhado são contornadas pelas suas caixas.

As cópias vão para a camada ativa. As distâncias ficam nas unidades de desenho.

Para copiar com uma rotação ou uma escala a cada passo, use o campo
[[dialog.copies]] do painel [[docker.transformations]]; veja
[Mover, escalonar, girar e inclinar](transforming.md#transformations-docker).
