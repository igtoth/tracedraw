# Tavolozze dei colori

La tavolozza dei colori lungo il fondo della finestra è il modo più rapido per
colorare gli oggetti. Sotto di essa, la **tavolozza del documento** raccoglie
i colori usati dal disegno. Puoi aprire altre tavolozze, crearne di tue e
modificarle.

## Usare la tavolozza {#using}

| Azione | Risultato |
|---|---|
| Clic su un colore | riempie la selezione (senza nulla di selezionato, imposta il riempimento dei nuovi oggetti) |
| Clic destro su un colore | colora il contorno della selezione |
| <kbd>Ctrl</kbd>+clic su un colore | ne mescola un decimo nel riempimento della selezione |
| Clic prolungato su un colore | mostra sue sfumature più chiare, più scure, più spente e più brillanti tra cui scegliere |
| Trascinare un colore su un oggetto | riempie l'oggetto, o ne colora il contorno se rilasciato sul bordo |
| Clic sulla casella barrata | rimuove il riempimento (clic destro: il contorno) |

Le frecce alle estremità scorrono la tavolozza e il pulsante all'estremità
destra mostra tutti i colori insieme. Passa il puntatore su un colore per
vederne il nome e i valori. Il pulsante del menu della tavolozza
all'estremità sinistra apre il menu della tavolozza.

La tavolozza può essere spostata su un altro bordo della finestra o resa
mobile: trascinala dalla sua maniglia tratteggiata.

Su ogni lato può stare al bordo della finestra o accanto al disegno, tra il disegno e i pannelli o la casella degli strumenti di quel lato. Mentre la trascini, una fascia blu mostra dove andrà; fai clic destro sulla maniglia per scegliere un posto da un elenco.

## La tavolozza del documento {#document-palette}

Ogni colore che applichi a un oggetto entra nella tavolozza del documento,
che viene salvata con il disegno. Il suo contagocce aggiunge il colore sotto
il puntatore e il suo menu contiene **Aggiorna automaticamente** (per smettere
di aggiungere colori da sola), **Aggiungi dalla selezione**, **Aggiungi dal
documento**, **Elimina colore** e **Reimposta tavolozza** (mantiene solo i
colori che il disegno usa ancora). Puoi anche trascinarvi sopra colori o
oggetti. Per mostrarla o nasconderla, vedi
[Opzioni di personalizzazione](../customize/customization.md#color-palette).

## Il pannello Tavolozze {#palettes-docker}

Il pannello [[docker.palettes]] elenca le tavolozze che hai, con il numero di
colori di ciascuna. Aprilo con
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![Il pannello Tavolozze](../../img/color-palettes.png)

- Spunta una tavolozza per mostrarla sotto la finestra di disegno.
- [[docker.edit]] la apre nell'editor delle tavolozze.
- [[menu.window.palette_open]] apre un file di tavolozza (`.tdpal`, `.gpl`,
  `.ase` o `.aco`).
- [[menu.window.palette_from_document]] e
  [[menu.window.palette_from_selection]] creano una tavolozza dei colori usati
  nel disegno o nella selezione.
- I colori della tavolozza del documento appaiono in fondo; fai clic su uno
  per riempire la selezione, clic destro per colorarne il contorno.

## Modificare una tavolozza {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]] apre
l'editor delle tavolozze.

![L'editor delle tavolozze](../../img/color-palettes-palette-editor.png)

1. Scegli la [[dialog.palette]] da modificare, o fai clic su
   [[dialog.new_palette]] per iniziarne una.
2. Digita il suo [[dialog.name]].
3. Fai clic su un colore per selezionarlo; [[dialog.add_color]] aggiunge il
   colore corrente e [[dialog.remove_color]] rimuove quello selezionato.
4. Fai clic su [[dialog.save_palette]] per salvarla come file.
5. Fai clic su [[dialog.close]].

Le tavolozze integrate non possono essere modificate; crea una nuova
tavolozza o modificane una copia.
