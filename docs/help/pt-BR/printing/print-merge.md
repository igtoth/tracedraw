# Mesclagem de impressão

A mesclagem de impressão preenche um desenho com dados, para fazer muitas
versões dele: crachás, ingressos numerados, certificados, etiquetas com códigos
QR. Cada registro dos seus dados vira uma cópia da página, preenchida.

## Configurar uma mesclagem {#setup}

1. Desenhe a página. Onde vai um valor, digite o nome do campo entre sinais de
   menor e maior em um objeto de texto, como `<Name>`.
2. Escolha [[menu.file > menu.file.print_merge > menu.file.print_merge_create]].
3. Acrescente os dados:
   - **Importar arquivo** lê um arquivo CSV (separado por vírgulas,
     ponto e vírgula ou tabulações, como o Excel ou outro programa o
     salva) ou uma pasta do Excel (`.xlsx`, a primeira planilha) cuja
     primeira linha nomeia os campos. Campos com
     nome de foto ou imagem, ou cujas células são arquivos de imagem,
     começam como imagens, campos com QR no nome como códigos QR; os tipos
     aparecem logo para serem mudados;
   - ou **Adicionar coluna** para acrescentar campos à mão, depois acrescente
     registros com **+**.
4. Clique em **Concluir**. Abre o painel [[docker.print_merge]], onde você
   põe os campos na página e faz a mesclagem.

![A caixa de diálogo Mesclagem de impressão](../../img/printing-print-merge.png)

## O painel Mala direta {#docker}

O painel [[docker.print_merge]] (também em Janela > Painéis) trabalha com a
fonte de dados que você criou ou carregou:

- **[[merge.create]]** e **[[merge.edit]]** abrem os registros.
- Escolha o tipo de campo, **Texto**, **Imagem** ou **Código QR**, e a coluna
  em **[[merge.columns]]**.
- Para uma imagem, **[[merge.pictures_folder]]** escolhe a pasta onde estão as
  fotos, quando os dados trazem só o nome dos arquivos (um nome sem extensão,
  como `1024`, encontra `1024.jpg` ou `1024.png`). **[[merge.placeholder]]** escolhe uma imagem que representa
  as mescladas enquanto você desenha, e **[[merge.align_scale]]** define onde
  cada imagem fica na moldura: clique em um dos nove pontos para ancorá-la e
  escolha [[merge.scale_actual]], [[merge.scale_fit]], [[merge.scale_fill]]
  (a moldura corta o que sobra) ou [[merge.scale_stretch]].
- **[[merge.insert_field]]** põe o campo no texto que você está editando, ou
  cria um texto, uma moldura de imagem ou um quadrado de QR no meio da
  visualização. Mova e redimensione como qualquer objeto.
- Para colocar cada foto dentro de uma forma sua (um círculo, um oval,
  qualquer contorno, também dentro de um cartão agrupado), escolha Imagem e a
  coluna da foto, selecione a forma e clique em **[[merge.place_inside]]**. A
  foto de cada cartão vai para dentro dessa forma, conforme o alinhamento e a
  escala.
- Selecione um campo e clique em **[[merge.update_field]]** para mudar a coluna
  ou o alinhamento e a escala.
- **[[merge.create_document]]** cria as páginas mescladas em um desenho novo: a
  página em que você está é copiada uma vez por registro e cada cópia é
  preenchida. O seu desenho fica como está. **[[merge.print_document]]** faz o
  mesmo e abre Imprimir.

[[menu.file.print_merge_perform]] em vez disso acrescenta as cópias
preenchidas ao desenho atual, logo depois da página, com o nome da página e o
número do registro.

## Tipos de campo {#fields}

- **Texto**: substitui `<Field>` em todos os textos da página.
- **Número**: um contador que você não digita: um início, um passo e um número
  de dígitos (início 7, passo 3, dígitos 3 dá 007, 010, 013).
- **Imagem**: a célula tem o caminho de uma imagem. Um texto que contém apenas
  `<Field>` vira a imagem, do maior tamanho que caiba na caixa dele. Ponha esse
  texto dentro de uma moldura com [[menu.object.clip_frame_place_inside]] (uma moldura de
  foto) e a imagem preenche a moldura, cortada por ela. Um retângulo, elipse ou
  curva com o nome `<Field>` (no painel Objetos) recebe a imagem dentro dele do
  mesmo jeito.

Os campos funcionam também em grupos e molduras, então um cartão pode estar
agrupado.
- **Código QR**: um texto que contém apenas `<Field>` vira um código QR do
  texto da célula.

Uma célula vazia, ou uma imagem que não carrega, remove esse objeto da página
do registro.

## Editar os registros {#records}

**Configurar colunas** renomeia campos, muda o tipo e as configurações do
contador e remove campos. **Ver um registro** mostra um registro como
formulário; use as setas ou **Ir para o registro** para percorrê-los. Os
botões de marcador inserem um campo no texto que você está editando. **Salvar
fonte de dados** grava os registros como arquivo CSV.

Para trabalhar em várias células de uma vez, arraste de uma célula a outra, ou
sobre os números dos registros ou os nomes dos campos para pegar registros ou
campos inteiros. Então **[[merge.sel_add]]** adiciona o mesmo número de
registros vazios depois deles, **[[merge.sel_delete]]** remove os registros e
**[[merge.sel_clear]]** (ou a tecla Delete) esvazia as células.

[[menu.file.print_merge_edit]] reabre os registros.
