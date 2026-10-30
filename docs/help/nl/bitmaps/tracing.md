# Bitmaps overtrekken

Overtrekken zet een bitmap om in vectorvormen die u zonder kwaliteitsverlies
kunt bewerken, herkleuren en schalen: een gescand logo, een schets, een foto.

## Snel overtrekken {#quick-trace}

Selecteer een bitmap en kies [[menu.bitmaps > menu.bitmaps.quick_trace]].
Het gebruikt de voorinstelling die op de pagina [[options.bitmap_tracing]]
van de algemene opties is gekozen (aanvankelijk Clipart).

[[menu.bitmaps.repeat_last_trace]] trekt de geselecteerde bitmap opnieuw over zoals de laatste keer, zonder dialoogvenster, en [[menu.bitmaps.last_trace_settings]] opent het dialoogvenster met die instellingen. Het contextmenu van een bitmap heeft deze opdrachten ook, samen met [[menu.bitmaps.centerline_trace]] en [[menu.bitmaps.outline_trace]].

## Het dialoogvenster Bitmap overtrekken {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] en
[[menu.bitmaps.centerline_trace]] tonen voorinstellingen; kies er een om het
dialoogvenster te openen.

- **Omtrek overtrekken** vult kleurvlakken met vormen:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] en [[menu.bitmaps.trace_high_quality]].
- **Middellijn overtrekken** volgt lijnen met open curven:
  [[menu.bitmaps.trace_technical]] en [[menu.bitmaps.trace_line_drawing]].

![Het dialoogvenster Bitmap overtrekken](../../img/bitmaps-tracing.png)

Het dialoogvenster is een venster dat u kunt vergroten en maximaliseren. De
voorvertoning vult de linkerkant, de instellingen staan in een paneel
rechts, en het resultaat wordt opnieuw gemaakt zodra u een instelling
wijzigt: u hoeft niets in te drukken om het te zien. Onder de voorvertoning
telt een regel de **curven**, **knooppunten** en **kleuren** van het
resultaat. Klik op [[dialog.ok]] om de vormen over de bitmap te plaatsen,
de donkerste eerst, of op [[dialog.cancel]] om de bitmap te laten zoals ze
is.

## De voorvertoning {#preview}

De lijst [[dialog.trace_preview_label]] kiest wat de voorvertoning toont:

- [[dialog.trace_view_before_after]]: de bitmap boven en het resultaat
  eronder, of [[dialog.view_side_by_side]]; [[dialog.before]] toont alleen
  de bitmap.
- [[dialog.trace_view_large]]: alleen het resultaat, zo groot als de
  voorvertoning.
- [[dialog.trace_view_wireframe]]: de curven van het resultaat als dunne
  lijnen over de bitmap. Naast de lijst verschijnt de schuifregelaar
  [[dialog.trace_overlay]]: die bepaalt hoe sterk de bitmap onder de lijnen
  zichtbaar is, van 0 (alleen de lijnen) tot 100 (de bitmap zoals ze is),
  aanvankelijk 80.

De knoppen na de lijst passen de afbeelding in het venster, zoomen uit,
zoomen in en verschuiven. Het muiswiel zoomt rond de aanwijzer en slepen
verschuift de afbeelding; beide vakken bewegen samen. Transparante delen
tonen een schaakbord.

## Instellingen {#settings}

Het tabblad [[dialog.trace_tab_settings]] bevat:

- [[dialog.trace_type]] Omtrek of Middellijn, en [[dialog.trace_image_type]]
  de voorinstelling. Een keuze zet de schuifregelaars eronder op de waarden
  die bij dat soort afbeelding passen.
- [[dialog.detail]] (0 tot 100): meer detail behoudt kleinere vlakken en
  fijnere lijnen; minder detail werkt vlekjes en korrel weg.
- [[dialog.smoothing]] (0 tot 100, aanvankelijk 25): vloeiendere curven
  met minder knooppunten.
- [[dialog.corner_smoothness]] (0 tot 100): rondere hoeken.
- [[dialog.trace_remove]]: wat buiten het resultaat blijft.
  **Achtergrondkleur** verwijdert de achtergrond waar die de rand van de
  afbeelding raakt, **Hele kleur uit afbeelding** verwijdert elk vlak van
  die kleur, **Geen** trekt alles over. [[dialog.trace_remove_auto]] vindt
  de achtergrondkleur langs de rand van de afbeelding;
  [[dialog.trace_remove_specify]] gebruikt de kleur van het staal, die u
  kunt wijzigen of met de pipet ernaast uit de voorvertoning kunt kiezen.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (aanvankelijk
  aan) maakt van elk vlak van een kleur één curve; uit, wordt elk los
  stukje een eigen curve. [[dialog.trace_remove_overlap]] houdt elke vorm
  op haar eigen pixels; uit, reiken vormen een beetje onder de vormen
  erboven, zodat tussen buren geen kieren verschijnen.
  [[dialog.trace_group_by_color]] zet de curven van elke kleur in een eigen
  groep; het is beschikbaar wanneer de vlakken van een kleur niet al
  samengevoegd zijn.
- [[dialog.delete_original]]: de bitmap verdwijnt zodra het resultaat
  wordt geplaatst.

Instellingen die niet gelden voor het huidige type blijven grijs: een
middellijn-overtrekking leest alleen het detail en de vloeiendheid.

## Kleuren {#colors}

Het tabblad [[dialog.trace_tab_colors]] stelt de [[dialog.color_mode]] in
(RGB, grijswaarden, of zwart-wit) en het [[dialog.trace_number_of_colors]]
(2 tot 256), en toont de kleuren van het resultaat met het aantal curven
van elk. Vink twee of meer kleuren aan en klik op
[[dialog.trace_merge_colors]] om ze samen te voegen in de eerste; klik op
het staal van een kleur om haar curven in een andere kleur te tekenen. Een
instelling wijzigen die de afbeelding opnieuw overtrekt, vergeet deze
bewerkingen, die bij het vorige resultaat hoorden.

## Herstellen, ongedaan maken en opnieuw {#steps}

Onder het instellingenpaneel zet [[dialog.reset]] de instellingen van de
voorinstelling terug, en de twee pijlen stappen terug en vooruit door elke
wijziging in het dialoogvenster: elke losgelaten schuifregelaar, lijstkeuze,
selectievakje, kleurbewerking en herstelling is één stap. Een wijziging na
ongedaan maken laat de ongedaan gemaakte stappen vallen.

[[dialog.trace_options]] opent de pagina [[options.bitmap_tracing]] van de
algemene opties, waar u de voorinstelling voor snel overtrekken en de
startkeuzes van elke overtrekking kiest, en keert dan terug naar het
dialoogvenster. De knop [[dialog.help_button]] opent deze pagina.
