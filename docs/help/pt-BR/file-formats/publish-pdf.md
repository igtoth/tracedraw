# Publicar em PDF

[[menu.file > menu.file.publish_to_pdf]] (também na barra de ferramentas padrão)
grava um arquivo PDF com controle total do que vai dentro: quais páginas, qual
padrão PDF, como as imagens são comprimidas, informações do documento,
marcadores, marcas de impressão e proteção por senha. As guias são
[[publish.tab_general]], [[publish.tab_color]], [[publish.tab_document]],
[[publish.tab_objects]], [[publish.tab_prepress]], [[publish.tab_security]] e
a guia de problemas.

![A caixa de diálogo Configurações de PDF](../../img/file-formats-publish-pdf.png)

## Geral {#general}

- **Nome do arquivo**, com **Procurar** para escolher outro lugar.
- **Intervalo de exportação**: o documento atual (todas as páginas), todos os
  documentos abertos, a página atual, uma lista de páginas (como `1-3, 5`) ou a
  seleção.
- **Tamanho da página** para uma seleção: [[publish.size_document]] (a página,
  só com os objetos selecionados) ou [[publish.size_selection]] (uma página do
  tamanho dos objetos selecionados, o padrão).
- **Predefinição de PDF**: [[publish.preset_distribution]],
  [[publish.preset_web]], [[publish.preset_prepress]], [[publish.preset_x1a]],
  [[publish.preset_x3]] ou uma sua; **+** salva as configurações atuais como
  predefinição com o nome que você digitar (o mesmo nome a substitui) e **-**
  exclui a escolhida. Uma predefinição define a compatibilidade, a condição de
  saída, a resolução de achatamento, os marcadores, a sangria, as marcas de
  impressão e as opções de bitmaps.
- **Compatibilidade**: PDF 1.4, PDF/X-1a:2003, PDF/X-3:2003 ou PDF/X-4.
  Arquivos PDF/X são feitos para gráficas.

## Cor {#color}

Para PDF/X, a **condição de saída** (FOGRA39 de início); o perfil CMYK do
desenho vai para o arquivo. Um PDF comum mantém as cores como estão.

## Documento {#document}

**Título**, **Autor**, **Assunto** e **Palavras-chave**, preenchidos a partir
das propriedades do desenho; **um marcador para cada página** (ligado de
início) e **mostrar os marcadores quando o arquivo abre**.

## Objetos {#objects}

- [[publish.flatten_resolution]] uma resolução (300 dpi), para PDF/X-1a e X-3,
  que não permitem transparência. Os outros padrões mantêm a transparência
  viva, por isso o campo fica cinza para eles.

### Bitmaps {#bitmaps}

O grupo [[publish.bitmaps]] define como as imagens entram no arquivo. As
escolhas ficam guardadas para a próxima vez e nas predefinições.

- [[publish.downsample_color]] (desligado, 300 dpi): uma imagem colorida com
  mais pixels do que essa resolução, do jeito que está colocada na página, é
  reduzida para ela.
- [[publish.downsample_gray]] (desligado, 300 dpi): o mesmo para imagens só
  de cinzas. Imagens cinza sempre entram com um canal, o que as deixa
  pequenas.
- [[publish.downsample_mono]] (desligado, 1200 dpi): o mesmo para imagens só
  de pixels pretos e brancos.
- [[publish.compression]]: [[publish.compression_zip]] (o padrão) mantém cada
  pixel; [[publish.compression_jpeg]] faz arquivos menores com alguma perda,
  na [[publish.jpeg_quality]] que você definir (1 a 100, 85 de início).
  Imagens em preto e branco, máscaras de transparência e arquivos PDF/X-1a
  sempre usam ZIP.
- [[publish.compress_content]] (ligado) comprime o texto e o desenho das
  páginas.

Uma imagem totalmente fora da página e da sangria não pode aparecer, então
fica fora do arquivo.

A predefinição [[publish.preset_web]] reduz imagens coloridas e cinza para
150 dpi e as de preto e branco para 300 dpi, com JPEG em qualidade 75.

## Pré-impressão {#prepress}

- [[dialog.bleed]] (0 a 50 mm, a sangria do desenho de início) em volta de
  cada página: a caixa de corte é a página, a caixa de mídia a página e a
  sangria.
- [[publish.marks]]: [[dialog.crop_marks]], [[dialog.registration_marks]],
  [[print.color_bars]] e [[publish.file_info]] em volta de cada página. Veja
  [Marcas de impressão em arquivos PDF](pdf-prepress-marks.md).

## Texto e fontes {#text}

O texto vai para o PDF como texto de verdade: num visualizador de PDF é
possível pesquisá-lo, selecioná-lo e copiá-lo, e ele mantém o preenchimento,
o contorno, as cores e o ângulo. As fontes também vão para o arquivo, então o
texto fica igual em qualquer computador. No grupo [[publish.text_fonts]] da
aba Objetos:

- **[[publish.text_as_curves]]** (desligado no início) grava todo o texto como
  curvas: o arquivo não tem fontes e o texto não pode ser pesquisado. As
  curvas seguem exatamente o contorno das próprias letras, então continuam
  lisas em qualquer zoom.
- **[[publish.embed_fonts]]** (ligado no início) coloca as fontes no arquivo.
  Desligado, as fontes são apenas nomeadas e o visualizador usa a sua cópia.
- **[[publish.embed_base14]]** coloca também Helvetica, Times e Courier no
  arquivo; caso contrário, as fontes padrão do visualizador as desenham.
- **[[publish.subset_fonts]]** coloca apenas as letras que o arquivo usa, o
  que o deixa pequeno, quando uma fonte usa menos que a parcela escolhida
  (80 %).

Arquivos PDF/X sempre levam as suas fontes. Algumas fontes não permitem ser
colocadas em outros arquivos: o texto delas é gravado como curvas, a barra de
status as nomeia depois de publicar e a verificação prévia lista esse texto
antes.

## Segurança {#security}

- **Exigir senha para abrir o arquivo**, e a senha.
- **Restringir impressão, cópia e alterações com senha**, e a senha, depois as
  **permissões**: impressão (não permitida, só baixa resolução, permitida),
  cópia, alteração do conteúdo, comentários, preenchimento de formulários,
  gerenciamento de páginas e acesso ao texto para leitores de tela.
- **Mostrar senhas**.

Arquivos PDF/X não podem ser protegidos. As senhas nunca são guardadas.

## Problemas {#issues}

A última aba lista o que a verificação prévia encontra; veja
[Verificação prévia](../printing/preflight.md). Clique em [[dialog.ok]] para
gravar o arquivo. Se faltar algo, como uma senha ou uma página na lista, a
caixa de diálogo avisa em vermelho.

## Abrir arquivos PDF protegidos {#password}

Quando você abre ou importa um PDF que precisa de senha, a caixa de diálogo
[[pdf_password.title]] a pede. Digite a [[publish.password]] e clique em
[[dialog.ok]]. Uma senha errada é recusada e você pode tentar de novo;
[[dialog.cancel]] deixa o arquivo fechado.

![A caixa de diálogo PDF protegido](../../img/file-formats-publish-pdf-password.png)
