# Marcas de impressão em arquivos PDF

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Uma gráfica apara, registra e confere as folhas impressas com marcas
desenhadas fora da página: linhas que mostram onde cortar, alvos que alinham
as chapas, amostras de tinta e uma linha com o nome do arquivo. A guia
Pré-impressão de [[menu.file > menu.file.publish_to_pdf]] acrescenta essas
marcas em volta de cada página do PDF, para o arquivo ir à impressão como
está.

## Acrescentar as marcas {#add}

1. Escolha [[menu.file > menu.file.publish_to_pdf]].
2. Na guia [[publish.tab_prepress]], defina a [[dialog.bleed]]: quanto o
   desenho passa da borda da página (a sangria do desenho de início, de 0 a
   50 mm). Objetos que passam da página são impressos até essa linha, para não
   sobrar borda branca depois do corte.
3. Em [[publish.marks]], marque as marcas que quiser (todas vêm desligadas):
   - [[dialog.crop_marks]]: linhas curtas (5 mm) alinhadas com as bordas da
     página, começando 1,5 mm fora da sangria, que mostram onde cortar.
   - [[dialog.registration_marks]]: um alvo (um pequeno círculo com uma cruz)
     no meio de cada lado, para alinhar as chapas de cada tinta.
   - [[print.color_bars]]: amostras das quatro tintas de processo, das
     misturas de duas delas e das tintas a 50 %, ao lado do alvo de cima, para
     conferir as tintas na impressão.
   - [[publish.file_info]]: o nome do desenho, o nome da página, o número da
     página e o total de páginas, e a data e a hora, embaixo da página.
4. Clique em [[dialog.ok]].

A predefinição [[publish.preset_prepress]] marca [[dialog.crop_marks]]. As
marcas que você marcar ficam guardadas para a próxima vez e nas predefinições
salvas com **+**.

## Como o arquivo fica {#boxes}

Com qualquer marca ligada, cada página do PDF cresce 12 mm além da sangria,
em todos os lados, para caber as marcas:

- A caixa de corte, o tamanho da peça pronta, é a página.
- A caixa de sangria é a página com a sangria. O desenho é recortado nela,
  então nada dele invade as marcas.
- A caixa de mídia, a folha inteira do PDF, inclui a sangria e a margem com as
  marcas.

Um fundo de página também cobre a sangria. Links e marcadores continuam
apontando para a página.

As marcas são desenhadas na cor de registro, então numa impressora elas saem
em todas as chapas, e saem igual nas separações feitas num visualizador de
PDF. Sem nenhuma marca ligada, a página é a página com a sangria, como antes.

> **Dica:** As marcas são as mesmas que a caixa de diálogo
> [[menu.file > menu.file.print]] desenha nas folhas (veja
> [Imprimir um desenho](../printing/print.md#prepress)). Use o PDF quando a
> gráfica pedir um arquivo; use Imprimir quando você mesmo imprimir as folhas.
