# Localizar e substituir

O painel [[docker.find_replace]] (<kbd>Ctrl+F</kbd>) localiza texto no desenho,
e o substitui, localiza objetos pelo nome, ou por preenchimento, contorno,
largura do contorno, fonte ou tipo e os substitui. Abra-o com
[[menu.edit > menu.edit.find_and_replace]].

![O painel Localizar e substituir](../../img/objects-find-replace.png)

## Localizar e substituir texto {#text}

1. Escolha [[dialog.find_text]].
2. Digite o que procurar em [[dialog.find]], e o texto novo em
   [[dialog.replace_with]].
3. Marque [[dialog.match_case]] para distinguir maiúsculas de minúsculas, e
   [[dialog.whole_word]] para ignorar o texto quando ele é parte de uma palavra
   maior.
4. Clique em [[dialog.find_next]] para selecionar o próximo objeto de texto que
   o contém, ou em [[dialog.find_all]] para selecionar todos.
5. Clique em [[dialog.replace]] para substituir no texto selecionado, ou em
   [[dialog.replace_all]] para substituir em todo lugar.

O painel diz quantos foram encontrados.

## Localizar objetos {#objects}

Escolha [[dialog.find_objects]], digite parte de um nome, ou um tipo de objeto
como "Rectangle", e clique em [[dialog.find_next]] ou [[dialog.find_all]]. A
busca cobre a página atual. Para dar nome a objetos, veja
[Renomear objetos e camadas](objects-and-layers.md#rename).

## Localizar e substituir por propriedades {#properties}

1. Escolha [[find.properties]].
2. Em [[find.look_in]], escolha [[find.this_page]] ou [[find.whole_drawing]].
3. Em [[find.find_objects_with]], marque o que procurar: [[find.object_type]],
   [[find.fill]], [[find.outline]] (uma cor, ou [[find.none]]),
   [[find.outline_width]] ou [[find.font]]. Um objeto é encontrado quando tem
   tudo o que está marcado. [[find.from_selection]] pega esses valores do
   objeto selecionado, por exemplo para achar todo objeto preenchido com a cor
   dele.
4. Clique em [[find.previous]] ou [[dialog.find_next]] para selecionar os
   objetos um a um (em outras páginas também), ou em [[dialog.find_all]] para
   selecionar os desta página.
5. Para mudá-los, marque em [[find.replace_with]] a nova cor de preenchimento,
   cor de contorno, largura do contorno ou fonte, e clique em
   [[dialog.replace_all]]. Todas as mudanças se desfazem de uma vez.

Uma cor de preenchimento procurada é substituída onde for usada, em
gradientes também. Objetos bloqueados ficam como estão.
