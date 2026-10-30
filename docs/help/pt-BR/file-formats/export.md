# Exportar

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

[[menu.file > menu.file.export]] (<kbd>Ctrl+E</kbd>) grava o desenho, uma página
ou a seleção em outro formato. A caixa de diálogo Exportar mostra uma prévia à
esquerda e as configurações à direita.

![A caixa de diálogo Exportar](../../img/file-formats-export.png)

## Para exportar {#export}

1. Escolha [[menu.file > menu.file.export]].
2. Escolha uma **Predefinição** para começar de escolhas salvas, ou escolha o
   **formato**. Os formatos mais comuns (JPG, PDF, AI, PNG) vêm primeiro.
   Escolher um formato traz de volta as configurações usadas nele da última
   vez.
3. Defina as opções do formato (abaixo).
4. Marque **Somente selecionados** para exportar só a seleção, ou **Todas as
   páginas** para exportar cada página.
5. Clique em [[dialog.ok]] e escolha onde salvar. No navegador, o arquivo é
   baixado.

A caixa de diálogo estima o tamanho do arquivo e, para formatos de bitmap, diz
o que sai, como "1240 x 1754 pixels, 150 dpi, sRGB". Com **Não mostrar a caixa de
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

## Tamanho dos bitmaps {#size}

Para formatos de bitmap, **Tamanho** tem uma largura, uma altura e a unidade
delas (px, mm, cm ou pol); o outro lado segue as proporções da página.

- Em pixels, digitar uma largura ou altura define a resolução que a produz,
  então você digita direto o tamanho de um ícone (32 x 32 pixels).
- Em comprimento, o tamanho é o tamanho impresso: digitar uma largura ou
  altura muda a escala da arte na resolução definida, de 1 % a 10000 %. Um
  botão com a escala (como "200 %") aparece quando ela não é 100 % e a volta.

Arquivos PNG, JPEG e BMP guardam a resolução, então abrem no tamanho escolhido.
Arquivos PostScript e PRN guardam todas as páginas exportadas. Para PDF,
PostScript e PRN, a opção de exportar todas as páginas começa marcada;
desmarque-a para exportar só a página atual.

## Predefinições {#presets}

A lista **Predefinição** tem todas as predefinições que você salvou, cada uma
com o formato entre parênteses, como "Web PNG (PNG)". Escolher uma predefinição
de outro formato muda a caixa de diálogo para esse formato; mudar o formato
deixa nenhuma predefinição escolhida. **+** salva as escolhas atuais com um
nome (o mesmo nome substitui aquela predefinição), o lápis renomeia a
escolhida e **-** a exclui.

## Escolhas iniciais {#defaults}

As escolhas iniciais são definidas na página [[options.file_formats]] das opções
globais, onde **Lembrar as configurações da última exportação** (ligado de
início) faz a caixa de diálogo começar com o formato, a resolução e a qualidade
da última exportação, também na sessão seguinte. [[menu.file.export_for]] tem predefinições para a web e para
documentos de escritório; veja
[Exportar para a web](../web-graphics/export-for-web.md). Para arquivos PDF com
marcadores e segurança, use [Publicar em PDF](publish-pdf.md).
