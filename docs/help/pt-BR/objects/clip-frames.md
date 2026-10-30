# Colocar objetos dentro de uma moldura

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Uma **moldura de recorte** é um objeto fechado que contém outros objetos e
mostra só as partes deles que ficam dentro dela, como uma imagem vista por uma
janela. A moldura mantém o próprio preenchimento e contorno; os objetos de
dentro ficam inteiros e podem ser retirados de novo. Os comandos ficam em
[[menu.object > menu.object.clip_frame]].

## Para colocar objetos dentro de uma moldura {#place-inside}

1. Selecione os objetos a colocar dentro.
2. Escolha [[menu.object > menu.object.clip_frame > menu.object.clip_frame_place_inside]].
3. Clique no objeto fechado que será a moldura.

Os objetos mantêm a posição. Você também pode arrastar os objetos até a
moldura com o botão direito do mouse e escolher colocá-los dentro; veja
[Arrastar com o botão direito](selecting.md#right-drag).

## Editar o que está dentro {#edit-contents}

- [[menu.object.clip_frame_edit]] entra na moldura: só os objetos de dentro
  podem ser editados, o resto da página fica esmaecido e a moldura aparece como
  contorno. [[menu.object.clip_frame_finish]], ou o botão na barra de
  propriedades, sai dela.
- [[menu.object.clip_frame_extract]] torna os objetos de dentro objetos comuns
  de novo; a moldura fica, vazia.
- [[menu.object.clip_frame_lock]] (ativado de início): mover, escalonar ou
  girar a moldura leva o conteúdo junto. Desativado, a moldura se move sozinha
  e o conteúdo fica onde está na página.

Clicar duas vezes em uma moldura com a ferramenta [[tool.pick]], ou
<kbd>Ctrl</kbd>+clique nela, também entra nela, com o objeto sob o ponteiro
selecionado; veja [Modo de foco](focus-mode.md).

Enquanto você edita o conteúdo, os objetos esmaecidos fora da moldura não
podem ser escolhidos (por clique, quadro de seleção, Selecionar tudo ou
<kbd>Tab</kbd>). Desfazer para no início da edição: desfazer além dele sai da
edição com a moldura como estava; refazer entra de novo, e desfazer Concluir
volta para dentro dela.

## Máscaras de opacidade {#opacity-mask}

[[menu.object > menu.object.clip_frame > menu.object.clip_frame_opacity_mask]]
transforma as molduras de recorte selecionadas em máscaras de opacidade: a
moldura deixa de ser desenhada, e o brilho dela decide quanto do conteúdo
aparece: tudo onde ela é branca, nada onde é preta, e nada fora da moldura.
Preencha a moldura com um gradiente para um esmaecimento suave.
[[menu.object.clip_frame_invert_mask]] inverte isso, de modo que o preto
mostra. Escolha o comando de novo para voltar a uma moldura simples.

## Molduras vazias {#empty-frames}

Clique com o botão direito em um objeto e use os comandos de tipo de moldura:

- **Criar moldura de recorte vazia** transforma cada objeto selecionado em uma
  moldura vazia, pronta para receber objetos.
- **Sem moldura** devolve uma moldura vazia ao objeto que ela era.
- **Criar moldura de texto vazia** substitui o objeto por uma moldura de texto
  de parágrafo vazia do mesmo tamanho, pronta para a ferramenta [[tool.text]].

Recortar um bitmap com a ferramenta [[tool.crop]] ou com a ferramenta
[[tool.shape]] também o coloca em uma moldura de recorte; veja
[Recortar, cortar e apagar](../lines-shapes/cut-and-erase.md#crop).
