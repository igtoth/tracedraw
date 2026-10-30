# Alinhar e distribuir objetos

Alinhar coloca objetos em fila pelas bordas ou pelos centros; distribuir os
espaça por igual. Use o painel [[docker.align_distribute]]
(<kbd>Ctrl+Shift+A</kbd>), os comandos de
[[menu.object > menu.object.align_distribute]] ou as teclas abaixo.

![O painel Alinhar e distribuir](../../img/objects-align-distribute.png)

## Alinhar {#align}

1. Selecione os objetos. Selecione por último aquele com o qual os outros devem
   se alinhar.
2. Na parte [[docker.align]] do painel, escolha [[docker.align_to]]:
   - [[docker.align_active]] (de início): o objeto selecionado por último; com
     um só objeto selecionado, a página;
   - [[docker.align_page_edge]]: as bordas da página;
   - [[docker.align_page_center]]: o centro da página;
   - [[docker.align_grid]]: cada objeto na linha mais próxima da grade do
     documento.
3. Clique em um dos seis botões: bordas esquerdas, centros em uma linha
   vertical, bordas direitas, bordas superiores, centros em uma linha
   horizontal ou bordas inferiores.

[[docker.align_use_outline]]: com isso ativado, as bordas externas dos
contornos se encontram, em vez dos próprios caminhos.

As teclas sempre alinham pelo objeto selecionado por último:

| Tecla | Comando |
|---|---|
| <kbd>L</kbd> | [[menu.object.align_left]] |
| <kbd>R</kbd> | [[menu.object.align_right]] |
| <kbd>T</kbd> | [[menu.object.align_top]] |
| <kbd>B</kbd> | [[menu.object.align_bottom]] |
| <kbd>E</kbd> | [[menu.object.align_centers_h]] |
| <kbd>C</kbd> | [[menu.object.align_centers_v]] |
| <kbd>P</kbd> | [[menu.object.center_to_page]] |

## Distribuir {#distribute}

1. Selecione três ou mais objetos (dois ou mais para distribuir pela página).
2. Na parte [[docker.distribute]], escolha [[docker.distribute_to]]:
   - [[docker.distribute_selection]] (de início): o primeiro e o último objeto
     ficam onde estão, e os outros se movem entre eles;
   - [[docker.distribute_page]]: os objetos se espalham pela página inteira, o
     primeiro tocando um lado e o último o outro.
3. Clique em um dos oito botões: bordas esquerdas, centros, espaços ou bordas
   direitas na horizontal, ou bordas superiores, centros, espaços ou bordas
   inferiores na vertical. Distribuir os espaços dá a mesma distância entre os
   objetos, sejam quais forem os tamanhos.

Com poucos objetos, os botões ficam claros e o painel diz quantos são
necessários. Cada alinhamento ou distribuição é um passo de desfazer.

Para colocar cópias a distâncias iguais de uma vez, veja
[Duplicar e repetir](step-and-repeat.md).

## Encaixar objetos num caminho {#fit-to-path}

O painel [[docker.fit_to_path]] coloca objetos ao longo de uma linha, de uma
curva ou do contorno de uma forma. Abra-o com
[[menu.object > menu.object.fit_objects_to_path]].

1. Selecione os objetos a colocar e, por último, o caminho (Shift+clique).
2. Em [[docker.fit_to_path_spacing]], escolha
   [[docker.fit_to_path_even]] para espalhar os objetos pelo caminho, ou
   [[docker.fit_to_path_distance]] e digite a [[docker.fit_to_path_distance_value]]
   de um objeto ao seguinte. [[docker.fit_to_path_measure]] mede essa
   distância entre os [[docker.fit_to_path_centres]] dos objetos vizinhos ou
   entre as [[docker.fit_to_path_edges]], para que objetos de tamanhos
   diferentes fiquem com o mesmo vão.
3. [[docker.fit_to_path_offset]] afasta o primeiro objeto do início do
   caminho; [[docker.fit_to_path_end_offset]] mantém os objetos longe do fim
   de um caminho aberto. Com distância fixa, [[docker.fit_to_path_align]]
   põe a fileira no [[docker.fit_to_path_align_start]], no
   [[docker.fit_to_path_align_middle]] ou no
   [[docker.fit_to_path_align_end]].
4. Em [[docker.fit_to_path_copies]], [[docker.fit_to_path_copies_count]]
   enche o caminho com esse número de objetos: os que você selecionou e
   depois cópias deles, um de cada vez. Com distância fixa,
   [[docker.fit_to_path_copies_fill]] faz quantas cópias couberem.
5. Em [[docker.fit_to_path_place]], coloque os objetos
   [[docker.fit_to_path_on]], [[docker.fit_to_path_above]] ou
   [[docker.fit_to_path_below]], com uma [[docker.fit_to_path_side]].
6. Em [[docker.fit_to_path_rotation]], escolha
   [[docker.fit_to_path_rotate]] para girar cada objeto com a direção do
   caminho, ou [[docker.fit_to_path_fixed]] para girar todos igualmente; o
   [[docker.fit_to_path_angle]] é somado nos dois casos.
7. [[docker.fit_to_path_order]] define qual objeto vem primeiro: como
   selecionados, invertida, da esquerda para a direita ou da direita para
   a esquerda, por tamanho, ou grande e pequeno alternados.
   [[docker.fit_to_path_reverse]] começa pela outra ponta do caminho.
8. Assim que você muda um valor, os contornos na tela mostram onde os
   objetos e as cópias vão ficar. Clique em [[docker.apply]]. As cópias
   vão para a camada do objeto que copiam, e os objetos colocados ficam
   selecionados.

Outras opções do painel:

- [[docker.fit_to_path_keep_originals]] deixa os objetos selecionados onde
  estão e posiciona cópias; [[docker.fit_to_path_group]] põe o que foi
  posicionado em um só grupo.
- [[docker.fit_to_path_contiguous]] percorre todas as partes de um caminho
  feito de várias peças, uma após a outra; desligado, só a primeira peça é
  usada.
- [[docker.fit_to_path_reference]] escolhe o ponto de cada objeto que vai
  sobre o caminho: seu [[docker.fit_to_path_ref_pivot]] (movido com a
  ferramenta Seleção, se você o moveu), ou um ponto da sua
  [[docker.fit_to_path_ref_box]] ou [[docker.fit_to_path_ref_outline]],
  escolhido na grade [[docker.fit_to_path_origin]]. O ponto do meio embaixo
  deixa os objetos em pé sobre o caminho.
- [[docker.fit_to_path_ignore_rotation]] endireita objetos que estavam
  girados antes de posicioná-los.
- Em [[docker.fit_to_path_modify_rotation]], um
  [[docker.fit_to_path_spin]] soma um giro a cada objeto:
  [[docker.fit_to_path_spin_uniform]] gira todos pelo
  [[docker.fit_to_path_start_angle]], [[docker.fit_to_path_spin_progressive]]
  gira cada um um pouco mais que o anterior (o
  [[docker.fit_to_path_spin_angle]] vezes as
  [[docker.fit_to_path_revolutions]] ao longo da fileira inteira),
  [[docker.fit_to_path_spin_jitter]] gira cada um de um valor diferente até a
  [[docker.fit_to_path_range]], e
  [[docker.fit_to_path_spin_progressive_jitter]] faz os dois. Marque
  [[docker.fit_to_path_clockwise]] para girar no outro sentido. Só os campos
  que o estilo usa podem ser mudados.

O painel começa com os valores padrão, sem contornos, sempre que você
seleciona outros objetos (ou trabalha em outro desenho); os valores que
você digita ficam enquanto os mesmos objetos estão selecionados. Numa forma fechada, os objetos dão a volta inteira sem que o primeiro e o
último se encontrem. Encaixar é um passo de desfazer.
