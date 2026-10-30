# Cambiare la modalità colore delle bitmap

> *Traduzione automatica: questa pagina può essere meno aggiornata della versione inglese.*

[[menu.bitmaps > menu.bitmaps.mode]] converte le bitmap selezionate in un'altra
modalità colore. Ogni bitmap selezionata cambia in un unico passaggio di
annullamento e la trasparenza viene mantenuta. Le modalità con impostazioni
aprono una finestra di dialogo con le anteprime prima e dopo.

| Modalità | Risultato |
|---|---|
| [[menu.bitmaps.mode_bw]] | solo bianco e nero (vedi sotto) |
| [[menu.bitmaps.mode_grayscale]] | sfumature di grigio |
| [[menu.bitmaps.mode_duotone]] | da uno a quattro inchiostri (vedi sotto) |
| [[menu.bitmaps.mode_paletted]] | al massimo 256 colori (vedi sotto) |
| [[menu.bitmaps.mode_rgb]] | la modalità della bitmap diventa RGB; i pixel restano come sono |
| [[menu.bitmaps.mode_lab]] | colore Lab |
| [[menu.bitmaps.mode_cmyk]] | la modalità della bitmap diventa CMYK e i suoi pixel mostrano i colori che gli inchiostri possono stampare, calcolati con i profili di gestione del colore |

Il menu spunta [[menu.bitmaps.mode_rgb]] o [[menu.bitmaps.mode_cmyk]] quando tutte le bitmap selezionate sono in quella modalità, e la barra di stato indica la modalità di una bitmap selezionata, per esempio «Bitmap 600 x 400 px (CMYK)». Un'immagine CMYK di un file `.cdr` si apre in modalità CMYK, e i file `.tdraw` conservano la modalità. [[menu.tools > menu.tools.convert_colors]] imposta allo stesso modo la modalità di ogni bitmap che converte (vedere [Convertire i colori](../color/convert-colors.md)).

## Bianco e nero {#black-and-white}

![La finestra di dialogo Bianco e nero](../../img/bitmaps-color-modes-black-and-white.png)

Scegli il metodo di conversione:

- **Tratto**: bianco dove il grigio è pari o superiore alla **soglia** (128 in
  partenza), nero altrove;
- **Ordinato**: un motivo regolare di punti;
- **Mezzetinte** (in partenza): un retino di punti rotondi, a linee o quadrati
  con un **angolo** (45 gradi) e un numero di **linee per pollice** (60);
- **Distribuzione cardinale**: una texture dall'aspetto casuale;
- **Jarvis**, **Stucki** e **Floyd-Steinberg**: diffusione dell'errore, i
  risultati più morbidi per le foto.

**Intensità** (da 0 a 100, 50 in partenza) rende il risultato più chiaro o più
scuro.

## Bicromia {#duotone}

![La finestra di dialogo Bicromia](../../img/bitmaps-color-modes-duotone.png)

Una bicromia stampa un'immagine in scala di grigi con pochi inchiostri, il che
dà toni ricchi con pochi colori.

1. Scegli il tipo: Monotono, Bicromia (in partenza), Tricromia o Quadricromia,
   per uno fino a quattro inchiostri.
2. Fai clic su un inchiostro per cambiarne il colore.
3. Modifica la sua curva: in orizzontale c'è il livello di grigio (il nero a
   sinistra), in verticale la quantità di quell'inchiostro che viene stampata.
   Trascina in uno spazio vuoto per aggiungere un punto, trascina un punto per
   spostarlo, fai doppio clic su di esso o trascinalo fuori dalla griglia per
   rimuoverlo. **Mostra tutto** disegna anche le curve degli altri inchiostri.
4. **Salva** e **Carica** conservano gli inchiostri e le curve in un file.

## A tavolozza {#paletted}

![La finestra di dialogo A tavolozza](../../img/bitmaps-color-modes-paletted.png)

- **Tavolozza**: Colori uniformi, VGA standard (16 colori), Adattiva e
  Ottimizzata (i colori dell'immagine stessa, ridotti al numero di **colori**
  impostato, fino a 256), Scala di grigi, Sistema o Personalizzata (la
  tavolozza del documento).
- **Retinatura**: Nessuna, Ordinata, Jarvis, Stucki o Floyd-Steinberg, con
  l'intensità della retinatura.

La tavolozza risultante appare sotto le impostazioni.

## Colorare le bitmap in bianco e nero {#monochrome}

Con selezionate solo bitmap a due colori, fai clic su un colore della
tavolozza per colorarne lo sfondo e fai clic con il pulsante destro per
colorarne il primo piano.
