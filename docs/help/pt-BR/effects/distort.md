# Distorção

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

A ferramenta [[tool.distort]] entorta o contorno de um objeto de três jeitos:
**empurrar e puxar** (pontas ou pétalas), **zíper** (dentes em zigue-zague) e
**torção** (um redemoinho). É um efeito vivo: você pode mudar, mover ou remover
a distorção a qualquer momento, e o objeto mantém o próprio contorno.

## Para distorcer um objeto {#create}

1. Escolha a ferramenta [[tool.distort]] no submenu Efeitos.
2. Escolha o tipo na barra de propriedades.
3. Arraste sobre o objeto. A distorção fica centrada onde você começa.
   - **Empurrar e puxar**: arraste para a direita para empurrar os nós para
     fora (formas pontudas com lados côncavos), para a esquerda para puxá-los
     para dentro (pétalas arredondadas).
   - **Zíper**: quanto mais você arrasta, mais altos os dentes.
   - **Torção**: arraste em volta do ponto inicial; voltas completas contam.

O centro de uma distorção selecionada aparece como um losango: arraste-o para
mover o centro. Arrastar de novo substitui a distorção; um objeto tem no máximo
uma. Grupos e bitmaps não são distorcidos.

## Configurações da distorção {#settings}

A barra de propriedades mostra e muda a distorção selecionada:

- o tipo: **Empurrar e puxar**, **Zíper** ou **Torção** (escolher outro tipo
  muda uma distorção selecionada);
- para empurrar e puxar, a **amplitude** (-200 a 200, 20 de início);
- para zíper, a **amplitude** (0 a 100 mm, 2 mm de início) e a **frequência**
  (dentes por segmento, 6 de início), com **Aleatório** (dentes desiguais),
  **Suave** (dentes arredondados) e **Local** (os dentes diminuem longe do
  centro);
- para torção, **horário** ou **anti-horário**, o número de **rotações**
  inteiras e os **graus** extras.

Um valor mudado na barra de propriedades aparece no objeto selecionado na
hora, mesmo quando ele ainda não tem distorção. O zíper desenha uma linha a
mais que a frequência, e a amplitude, a frequência e o ângulo dele têm alças na
página para arrastar. O redemoinho gira suavemente até as pontas da forma.

[[menu.object > menu.object.clear_effect]] remove a distorção.
