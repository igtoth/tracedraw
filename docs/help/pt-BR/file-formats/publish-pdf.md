# Publicar em PDF

[[menu.file > menu.file.publish_to_pdf]] (também na barra de ferramentas padrão)
grava um arquivo PDF com controle total do que vai dentro: quais páginas, qual
padrão PDF, informações do documento, marcadores e proteção por senha.

![A caixa de diálogo Configurações de PDF](../../img/file-formats-publish-pdf.png)

## Geral {#general}

- **Nome do arquivo**, com **Procurar** para escolher outro lugar.
- **Intervalo de exportação**: o documento atual (todas as páginas), todos os
  documentos abertos, a página atual, uma lista de páginas (como `1-3, 5`) ou a
  seleção.
- **Tamanho da página** para uma seleção: a página do documento, ou uma página
  do tamanho dos objetos selecionados.
- **Predefinição de PDF**: Distribuição de documentos, Web, Pré-impressão
  (PDF/X-4), PDF/X-1a, PDF/X-3 ou uma sua; **+** salva as configurações atuais
  como predefinição e **-** exclui uma.
- **Compatibilidade**: PDF 1.4, PDF/X-1a:2003, PDF/X-3:2003 ou PDF/X-4.
  Arquivos PDF/X são feitos para gráficas.

## Cor {#color}

Para PDF/X, a **condição de saída** (FOGRA39 de início); o perfil CMYK do
desenho vai para o arquivo. Um PDF comum mantém as cores como estão.

## Documento {#document}

**Título**, **Autor**, **Assunto** e **Palavras-chave**, preenchidos a partir
das propriedades do desenho; **um marcador para cada página** (ligado de
início) e **mostrar os marcadores quando o arquivo abre**.

## Objetos e pré-impressão {#objects}

- **Achatar a transparência em** uma resolução (300 dpi), para PDF/X-1a e X-3,
  que não permitem transparência.
- **Sangria** (0 a 50 mm) em volta de cada página.

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
