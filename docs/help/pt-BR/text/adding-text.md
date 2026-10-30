# Adicionar e editar texto

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

## Adicionar texto {#add}

1. Escolha a ferramenta [[tool.text]] (<kbd>F8</kbd>).
2. Para **texto artístico**, clique onde o texto começa e digite. Pressione
   <kbd>Enter</kbd> para começar uma nova linha.
3. Para **texto de parágrafo**, arraste uma moldura e depois digite. O texto
   quebra nas bordas da moldura.
4. Pressione <kbd>Esc</kbd> para terminar. Um texto vazio é removido.

O texto artístico novo tem 24 pontos e o de parágrafo 12 pontos, na primeira
fonte sem serifa encontrada. Mude os padrões formatando sem nada selecionado.

Para transformar um objeto existente em moldura de texto, clique com o botão
direito nele e escolha criar uma moldura de texto vazia; veja
[Molduras vazias](../objects/clip-frames.md#empty-frames).
[[menu.text > menu.text.insert_placeholder_text]] preenche as molduras
selecionadas com texto de exemplo, tanto quanto couber.

## Editar texto {#edit}

Clique em um texto com a ferramenta [[tool.text]], ou clique duas vezes nele com
a ferramenta [[tool.pick]], para editá-lo no lugar.
[[menu.text > menu.text.edit_text]] (<kbd>Ctrl+Shift+T</kbd>) abre uma caixa
com o texto inteiro para editar; **OK** o devolve ao desenho, e os
caracteres que você não mudou mantêm a formatação.

| Teclas | O que fazem |
|---|---|
| Digitar, <kbd>Ctrl+V</kbd> | inserem no cursor, substituindo a seleção |
| <kbd>Backspace</kbd>, <kbd>Delete</kbd> | excluem a seleção, senão o caractere antes ou depois |
| <kbd>Enter</kbd>, <kbd>Shift+Enter</kbd> | novo parágrafo, nova linha |
| Setas, com <kbd>Ctrl</kbd> | movem um caractere ou uma palavra; <kbd>Shift</kbd> seleciona |
| <kbd>Home</kbd>, <kbd>End</kbd>, com <kbd>Ctrl</kbd> | início ou fim da linha, ou do texto |
| <kbd>Ctrl+A</kbd> | seleciona tudo |
| <kbd>Ctrl+B</kbd>, <kbd>Ctrl+I</kbd>, <kbd>Ctrl+U</kbd> | negrito, itálico, sublinhado |
| <kbd>Ctrl+L</kbd>, <kbd>Ctrl+E</kbd>, <kbd>Ctrl+R</kbd> | alinha à esquerda, ao centro, à direita |
| <kbd>Ctrl+J</kbd>, <kbd>Ctrl+H</kbd>, <kbd>Ctrl+N</kbd> | justificado, justificado forçado, sem alinhamento |
| <kbd>Shift+F3</kbd> | muda maiúsculas e minúsculas (veja [Ferramentas de escrita](writing-tools.md#change-case)) |
| Clique duplo | seleciona uma palavra |

Um novo alinhamento mantém o texto artístico no lugar, no ponto de referência
dos campos X e Y da barra de propriedades; em um quadro, ele vale para os
parágrafos do cursor.

Texto em escritas da direita para a esquerda (árabe, hebraico) é disposto da
direita para a esquerda, também quando misturado com palavras da esquerda para
a direita, e as seleções sombreiam cada letra selecionada. Teclados e métodos
de entrada que enviam caracteres (como os de tâmil e outras escritas indianas
no Windows) digitam no texto e nos campos, e códigos <kbd>Alt</kbd> digitados
no teclado numérico dão o caractere uma vez só.

Tudo o que você digita em uma sessão de edição é um passo de desfazer.

## Formatar pela barra de propriedades {#property-bar}

Com a ferramenta [[tool.text]], ou com texto selecionado, a barra de
propriedades tem a lista de fontes, o tamanho (digite ou escolha na lista),
negrito, itálico, sublinhado, o alinhamento (esquerda, centro, direita,
justificado), marcadores e capitular para texto de parágrafo,
[[menu.text.text_docker]] (<kbd>Ctrl+T</kbd>) e [[menu.text.edit_text]]. O
botão ao lado do tamanho da fonte abre os controles dos eixos de uma fonte
variável (veja [Fontes variáveis](fonts.md#variable-fonts)).
Durante a edição, elas agem sobre os caracteres selecionados; senão, sobre os
textos selecionados inteiros. Sua escolha também passa a ser o padrão do
próximo texto.

A lista de fontes mostra cada família na própria fonte, com as usadas por
último no alto. Digite para filtrar a lista. Uma família com vários estilos tem
uma seta que os mostra. Veja [Fontes e glifos](fonts.md).

## O painel Texto {#text-docker}

O painel [[docker.text]] (<kbd>Ctrl+T</kbd>) tem todas as configurações de
texto, em três seções que você pode abrir e fechar: **Caractere**,
**Parágrafo** e **Moldura**. Os três botões no alto mostram o painel a partir
dessa seção.

O justificado completo estica as linhas em chinês e japonês até a borda do
quadro, pondo espaço entre os caracteres, e o texto de parágrafo nessas
escritas quebra as linhas pelas regras delas: pontuação como ，。、 e
parênteses de fechamento nunca começam uma linha, e parênteses de abertura
nunca a terminam. Letras com outra cor mantêm o kerning da fonte com as
vizinhas.

![O painel Texto](../../img/text-adding-text-text-docker.png)

- **Caractere**: a fonte, o estilo e o tamanho; o kerning de intervalo (-50 a
  200%) e o sublinhado; o preenchimento, o fundo e o contorno dos caracteres.
  Mais opções abaixo da seção: deslocamentos e ângulo dos caracteres,
  tachado e recursos OpenType.
- **Parágrafo**: o alinhamento (nenhum, esquerda, centro, direita, justificado
  e justificado forçado, que espalha também a última linha); o espaçamento
  entre linhas em porcentagem da altura do caractere, pontos ou porcentagem do
  tamanho da fonte; os recuos esquerdo, da primeira linha e direito; o espaço
  antes e depois dos parágrafos; o espaçamento entre caracteres e entre
  palavras. A engrenagem abre os hífens personalizados.
- **Moldura**, para texto de parágrafo: a cor de fundo da moldura, o
  alinhamento vertical (topo, meio, base), ajustar o texto à moldura, colunas
  (1 a 8) e o espaço entre elas, e links para as caixas de diálogo de
  marcadores, capitular e colunas, e a hifenização.

Toda alteração vale para os caracteres selecionados durante a edição, ou para
os textos selecionados, e é um passo de desfazer. Sem texto selecionado, as
configurações passam a ser as do novo texto. O botão no canto superior direito
dá ao novo texto as configurações do texto selecionado.

## Molduras de texto de parágrafo {#frames}

As molduras aparecem como linhas tracejadas. Uma moldura fica vermelha, com uma
pequena aba embaixo da borda inferior, quando o texto não cabe; o texto que não
cabe não é desenhado, impresso nem exportado. Para fazê-lo caber:

- arraste as alças da moldura para aumentá-la. O texto corre de novo; as letras
  mantêm o tamanho;
- escolha [[menu.text > menu.text.paragraph_text_frame > menu.text.frame_fit_text]]
  para redimensionar o texto até preencher a moldura;
- ligue a moldura a outra, para o texto continuar nela:
  [[menu.text.frame_link]]. [[menu.text.frame_unlink]] desfaz a ligação.
  Ou clique na pequena aba sob a borda de baixo do quadro e depois clique
  em outro quadro (nesta página ou em outra) ou clique ou arraste num lugar
  vazio para um quadro novo: o texto continua nele.

[[menu.text > menu.text.show_text_frames]] mostra ou oculta as molduras.

## Mover e girar caracteres {#characters}

Com a ferramenta [[tool.shape]], o texto mostra um nó em cada caractere:
arraste nós para mover caracteres, e arraste as setas nos cantos do texto para
mudar o espaçamento entre caracteres, palavras e linhas. Os deslocamentos e o
ângulo dos caracteres também podem ser digitados no painel [[docker.text]].
[[menu.text > menu.text.straighten_text]] devolve os caracteres à linha, e
[[menu.text.align_to_baseline]] (<kbd>Alt+F12</kbd>) remove os deslocamentos
verticais.

## Texto em um caminho {#text-on-path}

1. Selecione um texto e uma curva ou forma.
2. Escolha [[menu.text > menu.text.fit_text_to_path]].

Ou clique no contorno de um caminho com a ferramenta [[tool.text]] e digite. O
texto em uma forma fechada fica do lado de fora dela, centralizado no alto;
clicado na parte de baixo de uma elipse, ele começa ali.

Com texto em um caminho selecionado (também com a ferramenta [[tool.pick]]), a
barra de propriedades mostra primeiro os campos do caminho e depois os campos
de texto de sempre:

- A orientação das letras: giradas, inclinação vertical, inclinação
  horizontal ou em pé.
- A posição vertical: na linha de base, na ascendente, na descendente ou
  centralizada no caminho.
- A posição ao longo do caminho: no início, no centro ou no fim e, em um
  caminho fechado, também **Centro do topo** (centralizado no ponto mais alto,
  do lado de fora e em pé) e **Centro da base** (centralizado no ponto mais
  baixo, do lado de dentro e lido da esquerda para a direita, como a linha de
  baixo de um carimbo).
- A distância do caminho e o deslocamento ao longo dele. Uma nova distância
  mantém o texto centralizado onde estava, então uma linha na base de um
  círculo continua centralizada ao se afastar ou se aproximar.
- Os dois botões de espelhar, que põem o texto do outro lado do caminho ou o
  viram, e [[menu.text.straighten_text]], que o tira do caminho.

Arraste o texto para deslizá-lo pelo caminho, ou através do caminho para
afastá-lo dele.

## Outros comandos de texto {#commands}

- [[menu.text > menu.text.insert_formatting_code]]: caracteres especiais como
  [[menu.text.code_em_space]], [[menu.text.code_nonbreaking_space]],
  [[menu.text.code_optional_hyphen]] e [[menu.text.code_column_break]].
- [[menu.text.show_non_printing]] mostra espaços, tabulações e quebras.
- [[menu.text.make_web_compatible]] prepara o texto para páginas web.
- [[menu.text > menu.text.convert]] alterna entre texto artístico e de
  parágrafo.
- Converter texto em curvas ([[menu.object > menu.object.convert_to_curves]])
  mantém a aparência sem a fonte.
- Arquivos de texto podem ser importados; veja
  [Importar arquivos](../file-formats/import.md).
