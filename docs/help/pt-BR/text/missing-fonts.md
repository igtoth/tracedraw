# Fontes ausentes

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Um desenho feito em outro computador pode usar fontes que não estão
instaladas no seu. O TraceDraw sempre mostra esse texto em uma fonte instalada
parecida e mantém o nome da fonte do texto. Quando o desenho abre, a caixa de
diálogo [[missing_fonts.title]] deixa você escolher qual fonte instalada
desenha cada uma que falta, por enquanto ou de vez.

## Escolher as fontes substitutas {#dialog}

Quando você abre um desenho com fontes ausentes, a caixa de diálogo
[[missing_fonts.title]] as lista:

1. Cada linha mostra a [[missing_fonts.missing]] e a fonte com que ela é
   [[missing_fonts.substitute]] agora. A primeira opção da lista é a fonte que
   o TraceDraw escolheu sozinho, marcada "(automatic)": uma fonte desenhada
   com as mesmas larguras quando existe, senão uma do mesmo estilo.
2. Para usar outra fonte, abra a lista da linha e clique em uma família
   instalada. Digite parte de um nome em [[missing_fonts.filter]] para filtrar
   as listas.
3. Escolha por quanto tempo a escolha vale:
   - [[missing_fonts.session]] (o padrão): a fonte escolhida desenha o texto
     até o TraceDraw fechar, negrito e itálico inclusive. O desenho mantém os
     próprios nomes de fonte, então é salvo com eles e aparece nas próprias
     fontes num computador que as tenha.
   - [[missing_fonts.keep]]: todo texto na família ausente, também em grupos,
     molduras de recorte, tabelas e símbolos, recebe a fonte escolhida. Isso
     muda o desenho e se desfaz em um passo.
4. Clique em [[dialog.ok]].

[[dialog.cancel]], <kbd>Esc</kbd> ou o botão de fechar não mudam nada: as
fontes automáticas ficam. De qualquer forma, a barra de status então cita as
fontes ainda ausentes e as fontes que as desenham, ou diz que foram
substituídas. Quando alguns objetos do arquivo não puderam ser lidos, o aviso
deles vem depois da caixa de diálogo.

## Desligar e ligar a caixa de diálogo {#turn-off}

Marque [[notes.dont_show]] na caixa de diálogo para não vê-la mais. O desenho
então abre direto e a barra de status cita as fontes ausentes. Clique nesse
aviso da barra de status para abrir a página [[options.warnings]] de
[[menu.tools > menu.tools.options > menu.tools.options_global]], onde
[[options.ask_missing_fonts]] liga a caixa de diálogo de novo (vem ligada de
início).

[[options.warn_missing_fonts]], na mesma página, é uma mensagem simples à
parte que só cita as fontes ausentes depois que o desenho abre.

Abrir desenhos por scripts, pela linha de comando ou pela conexão de IA nunca
para nesta caixa de diálogo.

## Outras formas de lidar com fontes ausentes {#other}

- Instale as fontes, ou acrescente a pasta delas no Gerenciador de fontes
  (veja [Fontes e glifos](fonts.md#font-manager)); o texto passa a aparecer
  nelas.
- No painel [[docker.fonts]], as famílias que o desenho usa e que não estão
  instaladas aparecem marcadas como [[docker.missing]].
- Desenhos salvos como `.tdraw` e arquivos `.cdr` com [[dialog.embed_fonts]]
  levam as fontes junto, e um [pacote](../file-formats/package.md) guarda os
  arquivos de fonte na pasta `Fonts`.
- A [verificação prévia](../printing/preflight.md) lista o texto em fontes não
  instaladas antes de imprimir ou publicar.
