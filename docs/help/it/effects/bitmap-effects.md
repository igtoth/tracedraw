# Effetti bitmap

Gli effetti bitmap modificano i pixel: sfocano, aumentano la nitidezza,
regolano i colori, aggiungono texture e tratti artistici. Funzionano sulle
bitmap e anche sugli oggetti vettoriali, che vengono allora disegnati come
l'immagine prodotta dagli effetti pur mantenendo la propria forma, il
riempimento e il contorno.

Gli effetti sono tenuti separati dai pixel originali: puoi modificarli,
nasconderli, riordinarli o rimuoverli in seguito, e rimuovere l'ultimo
restituisce l'originale.

## Per applicare un effetto bitmap {#apply}

1. Seleziona uno o più oggetti.
2. Scegli un gruppo nel menu [[menu.effects]] e un effetto.
3. Gli effetti senza impostazioni si applicano subito. Gli altri aprono una
   finestra di dialogo con le anteprime **prima** e **dopo** del primo
   oggetto selezionato: imposta i valori e fai clic su [[dialog.ok]].
   Le impostazioni stanno in un pannello a destra; sotto di esso
   **Reimposta** riporta ogni impostazione al valore predefinito e le due
   frecce annullano e ripetono le modifiche fatte nella finestra di dialogo.

![Una finestra di dialogo di effetto bitmap](../../img/effects-bitmap-effects.png)

La modifica dell'intera selezione è un unico passaggio di annullamento. Con
una bitmap selezionata, <kbd>Ctrl+B</kbd> apre
[[fx.brightness_contrast_intensity]], <kbd>Ctrl+Shift+B</kbd>
[[fx.color_balance]] e <kbd>Ctrl+Shift+U</kbd> [[fx.hue_saturation_lightness]].

## I gruppi di effetti {#groups}

| Gruppo | Effetti |
|---|---|
| [[menu.effects.3d]] | [[fx.cylinder]], [[fx.emboss]], [[fx.page_curl]], [[fx.pinch_punch]], [[fx.sphere]] |
| [[menu.effects.adjust]] | [[fx.auto_adjust]], [[fx.image_adjustments]], [[fx.contrast_enhancement]], [[fx.local_equalization]], [[fx.target_balance]], [[fx.tone_curve]], [[fx.brightness_contrast_intensity]], [[fx.color_balance]], [[fx.gamma]], [[fx.hue_saturation_lightness]], [[fx.selective_color]], [[fx.replace_colors]], [[fx.desaturate]], [[fx.channel_mixer]] |
| [[menu.effects.art_strokes]] | [[fx.charcoal]], [[fx.conte_crayon]], [[fx.crayon]], [[fx.cubist]], [[fx.impressionist]], [[fx.palette_knife]], [[fx.pastels]], [[fx.pen_ink]], [[fx.pointillist]], [[fx.scraperboard]], [[fx.sketch_pad]], [[fx.watercolor]], [[fx.felt_marker]], [[fx.paper_grain]] |
| [[menu.effects.blur]] | [[fx.directional_smooth]], [[fx.gaussian_blur]], [[fx.jagged_edges]], [[fx.low_pass]], [[fx.motion_blur]], [[fx.radial_blur]], [[fx.smart_blur]], [[fx.smooth]], [[fx.soften]], [[fx.zoom]] |
| [[menu.effects.camera]] | [[fx.colorize]], [[fx.diffuse]], [[fx.photo_filter]], [[fx.sepia_toning]], [[fx.vintage_photo]] |
| [[menu.effects.color_transform]] | [[fx.bit_planes]], [[fx.halftone]], [[fx.psychedelic]], [[fx.solarize]] |
| [[menu.effects.contour_group]] | [[fx.edge_detect]], [[fx.find_edges]], [[fx.trace_contour]] |
| [[menu.effects.correction]] | [[fx.dust_and_scratch]] |
| [[menu.effects.creative]] | [[fx.crystallize]], [[fx.fabric]], [[fx.frame]], [[fx.glass_tiles]], [[fx.mosaic]], [[fx.scatter]], [[fx.tinted_glass]], [[fx.stained_glass]], [[fx.vignette]], [[fx.vortex]] |
| [[menu.effects.custom]] | [[fx.bump_map]] |
| [[menu.effects.distort]] | [[fx.blocks]], [[fx.displace]], [[fx.mesh_warp]], [[fx.offset]], [[fx.pixelate]], [[fx.ripple]], [[fx.swirl]], [[fx.tile]], [[fx.wet_paint]], [[fx.whirlpool]], [[fx.wind]] |
| [[menu.effects.noise]] | [[fx.add_noise]], [[fx.maximum]], [[fx.median]], [[fx.minimum]], [[fx.remove_moire]], [[fx.remove_noise]] |
| [[menu.effects.sharpen]] | [[fx.adaptive_unsharp]], [[fx.directional_sharpen]], [[fx.high_pass]], [[fx.sharpen]], [[fx.unsharp_mask]] |
| [[menu.effects.texture]] | [[fx.cobblestone]], [[fx.wrinkles]], [[fx.etching]], [[fx.plastic]], [[fx.relief_sculpture]], [[fx.stone]] |
| [[menu.effects.transform]] | [[fx.deinterlace]], [[fx.invert_colors]], [[fx.posterize]] |

## Finestre di dialogo speciali {#special-dialogs}

- **[[fx.image_adjustments]]** apre una finestra di dialogo più grande:
  un'ampia anteprima che puoi ingrandire, spostare e ruotare, mostrata da
  sola, affiancata o divisa; **Regolazione automatica** e selettori per i
  punti di bianco e di nero; cursori per temperatura, tinta, saturazione,
  luminosità, contrasto, luci, ombre e toni medi, con un istogramma; annulla
  e ripristina all'interno della finestra di dialogo; e istantanee delle
  impostazioni da confrontare.
- **[[fx.contrast_enhancement]]** mostra l'istogramma con indicatori da
  trascinare per i livelli di ingresso e di uscita, contagocce per il nero e
  il bianco, e **Regolazione automatica**.
- **[[fx.tone_curve]]** modifica una curva per canale (principale, rosso,
  verde, blu) in quattro stili (curva, retta, a mano libera, gamma),
  sull'istogramma; le curve possono essere salvate e caricate.
- I colori dell'immagine, come in [[fx.replace_colors]], hanno un contagocce:
  fai clic su un'anteprima per prendere il colore sotto il puntatore.

## Modificare gli effetti di un oggetto {#fx-section}

La sezione [[docker.fx]] del pannello [[docker.properties]] elenca gli effetti
dell'oggetto selezionato, con il primo applicato in alto. Ogni riga ha una
casella per mostrare o nascondere l'effetto, **Modifica** (apre la sua
finestra di dialogo), **Sposta su**, **Sposta giù** ed **Elimina**.
**Aggiungi effetto** apre i gruppi di effetti.

[[menu.effects > menu.effects.flatten]] rende permanenti gli effetti: una
bitmap mantiene ciò che mostra e un oggetto vettoriale diventa una bitmap.

## Il selettore di effetti {#effect-picker}

[[menu.effects > menu.effects.picker]] apre una finestra che resta aperta
mentre lavori, con ogni effetto rappresentato da una piccola immagine. Digita
nel campo di ricerca per trovare gli effetti, oppure scegli un gruppo o
**Recenti** dall'elenco. Con qualcosa selezionato, fai clic su una tessera per
applicare l'effetto. Puoi anche applicare un effetto come
[lente](lens.md).

## Effetti che si estendono oltre i bordi {#inflate}

Le sfocature ed effetti simili possono arrivare oltre i bordi di una bitmap.
Con [[menu.bitmaps > menu.bitmaps.inflate > menu.bitmaps.inflate_auto]] attivo
(come in partenza), la bitmap si ingrandisce per fare loro spazio. Vedi
[Ricampionare e raddrizzare](../bitmaps/resample-and-straighten.md#inflate).

Per salvare una bitmap con i suoi effetti come file PNG, fai clic con il
pulsante destro su di essa e scegli [[menu.bitmaps.save_bitmap_as]].
