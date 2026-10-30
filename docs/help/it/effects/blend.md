# Fusioni

Una fusione disegna una serie di passaggi intermedi tra due oggetti, cambiando
forma, dimensione e colore dall'uno all'altro. Usala per ombreggiature
morbide, motivi ripetuti o una forma che si trasforma in un'altra.

## Per fondere due oggetti {#create}

Con lo strumento:

1. Scegli lo strumento [[tool.blend]] dal menu a comparsa Effetti.
2. Trascina dall'oggetto iniziale all'oggetto finale.

Con il pannello:

1. Seleziona due oggetti.
2. Apri il pannello [[docker.blend]] con [[menu.effects > menu.effects.blend]]
   (<kbd>Ctrl+F8</kbd>).
3. Imposta le opzioni e fai clic su [[docker.apply]].

![Il pannello Fusione](../../img/effects-blend.png)

Entrambi gli oggetti restano oggetti del disegno: sposta, ridimensiona o
ricolora uno dei due e i passaggi lo seguono. Eliminare l'oggetto finale
rimuove la fusione.

## Impostazioni della fusione {#settings}

Con una fusione selezionata, ogni modifica nella barra delle proprietà o nel
pannello si applica subito.

- [[docker.blend_by_steps]]: il numero di passaggi (5 in partenza, fino a
  999).
- **Spaziatura**, per una fusione su un tracciato: la distanza tra i
  passaggi; il numero di passaggi deriva dalla lunghezza del tracciato.
- **Rotazione**: ruota progressivamente i passaggi; **Ciclo** li ruota invece
  attorno al punto intermedio tra i due oggetti.
- **Accelerazione**: il cursore degli oggetti fa addensare i passaggi verso
  un'estremità; il cursore del colore fa lo stesso per i colori.
  [[docker.blend_apply_sizing]] fa sì che l'accelerazione degli oggetti cambi
  anche le dimensioni, e [[docker.blend_link_accel]] mantiene uniti i due
  cursori. **Reimposta** li riporta entrambi in equilibrio.
- [[docker.blend_color_blend]]: [[docker.blend_direct]] (direttamente da un
  colore all'altro), [[docker.blend_clockwise]] o
  [[docker.blend_counterclockwise]] (attorno alla ruota dei colori).

La barra delle proprietà dello strumento contiene i passaggi, la rotazione e
le due accelerazioni.

## Fondere lungo un tracciato {#path}

1. Seleziona la fusione.
2. Fai clic su **Nuovo tracciato** nel pannello [[docker.blend]] e fai clic su
   una curva.

I passaggi seguono la curva. **Ruota tutti gli oggetti** ruota ogni passaggio
lungo la curva e [[docker.blend_full_path]] posiziona il primo e l'ultimo
passaggio alle estremità della curva.

## Rimuovere o dividere una fusione {#clear}

**Cancella fusione** rimuove i passaggi e lascia entrambi gli oggetti.
**Separa** trasforma i passaggi in oggetti a sé stanti.
I passaggi escono raggruppati accanto all'oggetto. [[menu.object > menu.object.break_blend_apart]] (<kbd>Ctrl+K</kbd>,
anche nel menu del tasto destro) fa lo stesso.
