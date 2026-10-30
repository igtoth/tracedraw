# Tabelas

Uma tabela é um objeto feito de linhas e colunas de células, cada uma com
texto e preenchimento. Desenhe tabelas com a ferramenta [[tool.table]] e
altere-as com o menu [[menu.table]].

## Criar uma tabela {#create-table}

- Escolha [[menu.table > menu.table.create_new_table]] (ou
  [[menu.object > menu.object.create > menu.object.create_table]]). Na caixa de
  diálogo [[dialog.create_table]], defina o [[dialog.table_rows]], o
  [[dialog.table_columns]] e a altura e a largura (100 por 75 mm de início), e
  clique em [[dialog.ok]]. A tabela é colocada no meio da página e
  selecionada.
- Ou escolha a ferramenta [[tool.table]] e arraste um retângulo: a tabela o
  preenche, com linhas e colunas iguais.

![A caixa de diálogo Criar nova tabela](../../img/objects-tables-create-table.png)

Na barra de propriedades da ferramenta [[tool.table]], defina o número de
linhas (3 de início) e de colunas (4 de início), o preenchimento das células
(nenhum de início) e a largura e a cor da borda. Com uma tabela selecionada, o
preenchimento e a borda mudam essa tabela; senão, valem para a próxima que você
desenhar.

## Digitar nas células {#typing}

Com a ferramenta [[tool.table]], clique em uma célula e digite. A edição
funciona como em qualquer texto: <kbd>Ctrl+B</kbd>, <kbd>Ctrl+I</kbd> e
<kbd>Ctrl+U</kbd> formatam os caracteres selecionados, ou a célula inteira.
<kbd>Enter</kbd> acrescenta uma linha, <kbd>Tab</kbd> e <kbd>Shift+Tab</kbd> vão
para a próxima célula e a anterior, e as setas passam para a célula vizinha na
borda do texto. Uma linha da tabela cresce quando o texto precisa de mais
altura. <kbd>Shift</kbd>+clique acrescenta células à seleção.

## Mudar linhas e colunas {#table-menu}

O menu [[menu.table]] age sobre a célula ativa e as células selecionadas:

- [[menu.table.insert]]: [[menu.table.insert_row_above]],
  [[menu.table.insert_row_below]], [[menu.table.insert_column_left]] e
  [[menu.table.insert_column_right]]. A nova linha ou coluna recebe o tamanho
  da vizinha.
- [[menu.table.select]]: [[menu.table.select_cell]],
  [[menu.table.select_row]], [[menu.table.select_column]] ou
  [[menu.table.select_table]].
- [[menu.table.delete]]: [[menu.table.delete_row]],
  [[menu.table.delete_column]] ou [[menu.table.delete_table]]. Uma tabela
  mantém pelo menos uma linha e uma coluna.
- [[menu.table.distribute]]: [[menu.table.distribute_rows]] ou
  [[menu.table.distribute_columns]], mantendo o tamanho da tabela.
- [[menu.table.merge_cells]] (<kbd>Ctrl+M</kbd>) transforma as células
  selecionadas em uma, com os textos em ordem; [[menu.table.unmerge_cells]] as
  separa de novo, mantendo o texto e o preenchimento na célula superior
  esquerda.
- [[menu.table.split_into_rows]] e [[menu.table.split_into_columns]] dividem a
  linha ou coluna ativa em duas metades.

## Converter texto e tabelas {#convert}

- [[menu.table.convert_text_to_table]]: cada parágrafo do texto selecionado
  vira uma linha da tabela. A caixa de diálogo pergunta o que separa as
  células: vírgulas, tabulações, nada (cada parágrafo é uma célula, em uma
  coluna) ou caracteres seus em Definido pelo usuário, e informa quantas
  linhas e colunas a tabela terá. Ela começa com tabulações quando o texto
  tem alguma, senão com vírgulas. Os espaços em volta do texto de cada
  célula ficam de fora, então "Ana, Bia, Caio" com vírgulas dá três
  células.
- [[menu.table.convert_table_to_text]]: as células são unidas por tabulações e
  as linhas por quebras de linha.

Os padrões da ferramenta ficam na página [[tool.table]] das opções de
ferramentas; veja [Opções das ferramentas](../customize/tool-options.md#table).
