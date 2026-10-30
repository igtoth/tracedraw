# Misturas

Uma mistura desenha uma série de passos intermediários entre dois objetos,
mudando forma, tamanho e cor de um para o outro. Use-a para sombreamentos
suaves, padrões repetidos ou uma forma que se transforma em outra.

## Para misturar dois objetos {#create}

Com a ferramenta:

1. Escolha a ferramenta [[tool.blend]] no submenu Efeitos.
2. Arraste do objeto inicial até o objeto final.

Com o painel:

1. Selecione dois objetos.
2. Abra o painel [[docker.blend]] com [[menu.effects > menu.effects.blend]]
   (<kbd>Ctrl+F8</kbd>).
3. Defina as opções e clique em [[docker.apply]].

![O painel Mistura](../../img/effects-blend.png)

Os dois objetos continuam sendo objetos do desenho: mova, redimensione ou mude a
cor de qualquer um e os passos acompanham. Excluir o objeto final remove a
mistura.

## Configurações da mistura {#settings}

Com uma mistura selecionada, toda alteração na barra de propriedades ou no
painel vale na hora.

- [[docker.blend_by_steps]]: o número de passos (5 de início, até 999).
- **Espaçamento**, para uma mistura em um caminho: a distância entre os passos;
  o número de passos sai do comprimento do caminho.
- **Rotação**: gira os passos progressivamente; **Laço** os gira em volta do
  meio entre os dois objetos.
- **Aceleração**: o controle de objeto faz os passos se concentrarem perto de
  uma ponta; o de cor faz o mesmo com as cores.
  [[docker.blend_apply_sizing]] faz a aceleração de objeto mudar também os
  tamanhos, e [[docker.blend_link_accel]] mantém os dois controles juntos.
  **Redefinir** devolve os dois ao equilíbrio.
- [[docker.blend_color_blend]]: [[docker.blend_direct]] (direto de uma cor à
  outra), [[docker.blend_clockwise]] ou [[docker.blend_counterclockwise]] (pela
  roda de cores).

A barra de propriedades da ferramenta tem os passos, a rotação e as duas
acelerações.

## Misturar ao longo de um caminho {#path}

1. Selecione a mistura.
2. Clique em **Novo caminho** no painel [[docker.blend]] e clique em uma curva.

Os passos seguem a curva. **Girar todos os objetos** gira cada passo ao longo da
curva, e [[docker.blend_full_path]] coloca o primeiro e o último passo nas
pontas da curva.

## Remover ou separar uma mistura {#clear}

**Limpar mistura** remove os passos e deixa os dois objetos. **Separar**
transforma os passos em objetos próprios.
Os passos saem agrupados ao lado do objeto. [[menu.object > menu.object.break_blend_apart]] (<kbd>Ctrl+K</kbd>, também no
menu do botão direito) faz o mesmo.
