# Empacotar um desenho

> *Tradução automática: esta página pode estar atrasada em relação à versão em inglês.*

Um desenho com vínculos para arquivos de imagem e que usa fontes que outros
computadores talvez não tenham não viaja bem como um arquivo só. Um pacote
reúne o desenho e tudo de que ele precisa em uma pasta: o desenho, as imagens
vinculadas, os arquivos de fonte que o texto usa, um relatório e, se você
quiser, um PDF. Entregue a pasta a uma gráfica ou a um colega, ou guarde-a com
o trabalho pronto.

> **Observação:** Pacotes precisam da versão para computador. Na versão para
> navegador o comando de menu fica cinza e Salvar como não oferece a opção.

## Fazer um pacote {#make}

1. Escolha [[menu.file > menu.file.package]].
2. Na caixa de diálogo [[package.title]], confira a [[package.folder]]. É uma
   pasta nova com o nome do desenho, na pasta do arquivo do desenho (senão na
   última pasta usada, senão na sua pasta pessoal). [[publish.browse]] escolhe
   outro lugar para ela e mantém o nome da pasta.
3. Confira o [[dialog.file_name]] do desenho no pacote (o nome do desenho,
   como arquivo `.tdraw`).
4. Marque [[package.pdf]] para acrescentar um PDF do desenho (desligado de
   início).
5. Clique em [[dialog.ok]].

O TraceDraw cria a pasta quando ela não existe e substitui arquivos de mesmo
nome dentro dela. O desenho aberto não muda: os vínculos dele continuam
apontando para as imagens originais. A barra de status diz quantas imagens e
fontes entraram, e quantos itens não puderam entrar (o relatório os nomeia).

## Salvar como pacote {#save-as}

Você também pode empacotar um desenho ao salvá-lo:

1. Escolha [[menu.file > menu.file.save_as]].
2. Escolha TraceDraw (`*.tdraw`) em [[dialog.save_as_type]].
3. Marque [[dialog.save_package]] e, se quiser também um PDF, [[package.pdf]].
4. Digite o nome e clique em [[dialog.save]].

O desenho é salvo como `<nome>/<nome>.tdraw` na pasta escolhida, com as
imagens copiadas ao lado. Ao contrário de [[menu.file.package]], Salvar como
muda o desenho aberto: os vínculos passam a apontar para as cópias do pacote,
então dali em diante você trabalha no desenho do pacote.

## O que vai na pasta {#contents}

| Item | Onde | O que contém |
|---|---|---|
| O desenho | `<nome>.tdraw` | O desenho, com as fontes que ele já leva. |
| Imagens vinculadas | pasta `Pictures` | Cada imagem a que o desenho se vincula, também em grupos, molduras de recorte e símbolos, cada arquivo uma vez. Dois arquivos de mesmo nome ficam separados como `logo.png` e `logo-2.png`. |
| Fontes | pasta `Fonts` | Um arquivo de fonte para cada estilo que o texto usa, mas só fontes cuja licença permite acompanhar um documento. |
| Um PDF (opcional) | `<nome>.pdf` | Gravado com as escolhas feitas por último em Publicar em PDF, com a sangria do desenho. |
| Um relatório | `<nome> report.txt` | O que entrou e o que não pôde entrar: imagens não encontradas, fontes deixadas de fora pela licença e fontes não instaladas com a fonte que desenha o texto delas. |

Os vínculos do desenho no pacote apontam para as cópias com caminhos relativos
ao desenho (`Pictures/logo.png`), então você pode mover, copiar ou enviar a
pasta inteira e os vínculos continuam funcionando. Uma imagem cujo arquivo
sumiu mantém o vínculo antigo e é citada no relatório; os pixels dela estão no
desenho de qualquer forma, então ela continua aparecendo.

## Abrir um pacote {#open}

Abra o arquivo `.tdraw` do pacote como qualquer desenho. Quando a pasta do
desenho tem uma pasta `Fonts` ao lado, o TraceDraw usa as fontes dela nesta
sessão, então o texto aparece nas próprias fontes mesmo onde elas não estão
instaladas. Nada é instalado no computador.

> **Dica:** Vínculos relativos a um desenho são lidos a partir da pasta do
> desenho em toda parte, também quando uma imagem vinculada é atualizada a
> partir do arquivo, então um pacote pode ficar em qualquer lugar.

Para conferir o desenho antes de empacotar (fontes ausentes, imagens de baixa
resolução, cores), rode a [verificação prévia](../printing/preflight.md)
antes.
