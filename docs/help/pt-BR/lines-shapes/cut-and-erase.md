# Recortar, cortar e apagar

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Quatro ferramentas do submenu Recortar tiram partes de objetos:
[[tool.crop]], [[tool.knife]], [[tool.segment_delete]] e [[tool.eraser]]. Cada
alteração é um passo de desfazer.

## Recortar {#crop}

A ferramenta [[tool.crop]] remove tudo o que fica fora de um retângulo.

1. Selecione os objetos a recortar, ou não selecione nada para recortar todos
   os objetos desbloqueados da página.
2. Escolha a ferramenta [[tool.crop]] e arraste a área de recorte. O desenho
   fora dela fica escurecido.
3. Ajuste a área: arraste por dentro para movê-la, arraste uma das oito alças
   para redimensioná-la, ou digite a posição e o tamanho na barra de
   propriedades.
4. Clique duas vezes dentro da área, pressione <kbd>Enter</kbd> ou clique em
   **Recortar** na barra de propriedades.

Para desistir, pressione <kbd>Esc</kbd>, clique fora da área, clique em
**Limpar área de recorte** ou escolha outra ferramenta.

O que acontece com cada objeto:

- objetos totalmente dentro ficam como estão, e objetos totalmente fora são
  excluídos;
- formas fechadas são cortadas na área, e curvas abertas são cortadas onde
  cruzam a borda; o texto vira curvas antes;
- bitmaps, tabelas e símbolos são colocados dentro de uma moldura da área, de
  modo que o conteúdo fica inteiro e pode ser retirado de novo (veja
  [Molduras de recorte](../objects/clip-frames.md));
- grupos são recortados membro a membro.

## Faca {#knife}

A ferramenta [[tool.knife]] divide objetos ao longo de um corte.

1. Selecione os objetos a cortar, ou não selecione nada para cortar todos os
   objetos visíveis da página.
2. Escolha a ferramenta [[tool.knife]] e o tipo de corte na barra de
   propriedades: **Linha de dois pontos** (um corte reto que atravessa os
   objetos) ou **Mão livre** (um corte que segue o arraste).
3. Arraste através dos objetos.

Na barra de propriedades, **Dividir** separa as partes com uma fresta mínima,
então elas ainda se tocam, e **Vão** remove uma faixa da largura digitada. Com
**Manter como um objeto**, as partes de cada objeto continuam sendo um único
objeto de várias partes. Curvas abertas são divididas onde o corte as cruza. As
partes mantêm preenchimento, contorno e transparência e ficam selecionadas.
Bitmaps, grupos e texto não são cortados.

## Excluir segmento {#segment-delete}

A ferramenta [[tool.segment_delete]] exclui as partes dos contornos entre os
lugares onde eles cruzam outros contornos ou a si mesmos, por exemplo os
pedaços de linha que sobram depois de um cruzamento.

- Clique em uma parte de um contorno para excluí-la.
- Arraste um retângulo por cima dos contornos para excluir todas as partes que
  ele tocar, em todos os objetos de uma vez. Cada parte sai inteira, até os
  cruzamentos nas pontas, mesmo onde passa do retângulo; uma forma fechada que
  nada cruza sai inteira.
- Escolha **Mão livre** na barra de propriedades e arraste um traço por cima
  dos contornos: cada parte que o traço cruzar é excluída, até os cruzamentos
  com outras linhas, em um passo. Mantenha <kbd>Alt</kbd> pressionado ao
  começar a arrastar para usar o outro modo (traço ou retângulo) só naquele
  arraste.

Retângulos, elipses e polígonos viram curvas; uma forma fechada que perde uma
parte se abre, e um objeto sem nada sobrando é excluído.

## Borracha {#eraser}

A ferramenta [[tool.eraser]] (<kbd>X</kbd>) apaga partes de objetos.

1. Selecione os objetos a apagar, ou não selecione nada para apagar o objeto
   onde você começar a arrastar.
2. Escolha a ferramenta [[tool.eraser]]. Defina a **espessura** (6,35 mm de
   início) e a ponta **redonda** ou **quadrada** na barra de propriedades.
3. Arraste sobre os objetos. Um clique apaga um ponto.

Objetos apagados viram curvas; um objeto apagado por inteiro é excluído.
Clique duas vezes em um objeto com a ferramenta para excluí-lo inteiro.
Bitmaps, grupos e molduras de recorte não são apagados. Os padrões da borracha
ficam na página [[tool.eraser]] das opções de ferramentas; veja
[Opções das ferramentas](../customize/tool-options.md#eraser).
