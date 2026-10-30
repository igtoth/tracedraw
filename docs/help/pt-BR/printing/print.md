# Imprimir um desenho

Escolha [[menu.file > menu.file.print]] (<kbd>Ctrl+P</kbd>). O TraceDraw monta
um PDF das folhas e o abre no visualizador do seu sistema, onde você escolhe a
impressora e imprime. Na versão para navegador, o PDF é baixado.

![A caixa de diálogo Imprimir](../../img/printing-print.png)

## Geral {#general}

- **Destino**: a impressora, um arquivo PDF ou um arquivo PostScript.
- **Orientação**: igual ao layout, retrato ou paisagem.
- **Intervalo de impressão**: o documento inteiro, a página atual, uma lista de
  páginas (como `1-3, 5, 8-`) ou os objetos selecionados. Os objetos
  selecionados saem sozinhos (nada mais da página, das camadas mestras ou do
  fundo), numa página do tamanho da área que ocupam: com a folha no tamanho
  da página o papel é essa área, numa folha fixa eles ficam centralizados no
  seu tamanho (menores só quando não cabem, a não ser que **Manter tamanho da página**
  esteja marcado), e os layouts repetem essa área.
- **Cópias**, agrupadas em conjuntos inteiros ou não.
- **Imprimir como bitmap**: cada folha vira uma imagem na resolução que você
  der.

## Cor {#color}

- **Composta** imprime todas as cores em cada folha; **Separações** imprime uma
  página cinza por tinta (ciano, magenta, amarelo, preto, depois cada tinta
  especial).
- **Converter cores especiais em CMYK** imprime as tintas especiais como a
  mistura CMYK delas.

## Composta {#composite}

- **Imprimir texto em preto**.
- **Sempre sobreimprimir o preto**: preenchimentos e contornos em preto total
  são impressos sobre as cores embaixo.

## Layout {#layout}

- **Como no documento**, **Ajustar à página** ou **Escala** em porcentagem.
  **Ajustar à página** também reduz objetos maiores que a página (ou a
  seleção, quando só ela é impressa) para que saiam inteiros nela.
- **Limite de sangria**: até onde além da página os objetos são impressos
  (4 mm de início).
- A imposição: várias páginas em uma folha, ou um livreto, cujas páginas são
  emparelhadas para dobrar, com o tamanho da folha e o espaço entre páginas.
  Um esboço mostra o resultado. Para o verso de cartões frente e verso, marque
  **Verso**: as páginas vão da direita para a esquerda na folha, então cada uma
  sai atrás da sua frente, e o texto não é espelhado. **Manter tamanho da
  página** mantém as páginas no tamanho delas numa folha fixa: elas não
  encolhem pelas marcas, e as colunas e linhas ficam como você definiu (as
  páginas que não cabem saem da folha, e um aviso diz isso). Sem ela, as
  páginas encolhem para caber nas colunas e linhas pedidas. Na Visualização de impressão, arraste na folha para mover a
  informação da página.

## Livretos {#booklet}

Escolha **Livreto** como imposição para imprimir duas páginas em cada lado
de uma folha, prontas para dobrar. O número de páginas é arredondado para
um múltiplo de quatro com páginas em branco no fim, e o resumo sob o esboço
diz quantas páginas, páginas em branco e folhas de papel são necessárias.
Para imprimir só algumas páginas, liste-as antes em **Páginas**.

- [[print.binding]]: [[print.saddle_stitch]] encaixa cada folha dentro da
  anterior e as grampeia na dobra (8 páginas: 8 1, 2 7 na folha externa).
  [[print.perfect_bound]] dobra as folhas em cadernos de
  [[print.signature_sheets]], empilha os cadernos e os cola na lombada (uma
  folha por caderno: 4 1, 2 3, depois 8 5, 6 7).
- [[print.creep]] (grampo a cavalo): quanto as páginas da folha mais interna
  se movem em direção à lombada, para que as bordas se alinhem depois do
  refile; as folhas intermediárias se movem em proporção, a externa nada.
- [[print.booklet_gap]]: o espaço entre as duas páginas de uma página dupla,
  centrado na dobra.

## Pré-impressão {#prepress}

- **Inverter** e **Espelhar** a folha inteira, para filme.
- **Imprimir informações do arquivo** (o nome do desenho ou um texto seu, os
  números de página, o número da folha e a data) e **Imprimir números de
  página**.
- **Marcas de corte** (e marcas de dobra para livretos), **Marcas de
  registro**, **Barras de cor** e **Escala densitométrica**.

## PostScript e problemas {#issues}

Com um arquivo PostScript como destino, a aba **PostScript** define o nível da
linguagem (2 ou 3). A aba **Problemas** mostra o que a verificação prévia
encontra; veja [Verificação prévia](preflight.md).

**Aplicar** guarda as configurações para a próxima vez; veja
[Opções globais](../customize/global-options.md#printing) para os padrões.

## Prévia de impressão {#preview}

[[menu.file > menu.file.print_preview]], ou **Prévia de impressão** na caixa de
diálogo Imprimir, mostra as folhas exatamente como serão impressas, com a
imposição e as marcas. A barra no topo tem **Imprimir**, **Fechar**, a folha
mostrada com anterior e próxima (<kbd>PgUp</kbd>, <kbd>PgDn</kbd>), a chapa a
mostrar ao imprimir separações, e o zoom. As opções à esquerda mudam as folhas
na hora.

## Outras formas de imprimir {#other}

- [[menu.file > menu.file.send_to]] salva um PDF na sua área de trabalho ou
  pasta de documentos, ou o anexa a um e-mail.
- [[menu.file.publish_to_pdf]] grava um PDF com controle total das
  configurações; veja [Publicar em PDF](../file-formats/publish-pdf.md).
