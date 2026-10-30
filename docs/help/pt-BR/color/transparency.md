# Transparência

A transparência deixa o que está embaixo de um objeto aparecer através dele.
Ela pode ser igual em todo o objeto, mudar ao longo dele como um gradiente ou
seguir um padrão ou uma textura.

## Aplicar transparência {#apply}

- Com a ferramenta [[tool.transparency]]: clique em um objeto e escolha um tipo
  de transparência na barra de propriedades, ou arraste através do objeto para
  uma transparência em gradiente, de opaco (onde você pressiona) a
  transparente (onde solta). Os nós do gradiente aparecem então no objeto,
  como na ferramenta [[tool.interactive_fill]]: arraste o início, o fim ou o
  nó redondo de inclinação; arraste a barra entre dois nós (o ponto médio)
  para mudar onde a mistura chega à metade; dê um duplo clique na linha para
  acrescentar um nó, e num nó para removê-lo.
- Na seção [[docker.transparency]] do painel [[docker.properties]].

## Configurações {#settings}

- O tipo: [[transparency.none]], [[transparency.uniform]] (de início),
  [[transparency.fountain]] (linear, radial, cônico, quadrado ou moldado, como
  os preenchimentos em gradiente), [[transparency.pattern]] ou
  [[transparency.texture]].
- A quantidade: 0 a 100% (50% de início). Em um gradiente, as paradas são tons
  de cinza: branco é opaco, preto é totalmente transparente.
- [[transparency.merge_mode]]: como as cores se misturam com o que está
  embaixo. Normal (de início) simplesmente deixa ver através; os outros, como
  Adicionar, Subtrair, Diferença, Multiplicar, Dividir, Se mais claro, Se mais
  escuro, Matiz, Saturação, Cor e Inverter, combinam as cores de outras formas.
- Se vale para o [[transparency.target_fill]], o
  [[transparency.target_outline]] ou [[transparency.target_all]].
- **Congelar**: mantém o que aparece através como está agora, mesmo que os
  objetos embaixo se movam.

Para removê-la, clique em [[toolbar.clear_transparency]] no fim da barra de
propriedades (ou escolha [[transparency.none]] como tipo).

Para copiar a transparência de outro objeto, use
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
Para áreas transparentes definidas pelo brilho de um objeto, veja
[Máscaras de opacidade](../objects/clip-frames.md#opacity-mask).
