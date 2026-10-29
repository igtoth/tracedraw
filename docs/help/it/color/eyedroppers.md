# Copiare colori e attributi con i contagocce

Gli strumenti contagocce prendono un colore, o un intero insieme di attributi,
da un oggetto e lo danno ad altri. Entrambi funzionano in due fasi, mostrate
nella barra delle proprietà: **seleziona** (preleva un campione) e **applica**
(dipingi con esso). Dopo un campione, lo strumento passa ad applica; fai clic
sul pulsante di selezione per campionare di nuovo. <kbd>Esc</kbd> scarta il
campione e torna allo strumento [[tool.pick]].

Il puntatore è una pipetta mentre lo strumento campiona e un secchiello mentre
applica. Le immagini danno il loro colore anche dentro un gruppo o una cornice.

## Contagocce colore {#color-eyedropper}

1. Scegli lo strumento [[tool.color_eyedropper]].
2. Fai clic su un oggetto per prenderne il colore: il colore di un
   riempimento uniforme, il primo colore di una sfumatura o il colore del
   contorno. Su una bitmap, prende il colore dei pixel sotto il puntatore, in
   media sulla dimensione del campione impostata nella barra delle proprietà
   (1 per 1, 2 per 2 o 5 per 5 pixel). <kbd>Shift</kbd>+clic prende prima il
   colore del contorno.
3. Fai clic sugli oggetti per riempirli con quel colore; <kbd>Shift</kbd>+clic
   per colorarne il contorno (un oggetto senza contorno ne riceve uno
   sottile). Facendo clic all'interno di una forma chiusa vuota, la si
   riempie.

La barra delle proprietà mostra il colore campionato.

Mentre sposti il puntatore per prendere un colore, un piccolo riquadro accanto
mostra il colore sotto: un campione, il nome del colore se è un colore della
tavolozza o un inchiostro spot, i suoi valori nel modello di colore del
disegno (CMYK o RGB) e il suo valore esadecimale. Il contagocce di un
selettore di colore mostra lo stesso riquadro.

## Contagocce attributi {#attributes-eyedropper}

Lo strumento [[tool.attributes_eyedropper]] copia più proprietà
contemporaneamente da un oggetto ad altri.

1. Scegli lo strumento [[tool.attributes_eyedropper]].
2. Nella barra delle proprietà, scegli cosa copiare:
   - **Proprietà**: contorno, riempimento e testo (font, dimensione e stile;
     il testo stesso viene mantenuto). Tutto attivo in partenza.
   - **Trasformazioni**: dimensione, rotazione e posizione. Disattivate in
     partenza.
   - **Effetti**: trasparenza, ombra esterna ed effetti dinamici. Disattivati
     in partenza.
3. Fai clic sull'oggetto da cui copiare.
4. Fai clic su ogni oggetto su cui copiare. <kbd>Shift</kbd>+clic campiona
   invece un altro oggetto.

Ogni clic è un passaggio di annullamento.
