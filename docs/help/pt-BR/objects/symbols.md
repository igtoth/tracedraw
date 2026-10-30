# Símbolos

Um **símbolo** é um objeto que você define uma vez e coloca muitas vezes. Cada
cópia, chamada **instância**, é um vínculo com a definição: edite a definição e
todas as instâncias mudam. Os símbolos deixam os arquivos pequenos e as partes
repetidas iguais. Eles ficam listados no painel [[docker.symbols]]
(<kbd>Ctrl+F3</kbd>).

![O painel Símbolos](../../img/objects-symbols.png)

## Criar um símbolo {#create}

1. Selecione os objetos que formam o símbolo.
2. Clique em [[docker.new_symbol]] no painel [[docker.symbols]], ou escolha
   [[menu.object > menu.object.symbol > menu.object.symbol_new]] ou
   [[menu.object > menu.object.create > menu.object.create_symbol]].

A seleção passa a ser a primeira instância do novo símbolo.

## Colocar instâncias {#insert}

O painel lista os símbolos do desenho, cada um com o número de instâncias.
Clique em [[docker.insert]] ao lado de um símbolo para colocar uma nova
instância, ou escolha
[[menu.object > menu.object.symbol > menu.object.symbol_insert]]. As instâncias
podem ser movidas, escalonadas e giradas como qualquer objeto.

## Transformar instâncias em objetos de novo {#revert}

Selecione instâncias e clique em [[docker.revert_to_objects]], ou escolha
[[menu.object > menu.object.symbol > menu.object.symbol_revert]]. Elas viram
objetos comuns que não acompanham mais o símbolo.
