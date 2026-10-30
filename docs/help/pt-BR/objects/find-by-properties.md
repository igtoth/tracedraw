# Localizar e substituir por propriedades

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

O modo [[find.properties]] do painel [[docker.find_replace]] localiza objetos
pela aparência e pelo tipo: todo objeto preenchido com um vermelho, todo
contorno de 0,2 mm, todo texto numa fonte que você não quer mais, todo bitmap.
Depois ele os seleciona, ou muda o preenchimento, o contorno ou a fonte de
todos de uma vez.

## Localizar objetos {#find}

1. Escolha [[menu.edit > menu.edit.find_and_replace]] (<kbd>Ctrl+F</kbd>) e
   clique em [[find.properties]].
2. Em [[find.look_in]], escolha [[find.this_page]] (o padrão) ou
   [[find.whole_drawing]] (todas as páginas, depois as camadas mestras).
3. Em [[find.find_objects_with]], marque o que procurar. Um objeto é
   encontrado quando tem tudo o que está marcado:
   - [[find.object_type]] (desligado; Retângulo de início): só objetos de um
     tipo: [[find.type.rectangle]], [[find.type.ellipse]],
     [[find.type.polygon]], [[find.type.curve]], [[find.type.artistic_text]],
     [[find.type.paragraph_text]], [[find.type.group]], [[find.type.bitmap]],
     [[find.type.clip_frame]], [[find.type.table]] ou [[find.type.symbol]].
   - [[find.fill]] (ligado; preto de início): objetos preenchidos com esta
     cor, como preenchimento uniforme ou num gradiente ou malha, ou
     [[find.none]] para objetos sem preenchimento.
   - [[find.outline]] (desligado): objetos cujo contorno tem esta cor, ou
     [[find.none]] para objetos sem contorno.
   - [[find.outline_width]] (desligado; 0,2 mm de início, na unidade do
     contorno): objetos cujo contorno tem esta largura.
   - [[find.font]] (desligado): texto que usa esta família de fonte, com
     qualquer mistura de maiúsculas e minúsculas.
4. Para procurar o que um objeto tem, selecione-o e clique em
   [[find.from_selection]]: a cor de preenchimento (ou nenhuma), a cor do
   contorno (ou nenhuma), a largura do contorno e a fonte preenchem os campos,
   marcados onde faz sentido (a largura e o tipo ficam desmarcados).
   [[find.reset]] limpa tudo de volta aos padrões.
5. Clique num botão:
   - [[find.previous]] ou [[dialog.find_next]] seleciona os objetos
     encontrados um de cada vez, voltando ao começo no fim. Com
     [[find.whole_drawing]], a página de cada objeto aparece.
   - [[dialog.find_all]] seleciona todos os objetos encontrados na página
     atual. A barra de status diz quantos foram encontrados ao todo e quantos
     nesta página.

O painel mostra o número de objetos encontrados.

## Substituir propriedades {#replace}

1. Localize os objetos como acima.
2. Em [[find.replace_with]], marque o que mudar e defina o novo valor:
   - [[find.fill]]: quando uma cor de preenchimento é procurada, essa cor é
     trocada pela nova onde for usada, em gradientes e malhas também; senão
     os objetos recebem um preenchimento uniforme da nova cor.
   - [[find.outline]]: quando uma cor de contorno é procurada, os contornos
     dessa cor recebem a nova; senão todos os contornos recebem. Objetos sem
     contorno continuam sem.
   - [[find.outline_width]]: todo contorno dos objetos encontrados recebe a
     nova largura.
   - [[find.font]]: quando uma fonte é procurada, o texto nessa fonte recebe a
     nova; senão todo o texto dos objetos encontrados recebe.
3. Clique em [[dialog.replace_all]].

Todas as mudanças se desfazem em um passo. A barra de status diz em quantos
objetos algo mudou. Objetos bloqueados, e objetos em camadas bloqueadas, são
encontrados mas ficam como estão.

## Como os objetos são comparados {#compare}

- Cores contam como iguais quando estão no mesmo modelo com cada valor a meio
  por cento um do outro, ou, em modelos diferentes, quando aparecem como a
  mesma cor na tela. Uma cor especial corresponde à mesma tinta no mesmo
  tom, e a cor de registro só a ela mesma.
- Larguras contam como iguais a meio milésimo de milímetro.
- Em grupos e molduras de recorte, os objetos de dentro são verificados em
  qualquer profundidade, e o grupo é encontrado quando um deles corresponde.
  Só com [[find.object_type]] marcado, conta o tipo do próprio grupo. Só
  objetos do nível de cima são selecionados, já que são eles que você pode
  selecionar.

Para localizar texto ou nomes, veja
[Localizar e substituir](find-replace.md).
