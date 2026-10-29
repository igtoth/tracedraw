# Il pannello Colore

Il pannello [[docker.color]] (<kbd>Shift+F11</kbd>) miscela un colore e lo
applica al riempimento o al contorno della selezione. Anche lo strumento
[[tool.outline_color]] (<kbd>Shift+F12</kbd>) lo apre. Aprilo da
[[menu.window > menu.window.dockers]].

![Il pannello Colore](../../img/color-color-docker.png)

## Scegliere un colore {#choose}

1. Seleziona gli oggetti. Il pannello parte dal colore di riempimento del
   primo.
2. Miscela il nuovo colore in una delle viste:
   - [[docker.color_viewers]]: fai clic o trascina nel quadrato per impostare
     la saturazione (da sinistra a destra) e la luminosità (dal basso verso
     l'alto), e nella striscia accanto per impostare la tonalità;
   - [[docker.color_sliders]]: un cursore per ogni componente del
     [[docker.model]]: RGB, CMYK (in partenza), HSB, HSL, Lab, Scala di grigi
     o YIQ;
   - [[docker.color_palettes]]: i campioni di una tavolozza.

   I tre pulsanti in alto mostrano o nascondono ciascuna vista; può comparire
   qualsiasi combinazione. Puoi anche digitare un valore `#rrggbb`.
3. Fai clic su [[docker.fill]] o [[docker.outline]] in fondo al pannello per
   applicare il nuovo colore.
   Il lucchetto ([[docker.lock_colors]]) lega riempimento e contorno, così
   ciascun pulsante applica il colore a entrambi. Il pulsante "..."
   ([[docker.more_actions]]) apre le altre azioni: [[docker.no_fill]],
   [[docker.no_outline_action]], [[docker.swap_colors]],
   [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]] e
   [[docker.add_to_color_styles]].

Il colore vecchio e quello nuovo appaiono affiancati. Quando il nuovo colore
non può essere stampato esattamente con gli inchiostri CMYK, il pannello
mostra un avviso.

## Campionare un colore {#sample}

Fai clic su [[docker.eyedropper]] e poi in un punto qualsiasi del disegno per
prendere il colore di quel punto. Poi fai clic su [[docker.use_sampled]] per
renderlo il nuovo colore. Vedi anche [Contagocce](eyedroppers.md).

## Inchiostri spot {#spot-inks}

Sotto i pulsanti, [[docker.spot_inks]] elenca gli inchiostri spot del
disegno; vedi [Colori spot](spot-colors.md).
