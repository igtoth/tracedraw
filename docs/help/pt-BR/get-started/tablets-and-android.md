# Tablets e o app para Android

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

O TraceDraw roda em tablets de dois jeitos: a versão para navegador instalada
como app (num tablet Android, num iPad ou num computador) e um app nativo
para tablets, celulares e Chromebooks com Android. Os dois são o mesmo editor
da versão para computador, com os mesmos menus, ferramentas e arquivos.

## Instalar a versão para navegador como app {#install}

A [versão para navegador](https://igtoth.github.io/tracedraw/) pode ser
instalada pelo navegador. Ela então abre em tela cheia com o ícone do
TraceDraw, sem a barra de endereço e as guias do navegador, em qualquer
orientação, e evita o entalhe ou os cantos arredondados da tela.

- **Tablet Android** (Chrome, Edge, Samsung Internet): abra a página, depois
  o menu do navegador e **Instalar app** ou **Adicionar à tela inicial**. O
  Chrome também pode oferecer isso numa barra abaixo da barra de endereço.
- **iPad** (Safari): abra a página, toque em **Compartilhar** e depois em
  **Adicionar à Tela de Início**. No iPad e no iPhone, como num tablet Android, as caixas
  Abrir e Importar mostram todos os arquivos, porque o sistema deixaria os
  arquivos `.cdr` cinza; escolha um desenho que o TraceDraw abre.
- **Computador** (Chrome, Edge): clique no ícone de instalar na ponta direita
  da barra de endereço, ou escolha **Instalar TraceDraw** no menu do
  navegador.

Depois de aberto uma vez, o app instalado inicia sem conexão de rede, na
versão usada por último (fontes baixadas da web ainda precisam de conexão na
primeira vez que seu texto as usa). Uma versão nova é usada na próxima vez que
o app iniciar depois que todas as suas páginas e janelas forem fechadas.

É o mesmo programa da versão para navegador, então valem as diferenças
listadas em [A versão para navegador](browser-version.md#differences).

## O app para Android {#android}

O app para Android é o TraceDraw feito para tablets e celulares ARM de 64 bits
e para Chromebooks com processadores Intel ou AMD. Os arquivos de instalação
(APK) acompanham as versões e as compilações noturnas do projeto no GitHub. O
app não procura atualizações sozinho; [[menu.help > menu.help.updates]] abre a
página dele na loja.

Uma instalação nova começa no espaço de trabalho
[[menu.window.workspace_touch]]: barras e botões maiores, sem réguas, painéis
ou barra de status; dois dedos movem a vista e um gesto de pinça aproxima.
[[menu.window > menu.window.workspace]] escolhe outro, e a escolha fica
guardada. Veja [Espaços de trabalho](../customize/workspaces.md).

O que funciona diferente:

- **Arquivos.** Abrir, Importar, Salvar, Salvar como, Exportar e Publicar em
  PDF passam pelo seletor de documentos do sistema. Salvar grava de volta no
  lugar um documento aberto. A caixa de diálogo [[dialog.save_drawing]] pede
  só o nome, o tipo e a versão; depois o sistema pergunta onde salvar.
- **Desenhos recentes** reabrem pela lista e pela Tela de boas-vindas, como no
  computador.
- **Área de transferência.** O texto passa pela área de transferência do
  sistema: objetos copiados no TraceDraw são colados em outros apps como
  texto, e texto copiado em outro lugar é colado como texto. Imagens copiadas
  em outros apps ainda não são lidas.
- **Fontes.** São usadas as fontes do sistema.
- **Teclado.** Um teclado físico ou Bluetooth funciona com os atalhos.
- As páginas da **Ajuda** abrem no navegador.

Não disponível no Android: imprimir, Enviar para, editar um bitmap em outro
programa e escolher uma pasta (como uma pasta de exportação ou de fontes).

## Toque e caneta {#touch}

Numa tela sensível ao toque, um dedo ou uma caneta operam a tela, a caixa de
ferramentas, as barras e os menus. Toque para clicar, arraste para desenhar ou
mover. No espaço de trabalho [[menu.window.workspace_touch]], dois dedos
movem a vista e a pinça aproxima. Para a pressão da caneta em sistemas de
computador, veja
[Desenhar com uma mesa digitalizadora](../lines-shapes/pen-pressure.md).
