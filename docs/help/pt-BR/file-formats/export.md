# Exportar

[[menu.file > menu.file.export]] (<kbd>Ctrl+E</kbd>) grava o desenho, uma página
ou a seleção em outro formato. A caixa de diálogo Exportar mostra uma prévia à
esquerda e as configurações à direita.

![A caixa de diálogo Exportar](../../img/file-formats-export.png)

## Para exportar {#export}

1. Escolha [[menu.file > menu.file.export]].
2. Escolha o **formato**. Os mais comuns (JPG, PDF, AI, PNG) vêm primeiro.
3. Defina as opções do formato (abaixo).
4. Marque **Somente selecionados** para exportar só a seleção, ou **Todas as
   páginas** para exportar cada página.
5. Clique em [[dialog.ok]] e escolha onde salvar. No navegador, o arquivo é
   baixado.

A caixa de diálogo estima o tamanho do arquivo. Com **Não mostrar a caixa de
filtro** marcado, a próxima exportação vai direto para o arquivo com as mesmas
escolhas.

## Opções dos formatos {#options}

| Formato | Opções |
|---|---|
| PNG, WebP, GIF | resolução (ou o tamanho em pixels), fundo transparente; WebP também qualidade |
| JPEG | resolução, qualidade (1 a 100) |
| BMP, TIFF, PSD | resolução |
| PDF | padrão (PDF 1.4, PDF/X-1a, PDF/X-3 ou PDF/X-4), sangria, condição de saída para PDF/X |
| PDF, AI | título e etiquetas |
| SVG, EPS, AI, DXF, HTML, EMF, WMF, PLT, PS, PRN | nenhuma |

Para formatos de bitmap você pode digitar a largura ou a altura em pixels; a
resolução acompanha. Arquivos PostScript e PRN guardam todas as páginas
exportadas.

As escolhas iniciais são definidas na página [[options.file_formats]] das opções
globais. [[menu.file.export_for]] tem predefinições para a web e para
documentos de escritório; veja
[Exportar para a web](../web-graphics/export-for-web.md). Para arquivos PDF com
marcadores e segurança, use [Publicar em PDF](publish-pdf.md).
