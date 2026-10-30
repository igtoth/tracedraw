# Abrir e salvar desenhos

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

O TraceDraw salva os desenhos no próprio formato, `.tdraw`, que guarda tudo, e
também pode salvar arquivos `.cdr`. Ele abre muitos outros formatos; veja
[Formatos de arquivo compatíveis](../file-formats/supported-formats.md).

## Abrir um desenho {#open}

1. Escolha [[menu.file > menu.file.open]] (<kbd>Ctrl+O</kbd>) ou clique em
   Abrir na barra de ferramentas padrão.
2. Escolha o arquivo e confirme.

O desenho abre em uma nova guia. Para abrir um desenho usado recentemente, use
[[menu.file > menu.file.open_recent]], a seta do botão Abrir ou a lista da Tela
de boas-vindas; [[menu.file.clear_recent]] esvazia a lista. Para tirar só um
desenho da lista da Tela de boas-vindas, aponte para ele e clique no x à
direita, ou clique nele com o botão direito e escolha
[[welcome.remove_recent]]; o arquivo continua onde está.
[[menu.file > menu.file.import]] (<kbd>Ctrl+I</kbd>) acrescenta o conteúdo de
um arquivo ao desenho aberto; veja [Importar arquivos](../file-formats/import.md).

Você também pode arrastar arquivos do gerenciador de arquivos para o
TraceDraw. Um desenho (como um arquivo `.tdraw` ou `.cdr`) solto sobre a barra
de título, os menus, as barras de ferramentas ou as guias de documentos abre em
uma guia própria; solto sobre a janela de desenho, é importado no desenho
aberto. A moldura e o rótulo mostrados durante o arraste dizem o que vai
acontecer. Onde o sistema não informa onde está o ponteiro (Linux, macOS),
desenhos abrem em uma guia e imagens são importadas.

Como um desenho aparece primeiro (a página inteira, seus objetos, a largura da
página, o tamanho real, ou a página, a posição e o zoom que tinha ao ser salvo)
é definido em [[options.open_view]], na página [[options.display]] de
[[menu.tools > menu.tools.options > menu.tools.options_app]].

### Avisos depois de abrir {#open-notes}

Quando alguns objetos de um arquivo aberto não puderam ser lidos, ou o desenho
usa fontes que não estão instaladas no seu computador, o TraceDraw avisa
depois que o arquivo abre:

- Para objetos que não puderam ser lidos, uma mensagem informa quantos são. Um
  botão **Relatar** abre um novo problema para o arquivo no rastreador do
  projeto, para que seja corrigido.
- Para fontes ausentes, a caixa de diálogo [[missing_fonts.title]] lista cada
  uma com a fonte instalada que a desenha e deixa você escolher outra para esta
  sessão ou substituí-la no desenho. Veja
  [Fontes ausentes](../text/missing-fonts.md).

Marque **Não mostrar esta mensagem novamente** para não ver mais esses avisos.
Você pode reativá-los na página [[options.warnings]] de
[[menu.tools > menu.tools.options > menu.tools.options_global]].

## Salvar um desenho {#save}

Escolha [[menu.file > menu.file.save]] (<kbd>Ctrl+S</kbd>) para gravar o
desenho de volta no seu arquivo. Um desenho novo, ou aberto de um formato que o
TraceDraw só lê, ainda não tem arquivo, então Salvar abre a caixa de diálogo
[[dialog.save_drawing]].

## Salvar como {#save-as}

[[menu.file > menu.file.save_as]] (<kbd>Ctrl+Shift+S</kbd>) salva o desenho com
outro nome, em outra pasta ou em outro formato.

![A caixa de diálogo Salvar desenho](../../img/get-started-open-and-save-save-as.png)

Para salvar um desenho com um nome novo:

1. Escolha a pasta. A lista à esquerda tem atalhos para
   [[dialog.place_home]], [[dialog.place_desktop]],
   [[dialog.place_documents]], [[dialog.place_downloads]] e
   [[dialog.place_pictures]], e para as unidades. [[dialog.up_folder]] vai para
   a pasta acima; você também pode digitar uma pasta na barra de caminho.
   Clique duas vezes em uma pasta da lista para abri-la; clique em um arquivo
   para usar o nome dele.
2. Digite o [[dialog.file_name]]. A extensão do tipo escolhido é acrescentada.
3. Escolha [[dialog.save_as_type]]: TraceDraw (`*.tdraw`) ou
   [[file.cdr_files]].
4. Escolha a [[dialog.cdr_version]]. Para arquivos TraceDraw, é a versão do
   formato, da mais nova para a mais antiga. Para arquivos `.cdr`, é a versão do
   arquivo, de 27 até 7; um programa mais antigo só abre uma versão tão antiga
   quanto ele ou mais. A versão escolhida fica com o desenho, então Salvar a
   grava de novo sem perguntar.
5. Escolha o que vai no arquivo junto com o desenho:
   - Para arquivos `.cdr` a partir da versão 16,
     [[dialog.embed_color_profile_none]] guarda o perfil CMYK do desenho no
     arquivo.
   - [[dialog.embed_fonts]] (desligado de início) guarda as fontes que o texto
     usa, quando a licença permite, em arquivos TraceDraw e em arquivos `.cdr`
     a partir da versão 16. A caixa mostra quanto as fontes acrescentam ao
     arquivo. Marque-a para desenhos que vão para um computador que talvez não
     tenha as fontes.
     Uma nota abaixo da caixa nomeia as fontes que o arquivo não vai levar e
     por quê: a licença proíbe incorporar, ou não há arquivo de fonte para
     incorporar (o texto é desenhado por uma fonte substituta, ou a fonte está
     num arquivo de coleção).
   - Para arquivos TraceDraw no computador, [[dialog.save_package]] salva o
     desenho em uma pasta com o nome dele, com as imagens vinculadas, os
     arquivos de fonte e um relatório, e [[package.pdf]] acrescenta um PDF.
     Veja [Empacotar um desenho](../file-formats/package.md).

   Essas escolhas são lembradas para a próxima vez.
6. Se quiser, preencha [[dialog.title]], [[dialog.subject]] e uma
   [[dialog.rating]] (clique de novo na última estrela acesa para tirá-la). São
   as propriedades do desenho, também mostradas em
   [Propriedades do documento](../pages-layout/document-properties.md).
7. Clique em [[dialog.save]] ou pressione <kbd>Enter</kbd>. Se o arquivo já
   existir, o TraceDraw pergunta se deve [[dialog.replace]].

A caixa de diálogo começa na pasta do desenho, senão na última pasta em que
você salvou, senão na sua pasta de documentos. Um desenho aberto de um arquivo
`.cdr` ou de outro formato começa na pasta desse arquivo, com o nome dele, então
salvar no mesmo formato o substitui.

Na lista de arquivos, [[dialog.new_folder]] cria uma pasta e já deixa você
digitar o nome; <kbd>F2</kbd> ou um clique com o botão direito renomeia uma
pasta ou arquivo; digitar letras pula para os itens que começam com elas, e
<kbd>Enter</kbd> abre a pasta escolhida. Uma pasta digitada na barra de caminho
é usada sem precisar pressionar <kbd>Enter</kbd>.

> **Observação:** Na versão para navegador, todo salvamento é um download: a
> caixa de diálogo guarda o nome, o tipo, a versão e as propriedades, e o
> navegador coloca o arquivo na pasta de downloads.

Outras formas de salvar: [[menu.file > menu.file.save_as_template]] salva um
modelo (veja [Modelos](../templates-styles/templates.md)),
[[menu.file.export]] grava outros formatos (veja
[Exportar](../file-formats/export.md)), [[menu.file.publish_to_pdf]] grava um
PDF (veja [Publicar em PDF](../file-formats/publish-pdf.md)),
[[menu.file.send_to]] salva um PDF na área de trabalho ou na pasta de
documentos ou o anexa a um e-mail, [[menu.file.package]] reúne o desenho, suas
imagens e suas fontes em uma pasta (veja
[Empacotar um desenho](../file-formats/package.md)), e [[menu.file.github]]
guarda desenhos em um repositório do GitHub (veja [GitHub](github.md)).
[[menu.file.revert]] volta à versão salva por último.

## Fechar desenhos {#closing}

Escolha [[menu.file > menu.file.close]] (<kbd>Ctrl+F4</kbd>), clique no botão
de fechar da guia do desenho ou clique na guia com o botão do meio.
[[menu.file.close_all]] fecha todos os desenhos, e [[menu.file.exit]]
(<kbd>Alt+F4</kbd>) fecha o TraceDraw.

Quando um desenho tem alterações não salvas, o TraceDraw pergunta se deve
salvá-las antes: [[dialog.yes]] salva (pedindo um nome se o desenho ainda não
tiver arquivo), [[dialog.no]] fecha sem salvar e [[dialog.cancel]] mantém o
desenho aberto. Quando vários desenhos têm alterações, ele pergunta sobre cada
um; Cancelar interrompe.

![A pergunta sobre alterações não salvas](../../img/get-started-open-and-save-closing.png)

## Backup automático e recuperação {#recovery}

O TraceDraw guarda cópias de segurança dos desenhos abertos
enquanto o backup automático estiver ativado (a página [[options.save]] de
[[menu.tools > menu.tools.options > menu.tools.options_app]] define a
frequência e o local; a cada 20 minutos de início). Fechar o TraceDraw
normalmente as remove.

Se o TraceDraw terminou sem fechar, por exemplo depois de uma queda de energia,
ele oferece as cópias na próxima vez que iniciar: [[dialog.recover]] abre cada
uma como um desenho não salvo (Salvar então pergunta onde salvá-lo), e
[[dialog.discard]] deixa as cópias na pasta. Duas janelas do TraceDraw abertas
ao mesmo tempo nunca oferecem os desenhos uma da outra. Quando o sistema fecha
o TraceDraw sem perguntar sobre desenhos não salvos (o Encerrar do menu do
macOS, sair da sessão) ou manda o app para Android para o segundo plano, as
cópias são gravadas na hora, e o próximo início as oferece.

Na versão para navegador as cópias ficam no armazenamento do navegador para o
site, numa pasta própria de cada aba. Fechar ou recarregar uma aba com
desenhos não salvos pergunta antes, e a próxima visita oferece o que a aba
deixou; [[dialog.discard]] apaga essas cópias.
