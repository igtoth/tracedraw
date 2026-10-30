# Trasparenza

La trasparenza lascia trasparire ciò che si trova sotto un oggetto. Può essere
uguale ovunque, cambiare attraverso l'oggetto come una sfumatura, oppure
seguire un motivo o una texture.

## Applicare la trasparenza {#apply}

- Con lo strumento [[tool.transparency]]: fai clic su un oggetto, poi scegli
  un tipo di trasparenza nella barra delle proprietà, oppure trascina
  attraverso l'oggetto per una trasparenza sfumata da opaca (dove premi) a
  trasparente (dove rilasci). I nodi della sfumatura compaiono poi sull'oggetto, come con lo strumento [[tool.interactive_fill]]: trascina l'inizio, la fine o il nodo rotondo di inclinazione; trascina la barra tra due nodi (il punto medio) per spostare dove la fusione è a metà; fai doppio clic sulla linea per aggiungere un nodo, su un nodo per rimuoverlo.
- Nella sezione [[docker.transparency]] del pannello [[docker.properties]].

## Impostazioni {#settings}

- Il tipo: [[transparency.none]], [[transparency.uniform]] (in partenza),
  [[transparency.fountain]] (lineare, radiale, conica, quadrata o sagomata,
  come i riempimenti sfumati), [[transparency.pattern]] o
  [[transparency.texture]].
- La quantità: da 0 a 100% (50% in partenza). Per una sfumatura, le
  interruzioni sono tonalità di grigio: il bianco è opaco, il nero è
  completamente trasparente.
- [[transparency.merge_mode]]: come i colori si mescolano con ciò che sta
  sotto. Normale (in partenza) lascia semplicemente trasparire; gli altri,
  come Aggiungi, Sottrai, Differenza, Moltiplica, Dividi, Se più chiaro, Se più
  scuro, Tonalità, Saturazione, Colore e Inverti, combinano i colori in altri
  modi.
- Se si applica a [[transparency.target_fill]], a
  [[transparency.target_outline]] o a [[transparency.target_all]].
- **Congela**: mantiene ciò che traspare così com'è ora, anche se gli oggetti
  sottostanti si spostano.

Per toglierla, fate clic su [[toolbar.clear_transparency]] alla fine della barra delle proprietà (oppure scegliete [[transparency.none]] come tipo).

Per copiare la trasparenza di un altro oggetto, usa
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
Per le aree trasparenti definite dalla luminosità di un oggetto, vedi
[Maschere di opacità](../objects/clip-frames.md#opacity-mask).
