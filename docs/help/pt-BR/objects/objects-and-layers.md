# Organizar objetos e camadas

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

O painel [[docker.objects]] mostra como a página atual está organizada: as
camadas, os objetos de cada camada, os grupos e as camadas mestras que aparecem
em todas as páginas. Use-o para selecionar, nomear, ocultar, bloquear e ordenar
objetos e camadas. Objetos mais acima na lista são desenhados na frente.

![O painel Objetos](../../img/objects-objects-and-layers.png)

## O painel Objetos {#objects-docker}

O painel lista a página com as camadas, a de cima primeiro, depois a
[[docker.master_page]] com as camadas mestras, as linhas-guia
([[docker.guides]]) e a [[menu.view.document_grid]]. Cada objeto mostra uma
miniatura, o próprio objeto desenhado em tamanho pequeno com o preenchimento e
o contorno, e o nome, ou o que ele é quando não tem nome ("Retângulo", "Curva
com 8 nós").

- Clique em um objeto para selecioná-lo; <kbd>Shift</kbd>+clique o acrescenta à
  seleção. A camada dele passa a ser a camada ativa.
- Clique na seta de um grupo para ver os objetos dentro dele.
- Cada camada tem três botões: [[docker.show_hide]], [[docker.print_layer]] e
  [[docker.lock_layer]]. Uma camada oculta ou bloqueada não pode ser editada. O
  cadeado de [[docker.guides]] bloqueia todas as linhas-guia da página.
- Objetos ocultos mostram o nome e a miniatura esmaecidos.
- Para aumentar ou diminuir as miniaturas, arraste o controle deslizante entre
  o quadrado pequeno e o grande na parte de baixo do painel, gire a roda do
  mouse sobre ele ou segure <kbd>Ctrl</kbd> e gire a roda sobre a lista. O
  tamanho é lembrado. Uma miniatura só é desenhada de novo quando o objeto
  muda, então listas longas continuam rápidas.

## Camadas {#layers}

Uma camada é uma folha onde ficam os objetos. A **camada ativa**, em negrito e
com uma barra colorida, é para onde vão os novos objetos quando você desenha,
cola, duplica ou importa. Clique em uma camada para torná-la ativa.

- [[docker.new_layer]], embaixo do painel, acrescenta uma camada no topo e a
  torna ativa.
- Clique com o botão direito em uma camada para [[docker.rename]],
  [[docker.move_up]], [[docker.move_down]] e [[docker.delete_layer]]. Uma
  página sempre mantém uma camada.
- [[docker.delete]], embaixo do painel, exclui os objetos selecionados, ou a
  camada ativa quando nada está selecionado.

### Camadas mestras {#master-layers}

Os objetos de uma **camada mestra** aparecem em todas as páginas, por baixo das
camadas da própria página: use-as para um logotipo, um cabeçalho ou números de
página. Clique no botão [[docker.new_master_layer]] para acrescentar uma para
todas as páginas; clique com o botão direito nele para acrescentar uma só para
páginas ímpares ou pares. O menu de uma camada mestra tem [[docker.show_on]]
para escolher [[options.scope_all]], [[options.scope_odd]] ou
[[options.scope_even]]. Você pode selecionar e editar objetos mestres a partir
de qualquer página em que eles apareçam.

Para pôr uma camada mestra sobre os objetos da própria página, marque
[[docker.master_in_front]] no menu dela, ou selecione um dos objetos dela e
escolha [[menu.object > menu.object.order > menu.object.to_front_of_page]]
(<kbd>Ctrl</kbd>+<kbd>Home</kbd>); [[menu.object.to_back_of_page]]
(<kbd>Ctrl</kbd>+<kbd>End</kbd>) põe a camada de novo por baixo dos objetos da
página.

## Renomear objetos e camadas {#rename}

Clique duas vezes em uma camada ou objeto no painel, ou escolha
[[docker.rename]] no menu dele, para abrir a caixa de diálogo
[[dialog.rename_layer]] ou [[dialog.rename_object]]. Digite o novo nome e
clique em [[dialog.ok]]. Um objeto com nome vazio volta a mostrar o que ele é.
Clique duas vezes na linha da página para renomear a página.

![A caixa de diálogo Renomear objeto](../../img/objects-objects-and-layers-rename.png)

Os nomes ajudam a encontrar objetos no painel e em
[Localizar e substituir](find-replace.md), e são mantidos nos arquivos SVG
exportados.

## Mudar a ordem {#order}

[[menu.object > menu.object.order]] move os objetos selecionados:
[[menu.object.to_front_of_page]], [[menu.object.to_back_of_page]],
[[menu.object.to_front_of_layer]], [[menu.object.to_back_of_layer]],
[[menu.object.forward_one]], [[menu.object.back_one]],
[[menu.object.in_front_of]], [[menu.object.behind]] (clique no objeto à frente
ou atrás do qual eles devem ir) e [[menu.object.reverse_order]].

## Agrupar e combinar {#combine}

- [[menu.object > menu.object.group > menu.object.group_group]]
  (<kbd>Ctrl+G</kbd>) transforma a seleção em um grupo que se move e se
  redimensiona junto. [[menu.object.group_ungroup]] (<kbd>Ctrl+U</kbd>) o
  desfaz, e [[menu.object.group_ungroup_all]] também desfaz os grupos dentro
  dele. Para editar um objeto dentro de um grupo sem desagrupar,
  <kbd>Ctrl</kbd>+clique nele.
- [[menu.object.combine]] (<kbd>Ctrl+L</kbd>) transforma os objetos
  selecionados em uma única curva de várias partes, com um preenchimento e um
  contorno; onde as partes se sobrepõem, surgem furos.
  [[menu.object.break_apart]] (<kbd>Ctrl+K</kbd>) separa uma curva em suas
  partes.
- [[menu.object.convert_to_curves]] (<kbd>Ctrl+Q</kbd>) transforma um
  retângulo, elipse, polígono ou texto em uma curva editável nó por nó.

## Bloquear e ocultar objetos {#lock-hide}

- [[menu.object > menu.object.lock > menu.object.lock_object]] impede que os
  objetos selecionados sejam selecionados ou alterados;
  [[menu.object.unlock_object]] e [[menu.object.unlock_all]] os liberam.
- [[menu.object > menu.object.hide > menu.object.hide_object]] oculta os objetos
  selecionados; [[menu.object.show_all]] os mostra de novo.
- No painel [[docker.objects]], a linha de cada objeto tem o próprio olho e
  cadeado, para ocultar ou mostrar e bloquear ou desbloquear o objeto pela
  lista, mesmo um objeto bloqueado.

Selecionar um objeto na página rola a lista até a linha dele, então um objeto
lá embaixo numa lista longa aparece.

Para tirar objetos do caminho enquanto trabalha em outros, veja também o
[Modo de foco](focus-mode.md).
