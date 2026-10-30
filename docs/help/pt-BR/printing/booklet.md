# Imprimir livretos

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Um livreto imprime duas páginas em cada lado de uma folha de papel, dispostas
para que as folhas dobradas se leiam em ordem: um cardápio, um programa, um
pequeno catálogo ou uma revista. O TraceDraw emparelha as páginas por você;
você escolhe como o livreto é encadernado.

## Imprimir um livreto {#print}

1. Ponha as páginas do desenho na ordem de leitura, a capa primeiro.
2. Escolha [[menu.file > menu.file.print]] (<kbd>Ctrl+P</kbd>) ou
   [[menu.file.print_preview]].
3. Na guia [[print.tab_layout]], escolha [[print.layout_booklet]] como
   [[print.imposition]].
4. Em [[print.binding]], clique em uma das duas imagens (veja abaixo).
5. Defina os campos da encadernação: [[print.signature_sheets]] para um
   livreto colado, [[print.creep]] para um grampeado, e o
   [[print.booklet_gap]] para os dois.
6. Confira o esboço e as contagens embaixo dele, e imprima nos dois lados do
   papel.

Para fazer um livreto de só algumas páginas, liste-as nas páginas a imprimir
da guia [[print.tab_general]] antes.

## Escolher a encadernação {#binding}

- [[print.saddle_stitch]] (o padrão): cada folha é dobrada dentro da anterior
  e o livreto é grampeado pela dobra. Com oito páginas, a folha de fora tem as
  páginas 8 e 1 de um lado e 2 e 7 do outro; a de dentro tem 6 e 3, depois 4
  e 5. Melhor para livretos finos.
- [[print.perfect_bound]]: as folhas vão em cadernos, pequenos grupos de
  [[print.signature_sheets]] (4 de início, de 1 a 32) dobrados juntos. Os
  cadernos são empilhados e colados na lombada. O último caderno leva as
  folhas que sobram. Com uma folha por caderno, as páginas formam pares 4 1,
  2 3, depois 8 5, 6 7. Melhor para livretos mais grossos.

## As configurações {#settings}

- [[print.signature_sheets]] (só colado): quantas folhas são dobradas juntas
  em cada caderno.
- [[print.creep]] (só grampeado, de 0 a 20 mm, 0 de início): quando folhas
  encaixadas são dobradas, as de dentro sobram na borda aberta e perdem mais
  no corte. O deslocamento move as páginas da folha mais de dentro essa
  distância em direção à lombada; as folhas do meio se movem na proporção e a
  de fora não se move. As marcas de corte se movem com as páginas.
- [[print.booklet_gap]] (de 0 a 100 mm, 0 de início): o espaço entre as duas
  páginas de uma página dupla, centralizado na dobra. Numa folha do tamanho
  das páginas, a folha cresce com o espaço.

## Páginas em branco e as contagens {#counts}

Um livreto sempre tem um múltiplo de quatro páginas, já que cada folha tem
quatro. Quando o desenho tem outro número, páginas em branco são acrescentadas
no fim. Embaixo do esboço, o resumo dá o número de páginas, as páginas em
branco acrescentadas e as folhas de papel impressas dos dois lados, e para um
livreto colado o número de cadernos.

Com [[dialog.crop_marks]] na guia [[print.tab_prepress]], um livreto também
ganha uma marca de dobra tracejada na lombada. Veja
[Imprimir um desenho](print.md#prepress) para as outras marcas.

> **Dica:** Imprima uma cópia primeiro e dobre-a para conferir a ordem e as
> margens antes de imprimir todas.
