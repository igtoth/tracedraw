# Formatos de arquivo suportados

## Salvar {#save}

| Formato | Observações |
|---|---|
| TraceDraw (`.tdraw`) | o formato nativo; guarda tudo, inclusive efeitos, estilos, comentários e fontes |
| CDR (`.cdr`) | versões 7 a 27 em Salvar como; guarda páginas, camadas, formas, curvas, grupos, texto, bitmaps, preenchimentos, contornos e transparência; os efeitos são gravados como objetos simples |
| Modelo do TraceDraw (`.tdt`) | em [[menu.file.save_as_template]] |

## Abrir e importar {#open}

| Formato | O que entra |
|---|---|
| TraceDraw (`.tdraw`, `.tdt`) | tudo |
| CDR (`.cdr`), das versões mais antigas às mais novas | páginas, camadas, formas, curvas, grupos, texto, bitmaps, preenchimentos, contornos, transparência, fontes incorporadas, o perfil de cor |
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
