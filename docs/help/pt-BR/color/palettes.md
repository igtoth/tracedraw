# Paletas de cores

A paleta de cores embaixo da janela é o jeito mais rápido de colorir objetos.
Abaixo dela, a **paleta do documento** reúne as cores que o desenho usa. Você
pode abrir mais paletas, criar as suas e editá-las.

## Usar a paleta {#using}

| Ação | Resultado |
|---|---|
| Clicar em uma cor | preenche a seleção (sem nada selecionado, define o preenchimento dos novos objetos) |
| Clicar com o botão direito em uma cor | colore o contorno da seleção |
| <kbd>Ctrl</kbd>+clicar em uma cor | mistura um décimo dela no preenchimento da seleção |
| Clicar e segurar em uma cor | mostra tons mais claros, mais escuros, mais apagados e mais vivos para escolher |
| Arrastar uma cor até um objeto | preenche o objeto, ou colore o contorno quando solta na borda |
| Clicar na amostra riscada | remove o preenchimento (botão direito: o contorno) |

As setas nas pontas rolam a paleta, e o botão na ponta direita mostra todas as
cores de uma vez. Passe o ponteiro sobre uma cor para ver o nome e os valores. O
botão de menu da paleta, na ponta esquerda, abre o menu da paleta.

A paleta pode ir para outra borda da janela, ou flutuar: arraste-a pela alça
pontilhada.

## A paleta do documento {#document-palette}

Toda cor aplicada a um objeto entra na paleta do documento, que é salva com o
desenho. O conta-gotas dela acrescenta a cor sob o ponteiro, e o menu tem
**Atualizar automaticamente** (para parar de acrescentar cores sozinha),
**Acrescentar da seleção**, **Acrescentar do documento**, **Excluir cor** e
**Redefinir paleta** (mantém só as cores que o desenho ainda usa). Você também
pode arrastar cores ou objetos até ela. Para mostrá-la ou ocultá-la, veja
[Opções de personalização](../customize/customization.md#color-palette).

## O painel Paletas {#palettes-docker}

O painel [[docker.palettes]] lista as paletas que você tem, com o número de
cores de cada uma. Abra-o com
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![O painel Paletas](../../img/color-palettes.png)

- Marque uma paleta para mostrá-la abaixo da janela de desenho.
- [[docker.edit]] a abre no editor de paletas.
- [[menu.window.palette_open]] abre um arquivo de paleta (`.tdpal`, `.gpl`,
  `.ase` ou `.aco`).
- [[menu.window.palette_from_document]] e
  [[menu.window.palette_from_selection]] criam uma paleta com as cores usadas
  no desenho ou na seleção.
- As cores da paleta do documento aparecem embaixo; clique em uma para
  preencher a seleção, com o botão direito para colorir o contorno.

## Editar uma paleta {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]] abre o
editor de paletas.

![O Editor de paletas](../../img/color-palettes-palette-editor.png)

1. Escolha a [[dialog.palette]] a editar, ou clique em [[dialog.new_palette]]
   para começar uma.
2. Digite o [[dialog.name]].
3. Clique em uma cor para selecioná-la; [[dialog.add_color]] acrescenta a cor
   atual e [[dialog.remove_color]] remove a selecionada.
4. Clique em [[dialog.save_palette]] para salvá-la como arquivo.
5. Clique em [[dialog.close]].

As paletas de fábrica não podem ser editadas; crie uma paleta nova ou edite uma
cópia.
