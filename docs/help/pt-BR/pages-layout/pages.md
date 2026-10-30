# Trabalhar com páginas

## Mover-se entre páginas {#go-to-page}

- Clique em uma guia de página embaixo da janela de desenho, ou use o
  navegador de páginas ao lado delas (primeira, anterior, próxima e última
  página).
- Pressione <kbd>PgUp</kbd> ou <kbd>PgDn</kbd>.
- Escolha [[menu.layout > menu.layout.go_to_page]], digite o número da página na
  caixa de diálogo [[dialog.go_to_page]] e clique em [[dialog.ok]].

![A caixa de diálogo Ir para a página](../../img/pages-layout-pages-go-to-page.png)

## O painel Páginas {#pages-docker}

O painel [[docker.pages]] lista as páginas do desenho. Clique em uma página para
ir até ela. Os botões [[menu.layout.insert_page]], [[menu.layout.delete_page]]
e [[menu.layout.duplicate_page]] agem sobre a página atual, e
[[menu.view.page_sorter]] mostra todas as páginas em miniatura. Abra-o em
[[menu.window > menu.window.dockers]].

![O painel Páginas](../../img/pages-layout-pages-pages-docker.png)

## Inserir páginas {#insert-page}

1. Escolha [[menu.layout > menu.layout.insert_page]], ou clique em um dos botões
   de inserir do navegador de páginas (eles acrescentam de uma vez uma página do
   tamanho atual antes ou depois da página atual).
2. Na caixa de diálogo [[dialog.insert_page]], defina o
   [[dialog.number_of_pages]] e escolha [[dialog.before]] ou [[dialog.after]]
   da página atual.
3. Clique em [[dialog.ok]].

![A caixa de diálogo Inserir página](../../img/pages-layout-pages-insert-page.png)

## Excluir páginas {#delete-page}

[[menu.layout.delete_page]] pergunta quais páginas excluir: o número da página
em [[dialog.delete_page_from]] (de início, a página atual) e, para excluir um
intervalo, [[dialog.delete_through]] com a última página dele. Todas saem em
um passo só, que [[menu.edit.undo]] desfaz. Um desenho mantém pelo menos uma
página. [[page_tab.delete]] no menu de uma aba de página exclui essa página na
hora.

## Duplicar uma página {#duplicate-page}

1. Escolha [[menu.layout > menu.layout.duplicate_page]], ou clique com o
   botão direito na guia da página e escolha [[menu.layout.duplicate_page]].
2. Em [[dialog.insert_new_page]], escolha [[dialog.before_selected_page]]
   ou [[dialog.after_selected_page]].
3. Escolha [[dialog.copy_layers_only]] para uma página com as mesmas
   camadas e nada nelas, ou [[dialog.copy_layers_and_contents]] para
   copiar também os objetos.
4. Clique em [[dialog.ok]]. A cópia passa a ser a página atual;
   [[menu.edit > menu.edit.undo]] a remove.

O menu da guia da página também tem [[page_tab.insert_after]],
[[page_tab.insert_before]], [[page_tab.delete]] e
[[menu.layout.switch_orientation]].

## Renomear uma página {#rename-page}

Escolha [[menu.layout > menu.layout.rename_page]], ou clique duas vezes na guia
da página. Digite o [[dialog.page_name]] e clique em [[dialog.ok]].

![A caixa de diálogo Renomear página](../../img/pages-layout-pages-rename-page.png)

## Tamanho e orientação da página {#page-size}

[[menu.layout > menu.layout.page_size]] abre a caixa de diálogo
[[dialog.page_size]]:

1. Digite a [[dialog.width]] e a [[dialog.height]]; [[dialog.swap]] as troca.
2. Marque [[dialog.apply_all_pages]] para mudar todas as páginas, ou deixe
   desmarcado para mudar só a página atual.
3. Clique em [[dialog.ok]].

![A caixa de diálogo Tamanho da página](../../img/pages-layout-pages-page-size.png)

[[menu.layout.switch_orientation]] alterna a página entre retrato e paisagem.
Sem nada selecionado, a barra de propriedades da ferramenta [[tool.pick]]
também define o tamanho e a orientação da página. As configurações completas
da página estão em [Opções do documento](document-options.md#page-size).

## Números de página {#page-numbers}

[[menu.layout > menu.layout.insert_page_number]] coloca um número de página
[[menu.layout.page_number_active]], [[menu.layout.page_number_all]],
[[menu.layout.page_number_odd]] ou [[menu.layout.page_number_even]]. Os números
em camadas mestras mostram o número certo em cada página.
[[menu.object > menu.object.insert > menu.object.insert_page_number]] coloca um
no texto que você está editando.

[[menu.layout > menu.layout.page_number_settings]] define a aparência deles:

- [[dialog.start_at]]: o primeiro número;
- [[dialog.prefix]] e [[dialog.suffix]]: texto em volta do número, como
  "Página ";
- [[dialog.number_style]]: [[dialog.style_arabic]],
  [[dialog.style_roman_upper]], [[dialog.style_roman_lower]],
  [[dialog.style_alpha_upper]] ou [[dialog.style_alpha_lower]];
- [[dialog.page_number_position]]: [[dialog.pos_bottom_center]],
  [[dialog.pos_bottom_outer]], [[dialog.pos_top_center]] ou
  [[dialog.pos_top_outer]].

![A caixa de diálogo Configurações de número de página](../../img/pages-layout-pages-page-numbers.png)

## Fundo da página {#background}

[[menu.layout > menu.layout.page_background]] define a cor atrás de todas as
páginas; veja [Opções do documento](document-options.md#background).
