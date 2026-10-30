# Converter cores para CMYK ou RGB

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Um desenho muitas vezes mistura cores definidas em RGB (para telas) com cores
definidas em CMYK (para tintas de impressão). Antes de mandar um desenho para
impressão, ou de reaproveitar um trabalho impresso na web,
[[menu.tools > menu.tools.convert_colors]] leva todas as cores dos objetos
selecionados, ou do desenho inteiro, para um só modelo, em um passo.

## Converter as cores {#convert}

1. Para converter só alguns objetos, selecione-os antes.
2. Escolha [[menu.tools > menu.tools.convert_colors]].
3. Em [[convert_colors.to]], escolha [[convert_colors.cmyk]] (o padrão) ou
   [[convert_colors.rgb]].
4. Em [[convert_colors.which]], escolha [[convert_colors.selection]] (o padrão
   quando há algo selecionado) ou [[convert_colors.drawing]], que também
   inclui os fundos das páginas e os símbolos.
5. Marque [[convert_colors.spots]] para levar também as cores especiais para a
   mistura CMYK ou RGB delas (desligado de início).
6. Clique em [[dialog.ok]].

A barra de status diz quantas cores mudaram. Todas as mudanças se desfazem em
um passo. Quando todas as cores já estão no modelo escolhido, ou nada está
selecionado, nada muda.

## O que é convertido {#what}

- Preenchimentos sólidos, as paradas dos gradientes, as cores de malhas,
  padrões e texturas.
- Contornos, texto e células de tabela.
- As cores dos efeitos (contornos, misturas, extrusões, chanfros, sombras em
  bloco e os outros) e das sombras projetadas.
- Objetos dentro de grupos e molduras de recorte, em qualquer profundidade.
- Bitmaps recebem o modo de cor: convertidos para CMYK, os pixels mudam para
  mostrar as cores que as tintas conseguem imprimir; convertidos para RGB, só
  o modo muda e os pixels ficam. Veja
  [Modos de cor de bitmaps](../bitmaps/color-modes.md).

A cor de registro nunca muda, então as marcas continuam saindo em todas as
chapas. Cores especiais continuam especiais a não ser que você marque
[[convert_colors.spots]]. Uma cor que já está no modelo fica como está.

## Como as cores são calculadas {#profiles}

A conversão passa pelos perfis do gerenciamento de cores: o próprio perfil
CMYK do desenho quando ele tem um, senão os perfis RGB e CMYK, a tentativa de
renderização e a opção de ponto preto de
[[menu.tools > menu.tools.color_management]]. Veja
[Gerenciamento de cores](color-management.md).

> **Dica:** Uma cor convertida para CMYK e de volta para RGB não é a cor do
> começo quando era viva demais para as tintas. Guarde uma cópia do desenho
> antes de converter um desenho inteiro.
