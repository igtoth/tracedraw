# Formatos de arquivo suportados

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

## Salvar {#save}

| Formato | Observações |
|---|---|
| TraceDraw (`.tdraw`) | o formato nativo; guarda tudo, inclusive efeitos, estilos, comentários e fontes |
| CDR (`.cdr`) | versões 7 a 27 em Salvar como; guarda páginas e seus tamanhos, camadas, formas, curvas, grupos e seus nomes, texto, bitmaps, preenchimentos, contornos e transparência, e as fontes com [[dialog.embed_fonts]]. A partir da versão 14, misturas, contornos, distorções, envelopes, perspectivas, extrusões, chanfros, sombras em bloco e simetrias reabrem no TraceDraw como efeitos vivos; outros programas veem os objetos que os efeitos desenharam. Sombras projetadas são gravadas como uma imagem da sombra sob o objeto |
| Modelo do TraceDraw (`.tdt`) | em [[menu.file.save_as_template]] |
| Um pacote (uma pasta) | em [[menu.file.package]]: o desenho, as imagens vinculadas, as fontes e um relatório; veja [Empacotar um desenho](package.md) |

## Abrir e importar {#open}

| Formato | O que entra |
|---|---|
| TraceDraw (`.tdraw`, `.tdt`) | tudo |
| CDR (`.cdr`), das versões mais antigas às mais novas | páginas, camadas, formas, curvas, grupos, texto, bitmaps (imagens CMYK no modo CMYK), preenchimentos incluindo malhas, contornos, transparência, misturas, objetos incorporados desenhados como imagens (como códigos de barras), fontes incorporadas, o perfil de cor |
| CMX | páginas, camadas, formas, preenchimentos, contornos, imagens, recortes, texto |
| SVG, SVGZ | formas, grupos, recortes, degradês, imagens, texto |
| PDF, AI | caminhos, imagens, texto, recortes, sombreamentos, padrões; arquivos protegidos por senha pedem a senha |
| EPS, PS, PRN, arquivos AI antigos | caminhos, imagens, texto, recortes, sombreamentos; uma página por página impressa |
| DXF | linhas, polilinhas, arcos, elipses, splines, hachuras, texto, blocos |
| EMF, WMF | caminhos, canetas, pincéis, texto, bitmaps, recortes |
| PLT (HPGL) | traços de caneta de plotter |
| PSD, PSB | as camadas como bitmaps, com opacidade, visibilidade e máscaras |
| PNG, JPEG, BMP, GIF, WebP, TIFF | bitmaps |
| TXT, RTF, DOCX | texto de parágrafo com a formatação |

## Exportar {#export}

| Formato | Tipo |
|---|---|
| SVG, HTML | vetorial, para a web |
| PDF, AI | vetorial, para impressão e compartilhamento; PDF/X-1a, X-3 e X-4 |
| EPS, PostScript (`.ps`), PRN | vetorial, para impressão |
| DXF, PLT | vetorial, para CAD, plotters e plotters de recorte |
| EMF, WMF | vetorial, para programas de escritório |
| PNG, JPEG, WebP, GIF, BMP, TIFF | bitmap |
| PSD | bitmap, uma camada por camada do desenho |

Scripts podem acrescentar mais formatos para importar e exportar; veja
[Scripts](../customize/scripts.md#filters).
