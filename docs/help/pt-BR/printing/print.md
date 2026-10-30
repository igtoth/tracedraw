# Imprimir um desenho

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Escolha [[menu.file > menu.file.print]] (<kbd>Ctrl+P</kbd>). O TraceDraw monta
as folhas e as envia à impressora que você escolher, ou abre um PDF delas no
visualizador do seu sistema, onde você escolhe a impressora e imprime. Na
versão para navegador, o PDF é baixado.

Imprimir em uma impressora acontece em segundo plano: a janela continua
funcionando, a barra de status diz "Imprimindo em ..." e depois como foi a
impressão.

As guias são Geral, Cor, Composta, Layout, Pré-impressão, PostScript e
Problemas. Abaixo delas, **Prévia de impressão** abre a prévia com essas
configurações, e **Aplicar** as guarda para a próxima vez.

![A caixa de diálogo Imprimir](../../img/printing-print.png)

## Geral {#general}

- **Destino**: uma das impressoras do sistema, o visualizador de PDF do
  sistema (que então escolhe a impressora), um arquivo PDF ou um arquivo
  PostScript. O botão de engrenagem ao lado de uma impressora abre as
  configurações dela, e o estado, o local e o comentário da impressora
  aparecem abaixo. A caixa de diálogo começa na impressora usada por último.
- **Orientação**: igual ao layout, retrato ou paisagem.
- **Folha**: o tamanho da própria página, os tamanhos de papel padrão, ou um
  tamanho personalizado que você pode salvar com um nome.
- **Intervalo de impressão**: o documento inteiro, a página atual, uma lista de
  páginas (como `1-3, 5, 8-`) ou os objetos selecionados; e todas, as ímpares
  ou as pares do documento ou da lista, para imprimir frente e verso à mão. Os objetos
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
  especial). Na prévia de impressão você pode desmarcar tintas em **Imprimir
  separações**; só as tintas marcadas são impressas.
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
  (4 mm de início, ou a sangria do próprio desenho).
- **Margens do papel** (esquerda, direita, superior, inferior, 0 de início)
  deixam as bordas do papel livres: as páginas e as marcas vão no que sobra.
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
de uma folha, prontas para dobrar. [Imprimir livretos](booklet.md) explica
passo a passo. O número de páginas é arredondado para
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
imposição e as marcas.

- A barra no topo tem **Imprimir**, a folha mostrada com anterior e próxima
  (<kbd>PgUp</kbd>, <kbd>PgDn</kbd>), o zoom com **Ajustar**, a impressora com
  o botão de configurações e o estado, o número de cópias com **Agrupar**, e
  **Fechar** (<kbd>Esc</kbd>). Numa janela estreita a barra ocupa duas linhas.
- As opções à esquerda ficam duas por linha: as páginas (todas, a atual, uma
  lista, a seleção, ímpares ou pares), ajustar à página ou a escala, depois as
  opções de pré-impressão (separações, sangria, marcas, informações da página,
  espelhar, inverter, transformar e posição da arte), depois o layout com o
  esboço. Cada mudança refaz as folhas na hora. Arraste a borda do painel
  para alargá-lo.
- Com **Imprimir separações** ligado, uma caixa para cada tinta aparece
  abaixo. Todas marcadas mostram a folha; uma só mostra a chapa dela em cinza;
  algumas mostram as cores que essas tintas imprimem juntas.
- Arraste as linhas azuis tracejadas na folha para definir as margens do
  papel, e arraste a folha para mover a arte na página (**Posição da arte**
  mostra o deslocamento, com **Redefinir**). **Transformar** mostra alças para
  esticar a arte.

## Outras formas de imprimir {#other}

- [[menu.file > menu.file.send_to]] salva um PDF na sua área de trabalho ou
  pasta de documentos, ou o anexa a um e-mail.
- [[menu.file.publish_to_pdf]] grava um PDF com controle total das
  configurações; veja [Publicar em PDF](../file-formats/publish-pdf.md).
