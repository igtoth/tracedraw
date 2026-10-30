# Combinar formas: soldar, aparar e interseção

A modelagem cria formas novas a partir de objetos sobrepostos: une-os em um,
corta um com outro ou mantém só onde se sobrepõem. Os comandos ficam em
[[menu.object > menu.object.shaping]], na barra de propriedades quando dois ou
mais objetos estão selecionados, e no painel [[docker.shaping]].

## O alvo e as origens {#target}

O **último objeto que você seleciona é o alvo**: ele dá ao resultado o
preenchimento, o contorno e a transparência. Os outros objetos selecionados são
as **origens**. Para escolher o alvo, selecione as origens primeiro e depois
<kbd>Shift</kbd>+clique no alvo.

O resultado vai para o topo da camada ativa e fica selecionado. Cada operação é
um passo de desfazer. Quando não há resultado (por exemplo, dois objetos que
não se sobrepõem), nada muda e a barra de status avisa.

## As operações {#operations}

| Operação | Precisa de | Resultado |
|---|---|---|
| [[menu.object.weld]] | 2 objetos | um objeto que cobre todos eles |
| [[menu.object.trim]] | 2 objetos | o alvo sem o que as origens cobrem |
| [[menu.object.intersect]] | 2 objetos | só a área que todos compartilham |
| [[menu.object.simplify]] | 1 objeto | cada objeto sem o que os objetos acima dele cobrem |
| [[menu.object.front_minus_back]] | 2 objetos | o objeto da frente sem os de trás |
| [[menu.object.back_minus_front]] | 2 objetos | o objeto de trás sem os da frente |
| [[menu.object.boundary]] | 1 objeto | um contorno em volta de toda a seleção |

Pelo menu e pela barra de propriedades, [[menu.object.weld]] remove os
originais, [[menu.object.trim]] mantém as origens, [[menu.object.intersect]]
mantém todos e [[menu.object.boundary]] mantém todos.

## O painel Modelagem {#shaping-docker}

Abra o painel [[docker.shaping]] com
[[menu.object > menu.object.shaping > menu.object.shaping_docker]].

![O painel Modelagem](../../img/lines-shapes-shaping.png)

1. Clique em um dos sete botões ilustrados para escolher a operação. O painel
   mostra o nome, uma imagem e o que ela faz.
2. Para [[menu.object.weld]], [[menu.object.trim]] e
   [[menu.object.intersect]], escolha o que manter:
   [[docker.leave_original_source]] e [[docker.leave_original_target]]. Cada
   operação lembra sua própria escolha.
3. Selecione os objetos, o alvo por último.
4. Clique no botão com o nome da operação. Ele fica claro até haver objetos
   suficientes selecionados, e uma linha diz quantos são necessários.

Relacionado: [[menu.object.combine]] (<kbd>Ctrl+L</kbd>) torna vários objetos
uma única curva sem mudar as formas, e [[menu.object.break_apart]] separa uma
curva em suas partes de novo. Veja
[Objetos e camadas](../objects/objects-and-layers.md#combine).
