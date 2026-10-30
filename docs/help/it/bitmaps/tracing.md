# Ricalcare le bitmap

Il ricalco trasforma una bitmap in forme vettoriali che si possono
modificare, ricolorare e ridimensionare senza perdita di qualità: un logo
scansionato, uno schizzo, una foto.

## Ricalco rapido {#quick-trace}

Selezionate una bitmap e scegliete
[[menu.bitmaps > menu.bitmaps.quick_trace]]. Usa la preimpostazione scelta
nella pagina [[options.bitmap_tracing]] delle opzioni globali (Clipart
all'inizio).

[[menu.bitmaps.repeat_last_trace]] ricalca di nuovo la bitmap selezionata come l'ultimo ricalco, senza finestra di dialogo, e [[menu.bitmaps.last_trace_settings]] apre la finestra di dialogo con le sue impostazioni. Anche il menu contestuale di una bitmap ha questi comandi, con [[menu.bitmaps.centerline_trace]] e [[menu.bitmaps.outline_trace]].

## La finestra di dialogo Ricalca bitmap {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] e
[[menu.bitmaps.centerline_trace]] elencano le preimpostazioni; sceglietene
una per aprire la finestra di dialogo.

- **Ricalco del contorno** riempie le aree di colore con forme:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] e [[menu.bitmaps.trace_high_quality]].
- **Ricalco della linea centrale** segue le linee con curve aperte:
  [[menu.bitmaps.trace_technical]] e [[menu.bitmaps.trace_line_drawing]].

![La finestra di dialogo Ricalca bitmap](../../img/bitmaps-tracing.png)

La finestra di dialogo si può ridimensionare e ingrandire. L'anteprima
occupa il lato sinistro, le impostazioni stanno in un pannello a destra, e
il ricalco viene rifatto a ogni cambio di impostazione: non c'è nulla da
premere per vedere il risultato. Sotto l'anteprima una riga conta le
**curve**, i **nodi** e i **colori** del risultato. Fate clic su
[[dialog.ok]] per collocare le forme sopra la bitmap, le più scure per
prime, o su [[dialog.cancel]] per lasciare la bitmap com'è.

## L'anteprima {#preview}

L'elenco [[dialog.trace_preview_label]] sceglie cosa mostra l'anteprima:

- [[dialog.trace_view_before_after]]: la bitmap sopra e il suo ricalco
  sotto, oppure [[dialog.view_side_by_side]]; [[dialog.before]] mostra solo
  la bitmap.
- [[dialog.trace_view_large]]: solo il ricalco, grande quanto l'anteprima.
- [[dialog.trace_view_wireframe]]: le curve del risultato disegnate come
  linee sottili sopra la bitmap. Accanto all'elenco compare il cursore
  [[dialog.trace_overlay]]: stabilisce quanto la bitmap traspare sotto le
  linee, da 0 (solo le linee) a 100 (la bitmap com'è), 80 all'inizio.

I pulsanti dopo l'elenco adattano l'immagine alla finestra, riducono,
ingrandiscono e spostano. La rotellina ingrandisce intorno al puntatore e
un trascinamento sposta l'immagine; i due riquadri si muovono insieme. Le
parti trasparenti mostrano una scacchiera.

## Impostazioni {#settings}

La scheda [[dialog.trace_tab_settings]] contiene:

- [[dialog.trace_type]] Contorno o Linea centrale, e
  [[dialog.trace_image_type]] la preimpostazione. Sceglierne una porta i
  cursori sottostanti ai valori adatti a quel tipo di immagine.
- [[dialog.detail]] (da 0 a 100): più dettaglio conserva aree più piccole e
  linee più sottili; meno dettaglio leviga macchie e grana.
- [[dialog.smoothing]] (da 0 a 100, 25 all'inizio): curve più morbide con
  meno nodi.
- [[dialog.corner_smoothness]] (da 0 a 100): angoli più arrotondati.
- [[dialog.trace_remove]]: cosa lasciare fuori dal risultato. **Colore di
  sfondo** toglie lo sfondo dove raggiunge il bordo dell'immagine, **Intero
  colore dall'immagine** toglie ogni area di quel colore, **Nessuno**
  ricalca tutto. [[dialog.trace_remove_auto]] trova il colore di sfondo
  lungo il bordo dell'immagine; [[dialog.trace_remove_specify]] usa il
  colore del campione, che potete cambiare o prelevare dall'anteprima con
  il contagocce accanto.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (attivo
  all'inizio) fa una sola curva di ogni area di un colore; disattivato,
  ogni chiazza separata è una curva a sé. [[dialog.trace_remove_overlap]]
  tiene ogni forma sui propri pixel; disattivato, le forme si estendono un
  poco sotto quelle sopra, così tra vicine non si vedono fessure.
  [[dialog.trace_group_by_color]] mette le curve di ogni colore in un
  gruppo a sé; è disponibile quando le aree di un colore non sono già
  unite.
- [[dialog.delete_original]]: la bitmap sparisce quando il risultato viene
  collocato.

Le impostazioni che non valgono per il tipo di ricalco in uso sono in
grigio: un ricalco della linea centrale legge solo il dettaglio e la
levigatura.

## Colori {#colors}

La scheda [[dialog.trace_tab_colors]] imposta la [[dialog.color_mode]] (RGB,
scala di grigi, o bianco e nero) e il [[dialog.trace_number_of_colors]] (da
2 a 256), ed elenca i colori del risultato con il numero di curve di
ciascuno. Spuntate due o più colori e fate clic su
[[dialog.trace_merge_colors]] per unirli nel primo; fate clic sul campione
di un colore per disegnare le sue curve in un altro colore. Cambiare
un'impostazione che ricalca di nuovo l'immagine dimentica queste
modifiche, che appartenevano al risultato precedente.

## Ripristina, annulla e ripeti {#steps}

Sotto il pannello delle impostazioni, [[dialog.reset]] rimette le
impostazioni della preimpostazione, e le due frecce vanno indietro e avanti
su ogni cambio fatto nella finestra di dialogo: ogni rilascio di un
cursore, scelta in un elenco, casella, modifica di colore e ripristino è un
passo. Un cambio fatto dopo un annullamento scarta i passi annullati.

[[dialog.trace_options]] apre la pagina [[options.bitmap_tracing]] delle
opzioni globali, dove scegliete la preimpostazione del ricalco rapido e le
scelte con cui inizia ogni ricalco, e poi torna alla finestra di dialogo.
Il pulsante [[dialog.help_button]] apre questa pagina.
