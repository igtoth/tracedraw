# Encaixar objetos num caminho

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

O painel [[docker.fit_to_path]] coloca objetos ao longo de uma linha, de uma
curva ou do contorno de uma forma: contas num fio, letras ou estrelas em volta
de um selo, uma fileira de mourões ao longo de uma estrada, traços seus ao
longo de uma borda. Ele pode fazer cópias para preencher o caminho,
espaçá-las por igual ou a uma distância definida, pô-las em pé sobre o caminho
ou girá-las com ele.

## Colocar objetos num caminho {#place}

1. Selecione os objetos a colocar e, por último, o caminho (Shift+clique). O
   painel mostra quantos objetos estão selecionados e que tipo de objeto é o
   caminho.
2. Escolha [[menu.object > menu.object.fit_objects_to_path]] para abrir o
   painel.
3. Mude os valores de que precisar (veja abaixo). Assim que um valor muda,
   contornos tracejados na tela mostram onde cada objeto e cópia vai ficar.
4. Clique em [[docker.apply]].

Encaixar se desfaz em um passo. As cópias vão para a camada do objeto que
copiam, com o preenchimento, o contorno e os efeitos dele. Depois de um
Aplicar que fez cópias, os objetos colocados e as cópias ficam selecionados
(não o caminho).

O painel começa dos valores padrão, sem contornos, cada vez que você seleciona
outros objetos ou trabalha em outro desenho; os valores digitados ficam
enquanto os mesmos objetos estiverem selecionados.

## As configurações {#settings}

Os grupos do painel, de cima para baixo:

### Objetos e caminho {#objects}

- [[docker.fit_to_path_keep_originals]] (desligado): os objetos selecionados
  ficam onde estão e cópias deles são colocadas.
- [[docker.fit_to_path_group]] (desligado): o que foi colocado vai para um
  grupo, que fica selecionado.
- [[docker.fit_to_path_contiguous]] (ligado): um caminho feito de várias
  partes é percorrido parte após parte, como um caminho só. Desligado, só a
  primeira parte é usada.

### Espaçamento {#spacing}

- [[docker.fit_to_path_spacing]]: [[docker.fit_to_path_even]] (o padrão)
  espalha os objetos pelo caminho inteiro; [[docker.fit_to_path_distance]] os
  põe à [[docker.fit_to_path_distance_value]] um do outro (10 mm de início).
- [[docker.fit_to_path_measure]]: entre os [[docker.fit_to_path_centres]] de
  objetos vizinhos (o padrão) ou entre as [[docker.fit_to_path_edges]] deles,
  para objetos de tamanhos diferentes ficarem com o mesmo vão.
- [[docker.fit_to_path_offset]] (0): move o primeiro objeto ao longo do
  caminho a partir do início. Um valor negativo é aceito.
- [[docker.fit_to_path_end_offset]] (0, só caminhos abertos): mantém os
  objetos longe do fim do caminho.
- [[docker.fit_to_path_align]] (só distância fixa): põe a fileira no
  [[docker.fit_to_path_align_start]] (o padrão), no
  [[docker.fit_to_path_align_middle]] ou no [[docker.fit_to_path_align_end]]
  do caminho.

### Cópias {#copies}

- [[docker.fit_to_path_copies_none]] (o padrão): cada objeto selecionado é
  colocado uma vez.
- [[docker.fit_to_path_copies_count]] (de 1 a 1000, 10 de início): esse
  número de objetos ao todo: os selecionados, depois cópias deles na mesma
  ordem.
- [[docker.fit_to_path_copies_fill]] (só distância fixa): tantos quantos
  couberem no caminho.

Nunca há mais de 1000 objetos ao todo.

### Posição {#placement}

- [[docker.fit_to_path_place]]: [[docker.fit_to_path_on]] (o padrão),
  [[docker.fit_to_path_above]] ou [[docker.fit_to_path_below]], com uma
  [[docker.fit_to_path_side]] (0 de início) entre os objetos e o caminho.
  "Acima" fica à esquerda do sentido do caminho.
- [[docker.fit_to_path_reference]]: o ponto de cada objeto que vai no
  caminho: o [[docker.fit_to_path_ref_pivot]] dele (o padrão; o centro, ou
  onde você o moveu com a ferramenta Seleção), ou um ponto da
  [[docker.fit_to_path_ref_box]] ou da [[docker.fit_to_path_ref_outline]].
- [[docker.fit_to_path_origin]] (só referências de caixa): um de nove pontos
  da caixa, escolhido na grade (o centro de início). O ponto do meio de baixo
  põe os objetos em pé sobre o caminho.

### Rotação {#rotation}

- [[docker.fit_to_path_rotation]]: [[docker.fit_to_path_rotate]] gira cada
  objeto com o sentido do caminho; [[docker.fit_to_path_fixed]] (o padrão)
  gira todos igual. O [[docker.fit_to_path_angle]] (0, em graus no sentido
  anti-horário) é somado nos dois casos.
- [[docker.fit_to_path_ignore_rotation]] (desligado): endireita objetos que
  estavam girados antes de colocá-los.
- Em [[docker.fit_to_path_modify_rotation]], o [[docker.fit_to_path_spin]]
  soma um giro a cada objeto:
  - [[docker.fit_to_path_spin_none]] (o padrão): nenhum giro extra.
  - [[docker.fit_to_path_spin_uniform]]: todo objeto gira o
    [[docker.fit_to_path_start_angle]].
  - [[docker.fit_to_path_spin_progressive]]: cada objeto gira um pouco mais
    que o anterior, o [[docker.fit_to_path_spin_angle]] (360 de início) vezes
    as [[docker.fit_to_path_revolutions]] (1) na fileira inteira.
  - [[docker.fit_to_path_spin_jitter]]: cada objeto gira um valor diferente,
    até a [[docker.fit_to_path_range]] (15 graus) para cada lado. O mesmo
    objeto gira do mesmo jeito sempre, na prévia e depois de Aplicar.
  - [[docker.fit_to_path_spin_progressive_jitter]]: os dois.

  Marque [[docker.fit_to_path_clockwise]] para girar no outro sentido. Só os
  campos que o estilo usa podem ser mudados.

### Ordem {#order}

- [[docker.fit_to_path_order]]: qual objeto vai primeiro:
  [[docker.fit_to_path_order_selected]] (o padrão),
  [[docker.fit_to_path_order_reversed]], [[docker.fit_to_path_order_ltr]],
  [[docker.fit_to_path_order_rtl]], [[docker.fit_to_path_order_largest]],
  [[docker.fit_to_path_order_smallest]],
  [[docker.fit_to_path_order_alternate]], [[docker.fit_to_path_order_ttb]],
  [[docker.fit_to_path_order_btt]], [[docker.fit_to_path_order_tall]],
  [[docker.fit_to_path_order_short]], [[docker.fit_to_path_order_wide]],
  [[docker.fit_to_path_order_narrow]] ou [[docker.fit_to_path_order_random]]
  (embaralhados sempre do mesmo modo para os mesmos objetos).
- [[docker.fit_to_path_reverse]] (desligado): começa pela outra ponta do
  caminho.

## Dicas e limites {#tips}

- Numa forma fechada, os objetos dão a volta inteira, o lado de fechamento
  inclusive, sem o primeiro e o último se encontrarem.
- Num caminho aberto, um objeto que cairia além de uma ponta fica nessa ponta.
- Com mais de 200 objetos, cada um aparece na prévia como a sua caixa; com
  mais de 200 selecionados não há prévia, mas Aplicar funciona.
- Para pôr texto ao longo de um caminho, veja
  [Adicionar e editar texto](../text/adding-text.md). Para espalhar objetos
  numa fileira reta, veja [Alinhar e distribuir](align-distribute.md).
