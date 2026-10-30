# Fontes e glifos

O TraceDraw usa as fontes instaladas no seu computador, as fontes que você
acrescenta no Gerenciador de fontes e as fontes que vêm com um desenho. Esta
página explica os painéis e a caixa de diálogo para trabalhar com fontes e
caracteres especiais.

## O painel Fontes {#fonts-docker}

O painel [[docker.fonts]] lista as famílias de fontes.

![O painel Fontes](../../img/text-fonts.png)

- Digite no campo de filtro para reduzir a lista.
- Em [[docker.fonts_in_document]] ficam as famílias que o desenho usa. Uma
  família que não está instalada aparece marcada como [[docker.missing]]; o
  texto dela aparece em uma fonte parecida e mantém o nome da fonte. Uma família
  que veio com o desenho aparece marcada como [[docker.embedded]].
- Clique em uma família para dá-la ao texto selecionado, ou para torná-la a
  fonte do novo texto.

## Fontes ausentes {#missing}

Quando um desenho usa fontes que não estão instaladas, uma mensagem informa os
nomes depois que ele abre (veja
[Avisos depois de abrir](../get-started/open-and-save.md#open-notes)). Instale
as fontes e o texto volta a aparecer nelas. Desenhos salvos como `.tdraw`, e
arquivos `.cdr` com fontes incorporadas, podem levar as fontes junto.

## O Amostrador de fontes {#font-sampler}

O painel [[docker.font_sampler]] ([[menu.text > menu.text.font_sampler]])
mostra uma linha de texto de exemplo em várias fontes, uma abaixo da outra,
para compará-las.

![O painel Amostrador de fontes](../../img/text-fonts-font-sampler.png)

- Digite o texto de exemplo no alto; em branco, é usada uma frase no idioma da
  interface.
- Defina o tamanho (10 a 96, 28 de início).
- Acrescente fontes digitando parte de um nome e clicando em um dos resultados
  (até 24 fontes); o X remove uma. De início, o painel lista a fonte de texto e
  as fontes do desenho.
- Clique em uma amostra para dar essa fonte ao texto selecionado.

## O painel Glifos {#glyphs}

O painel [[docker.glyphs]] ([[menu.text > menu.text.glyphs]],
<kbd>Ctrl+F11</kbd>) mostra todos os caracteres de uma fonte, para inserir
símbolos e caracteres especiais.

![O painel Glifos](../../img/text-fonts-glyphs.png)

1. Escolha a fonte.
2. Escolha um intervalo de caracteres: a fonte inteira, Latim básico,
   Suplemento Latin-1, Latim estendido, Grego, Cirílico, Hebraico, Árabe,
   pontuação, números, moedas, setas, operadores matemáticos ou símbolos.
3. Clique em um caractere. Ele é inserido no cursor do texto que você está
   editando, ou como um novo texto no meio da página.

Passe o ponteiro sobre um caractere para ver o nome e o código. Uma pequena seta
embaixo de um caractere indica que a fonte tem alternativas para ele (como
floreios ou versaletes); clique nela para vê-las. Os glifos usados por último
ficam no alto do painel.

Digite no campo de pesquisa acima da grade para encontrar caracteres: digite os
próprios caracteres (como `é`) ou um código Unicode como `U+20AC`, `0x20ac` ou
`20AC`. A linha acima da grade mostra o código, o caractere e o intervalo do
caractere sob o ponteiro.

## O Gerenciador de fontes {#font-manager}

[[menu.tools > menu.tools.font_manager]] abre o Gerenciador de fontes, uma
janela que organiza suas fontes.

![O Gerenciador de fontes](../../img/text-fonts-font-manager.png)

- **Bibliotecas** (no alto à esquerda): **Minhas fontes** (as fontes
  instaladas), as pastas que você acrescentou, **Favoritas** e suas coleções.
  Clique duas vezes em uma coleção para renomeá-la; clique com o botão direito
  para mais opções.
- **Filtros** (embaixo à esquerda): por situação (instaladas, não instaladas,
  fontes protegidas do sistema, duplicadas), direitos de incorporação,
  tecnologia (TrueType, OpenType, variável), peso, largura, estilo, intervalo de
  caracteres e recursos OpenType. **Limpar** remove os filtros.
- **Lado direito**: o texto de exemplo e as predefinições, um campo de busca
  (família, estilo ou nome PostScript) e as fontes como cartões, agrupadas por
  família. Cada cartão mostra uma amostra, a tecnologia e uma barra que fica
  verde quando a fonte está instalada. Clique para escolher uma fonte,
  <kbd>Ctrl</kbd>+clique para escolher várias.

Para usar o Gerenciador de fontes:

- **Acrescentar pasta** procura arquivos de fonte (TTF, OTF, TTC, OTC) em uma
  pasta; as fontes aparecem como não instaladas. **Acrescentar fonte** instala
  um arquivo de uma vez.
- **Instalar** torna as fontes escolhidas disponíveis no TraceDraw, agora e a
  cada início. **Desinstalar** as remove; as fontes do próprio sistema são
  protegidas.
- **Acrescentar à coleção** e **Remover da coleção** organizam as fontes;
  arraste fontes até Favoritas ou uma coleção para acrescentá-las.
- Clique duas vezes em uma fonte instalada, ou escolha **Usar no desenho**, para
  dá-la ao texto selecionado.
- **Mostrar detalhes** mostra a família, o estilo, a versão, o arquivo, a
  tecnologia, os direitos de incorporação, o peso, a largura, o número de
  glifos, as escritas e os recursos da fonte que você clicou.

As fontes que o desenho usa e que não estão instaladas aparecem em laranja
acima dos cartões. Na versão para navegador, **Acrescentar fonte** carrega um
arquivo de fonte só para a sessão.
