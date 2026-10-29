# Copiar cores e atributos com os conta-gotas

As ferramentas conta-gotas pegam uma cor, ou um conjunto inteiro de atributos,
de um objeto e o dão a outros. As duas trabalham em dois passos, mostrados na
barra de propriedades: **selecionar** (pegar uma amostra) e **aplicar** (pintar
com ela). Depois de uma amostra, a ferramenta passa para aplicar; clique no
botão de selecionar para amostrar de novo. <kbd>Esc</kbd> descarta a amostra e
volta para a ferramenta [[tool.pick]].

O ponteiro é uma pipeta enquanto a ferramenta amostra e um balde de tinta
enquanto aplica. Imagens dão a cor mesmo dentro de um grupo ou de uma moldura.

## Conta-gotas de cor {#color-eyedropper}

1. Escolha a ferramenta [[tool.color_eyedropper]].
2. Clique em um objeto para pegar a cor: a cor de um preenchimento uniforme, a
   primeira cor de um gradiente ou a cor do contorno. Em um bitmap, pega a cor
   dos pixels sob o ponteiro, na média do tamanho de amostra definido na barra
   de propriedades (1 por 1, 2 por 2 ou 5 por 5 pixels). <kbd>Shift</kbd>+clique
   pega primeiro a cor do contorno.
3. Clique em objetos para preenchê-los com essa cor; <kbd>Shift</kbd>+clique
   para colorir o contorno (um objeto sem contorno ganha um fino). Clicar dentro
   de uma forma fechada vazia a preenche.

A barra de propriedades mostra a cor amostrada.

Enquanto você move o ponteiro para pegar uma cor, uma pequena caixa ao lado
mostra a cor embaixo dele: uma amostra, o nome da cor quando é uma cor da
paleta ou uma tinta especial, seus valores no modo de cor do desenho (CMYK ou
RGB) e seu valor hexadecimal. O conta-gotas de um seletor de cores mostra a
mesma caixa.

## Conta-gotas de atributos {#attributes-eyedropper}

A ferramenta [[tool.attributes_eyedropper]] copia várias propriedades de uma vez
de um objeto para outros.

1. Escolha a ferramenta [[tool.attributes_eyedropper]].
2. Na barra de propriedades, escolha o que copiar:
   - **Propriedades**: contorno, preenchimento e texto (a fonte, o tamanho e o
     estilo; o texto em si é mantido). Todas ativadas de início.
   - **Transformações**: tamanho, rotação e posição. Desativadas de início.
   - **Efeitos**: transparência, sombra e efeitos vivos. Desativados de início.
3. Clique no objeto de onde copiar.
4. Clique em cada objeto que deve receber a cópia. <kbd>Shift</kbd>+clique
   amostra outro objeto.

Cada clique é um passo de desfazer.
