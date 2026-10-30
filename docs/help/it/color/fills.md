# Riempimenti

Un riempimento colora l'interno di un oggetto chiuso. TraceDraw ha sei tipi di
riempimento: [[docker.fill_none]], [[docker.fill_uniform]] (un unico colore
piatto), [[docker.fill_fountain]] (colori che sfumano l'uno nell'altro),
[[docker.fill_pattern]], [[docker.fill_texture]] e [[docker.fill_mesh]]
(colori che scorrono tra i nodi di una griglia).

## Modi per riempire un oggetto {#ways}

- Fai clic su un colore della tavolozza per dare alla selezione un
  riempimento uniforme.
- Usa la sezione [[docker.fill]] del pannello [[docker.properties]]
  (<kbd>F11</kbd>): scegli il tipo di riempimento con la fila di immagini, poi
  le sue impostazioni.
- Usa lo strumento [[tool.interactive_fill]] (<kbd>G</kbd>) per impostare le
  sfumature direttamente sull'oggetto.
- Usa lo strumento [[tool.area_fill]] per riempire un'area racchiusa da più
  oggetti.
- Usa lo strumento [[tool.mesh_fill]] (<kbd>M</kbd>) per un riempimento a
  maglia.

Senza nulla di selezionato, il riempimento che scegli diventa il riempimento
dei nuovi oggetti.

## Riempimenti uniformi {#uniform}

Un riempimento uniforme è un unico colore, in qualsiasi modello di colore.
Sceglilo dalla tavolozza, dal pannello [[docker.color]] (vedi
[Il pannello Colore](color-docker.md)) o dalla sezione riempimento del
pannello [[docker.properties]].

## Riempimenti sfumati {#gradient}

Una sfumatura fonde due o più colori. Le sue impostazioni, nella sezione
riempimento del pannello [[docker.properties]]:

- Il tipo: [[fill.linear]] (lungo una linea), [[fill.radial]] (dal centro
  verso l'esterno), [[fill.conical]] (attorno al centro), [[fill.square]] (in
  quadrati crescenti) o [[fill.shaped]] (seguendo il contorno dell'oggetto,
  dal bordo verso l'interno).
- [[docker.gradient_presets]]: sfumature già pronte da cui partire.
- Le **interruzioni**: i colori e la loro posizione lungo la sfumatura, da 2 a
  64. Seleziona un'interruzione per cambiarne il colore o la posizione;
  [[docker.add_stop]] e [[docker.remove_stop]] aggiungono e rimuovono
  interruzioni. [[docker.import_gradient]] prende i colori da un file di
  sfumatura `.ggr`.
- L'**angolo** delle sfumature lineari, coniche e quadrate, il
  [[docker.edge_pad]] (quanta parte di ciascuna estremità è colore pieno) e lo
  spostamento del centro delle sfumature radiali, coniche e quadrate.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]] (i
  colori ripartono dall'inizio) o [[docker.repeat_mirror]] (vanno avanti e
  indietro), con il numero di [[docker.repeat_count]].
- [[docker.bilinear]]: i colori vanno dal primo all'ultimo e ritorno.
- [[docker.invert_colours]]: i colori vanno in ordine inverso.

Una striscia sotto il tipo mostra l'intera sequenza di colori come verrà
disegnata.

## La finestra di dialogo Modifica riempimento {#edit-fill}

<kbd>F11</kbd> apre la finestra di dialogo [[edit_fill.title]] sul riempimento
della selezione. Per una sfumatura mostra un'anteprima, il
[[edit_fill.fill_picker]], il tipo, la ripetizione e la banda dei colori: fate
clic su un nodo per sceglierlo, trascinatelo per spostarlo, fate doppio clic
per aggiungerne o toglierne uno. Sotto la banda: colore, trasparenza,
posizione e [[edit_fill.midpoint]] del nodo, e [[edit_fill.reverse]].
[[edit_fill.transform]] contiene rotazione, centro e margine del bordo. **OK**
applica ciò che avete cambiato, con [[edit_fill.overprint]]. Finché la finestra di dialogo è aperta, la selezione mostra il riempimento mentre lo modificate; **Annulla** o <kbd>Esc</kbd> ripristina il riempimento precedente.

![La finestra di dialogo Modifica riempimento](../../img/color-fills-edit-fill.png)

## Lo strumento Riempimento interattivo {#interactive-fill}

Lo strumento [[tool.interactive_fill]] (<kbd>G</kbd>) modifica i riempimenti
direttamente sull'oggetto.

1. Scegli lo strumento e trascina attraverso un oggetto: ottieni una sfumatura
   lineare dal punto in cui premi a quello in cui rilasci. Tieni premuto
   <kbd>Ctrl</kbd> per vincolare l'angolo a passi di 15 gradi. Un clic dà il
   riempimento predefinito.
2. Trascina i nodi sull'oggetto per cambiare la sfumatura: i due nodi quadrati
   di una sfumatura lineare ne cambiano direzione e lunghezza; il nodo di una
   sfumatura radiale, conica o quadrata ne sposta il centro.
3. Trascina un colore della tavolozza su un nodo per ricolorarlo, o sulla
   linea tra i nodi per aggiungere lì un colore.

La barra delle proprietà contiene il tipo di riempimento e le sue
impostazioni: il colore di un riempimento uniforme; per una sfumatura il
tipo, i colori iniziale e finale, l'angolo e il margine del bordo.

## Lo strumento Riempimento area {#area-fill}

Lo strumento [[tool.area_fill]] riempie qualsiasi area chiusa, anche una
formata da più oggetti sovrapposti, creando un nuovo oggetto di quell'area.
Gli oggetti intorno restano come sono.

1. Scegli lo strumento [[tool.area_fill]].
2. Scegli riempimento e contorno nella barra delle proprietà: **Specifica**
   (il colore e, per il contorno, lo spessore, accanto), **Usa predefinito**
   (il riempimento e il contorno dei nuovi oggetti; un colore quando quello è
   nessun riempimento, così l'area si vede) o **Nessuno**.
3. Fai clic all'interno dell'area.

Il nuovo oggetto va in cima al livello attivo e viene selezionato. Un clic su
un contorno, o in un'area che nulla racchiude, non crea nulla.

## Riempimenti a maglia {#mesh-fill}

Un riempimento a maglia è una griglia di nodi sopra un oggetto, ciascuno con
il proprio colore e la propria trasparenza; i colori scorrono in modo morbido
tra i nodi. È adatto a ombreggiature realistiche.

1. Scegli lo strumento [[tool.mesh_fill]] (<kbd>M</kbd>) e imposta righe e
   colonne nella barra delle proprietà (2 per 2 in partenza, fino a 50).
2. Fai clic su un oggetto. Riceve una maglia del colore del suo riempimento.
3. Fai clic su un nodo per selezionarlo, poi su un colore della tavolozza per
   colorarlo. Trascina i nodi per piegare il flusso dei colori.
4. Fai doppio clic all'interno della maglia per aggiungere una riga o una
   colonna passante per il nodo più vicino; seleziona un nodo e premi
   <kbd>Delete</kbd> per rimuoverne la riga e la colonna.

La barra delle proprietà contiene anche **Colore morbido**, che fonde i colori
in modo più delicato, la **trasparenza** del nodo selezionato e **Cancella
maglia**, che rimuove il riempimento a maglia. Cambiare righe e colonne di una
maglia ne mantiene i colori.

## Riempimenti a motivo e texture {#pattern-texture}

- Un riempimento [[docker.fill_pattern]] ripete una tessera: due colori, un
  motivo vettoriale o una bitmap. Vedi [Riempimenti a motivo](patterns.md).
- Un riempimento [[docker.fill_texture]] è una texture generata dall'aspetto
  naturale: [[texture.clouds]], [[texture.marble]], [[texture.noise]] o
  [[texture.wood]], con i suoi colori e la sua scala.

## Più riempimenti su un oggetto {#several-fills}

Nella sezione riempimento del pannello [[docker.properties]],
[[docker.add_fill]] dipinge un altro riempimento sopra quello dell'oggetto,
partendo da una copia da modificare. I riempimenti successivi appaiono come
"Riempimento 2", "Riempimento 3" e così via, ciascuno con il proprio editor e
un pulsante per rimuoverlo. La sezione contorno può aggiungere altri contorni
allo stesso modo.

I riempimenti possono essere dipinti anche sui contorni; vedi
[Impostare i contorni](../lines-shapes/outlines.md#outline-pen).
