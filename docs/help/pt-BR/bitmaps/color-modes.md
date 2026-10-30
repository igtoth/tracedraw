# Mudar o modo de cor de bitmaps

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

[[menu.bitmaps > menu.bitmaps.mode]] converte os bitmaps selecionados para outro
modo de cor. Todos os bitmaps selecionados mudam em um passo de desfazer, e a
transparência é mantida. Os modos com configurações abrem uma caixa de diálogo
com prévias de antes e depois.

| Modo | Resultado |
|---|---|
| [[menu.bitmaps.mode_bw]] | só preto e branco (veja abaixo) |
| [[menu.bitmaps.mode_grayscale]] | tons de cinza |
| [[menu.bitmaps.mode_duotone]] | de uma a quatro tintas (veja abaixo) |
| [[menu.bitmaps.mode_paletted]] | no máximo 256 cores (veja abaixo) |
| [[menu.bitmaps.mode_rgb]] | o modo do bitmap passa a ser RGB; os pixels ficam como estão |
| [[menu.bitmaps.mode_lab]] | cor Lab |
| [[menu.bitmaps.mode_cmyk]] | o modo do bitmap passa a ser CMYK e seus pixels mostram as cores que as tintas conseguem imprimir, calculadas com os perfis do gerenciamento de cores |

O menu marca [[menu.bitmaps.mode_rgb]] ou [[menu.bitmaps.mode_cmyk]] quando todos os bitmaps selecionados estão nesse modo, e a barra de status mostra o modo de um bitmap selecionado, como "Bitmap 600 x 400 px (CMYK)". Uma imagem CMYK de um arquivo `.cdr` abre no modo CMYK, e os arquivos `.tdraw` guardam o modo. [[menu.tools > menu.tools.convert_colors]] define do mesmo jeito o modo de cada bitmap que converte (veja [Converter cores](../color/convert-colors.md)).

## Preto e branco {#black-and-white}

![A caixa de diálogo Preto e branco](../../img/bitmaps-color-modes-black-and-white.png)

Escolha o método de conversão:

- **Arte de linha**: branco onde o cinza está no **limiar** ou acima (128 de
  início), preto no resto;
- **Ordenado**: um padrão regular de pontos;
- **Meio-tom** (de início): uma retícula de pontos redondos, linhas ou
  quadrados em um **ângulo** (45 graus) e um número de **linhas por
  polegada** (60);
- **Distribuição por cardinalidade**: uma textura de aparência aleatória;
- **Jarvis**, **Stucki** e **Floyd-Steinberg**: difusão de erro, os resultados
  mais suaves para fotos.

**Intensidade** (0 a 100, 50 de início) deixa o resultado mais claro ou mais
escuro.

## Duotom {#duotone}

![A caixa de diálogo Duotom](../../img/bitmaps-color-modes-duotone.png)

Um duotom imprime uma imagem em tons de cinza com poucas tintas, o que dá tons
ricos com poucas cores.

1. Escolha o tipo: Monotom, Duotom (de início), Tritom ou Quadritom, para uma a
   quatro tintas.
2. Clique em uma tinta para mudar a cor dela.
3. Edite a curva: na horizontal fica o nível de cinza (preto à esquerda), na
   vertical quanto dessa tinta é impresso. Arraste em um espaço vazio para
   acrescentar um ponto, arraste um ponto para movê-lo, clique duas vezes nele
   ou arraste-o para fora da grade para removê-lo. **Mostrar todas** desenha
   também as curvas das outras tintas.
4. **Salvar** e **Carregar** guardam as tintas e curvas em um arquivo.

## Paleta {#paletted}

![A caixa de diálogo Paleta](../../img/bitmaps-color-modes-paletted.png)

- **Paleta**: Cores uniformes, VGA padrão (16 cores), Adaptável e Otimizada
  (as cores da própria imagem, reduzidas ao número de **cores** que você
  definir, até 256), Tons de cinza, Sistema ou Personalizada (a paleta do
  documento).
- **Pontilhamento**: Nenhum, Ordenado, Jarvis, Stucki ou Floyd-Steinberg, com
  a intensidade do pontilhamento.

A paleta resultante aparece abaixo das configurações.

## Colorir bitmaps preto e branco {#monochrome}

Com apenas bitmaps de duas cores selecionados, clique em uma cor da paleta para
colorir o fundo deles, e clique com o botão direito para colorir a frente.
