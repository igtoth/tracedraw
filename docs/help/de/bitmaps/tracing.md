# Bitmaps nachzeichnen

Das Nachzeichnen wandelt eine Bitmap in Vektorformen um, die Sie ohne
Qualitätsverlust bearbeiten, umfärben und skalieren können: ein eingescanntes
Logo, eine Skizze, ein Foto.

## Schnelles Nachzeichnen {#quick-trace}

Wählen Sie eine Bitmap aus und wählen Sie
[[menu.bitmaps > menu.bitmaps.quick_trace]]. Es verwendet die Vorgabe, die auf
der Seite [[options.bitmap_tracing]] der globalen Optionen gewählt ist
(anfangs Clipart).

[[menu.bitmaps.repeat_last_trace]] vektorisiert die ausgewählte Bitmap erneut wie die letzte Vektorisierung, ohne Dialogfeld, und [[menu.bitmaps.last_trace_settings]] öffnet das Dialogfeld mit deren Einstellungen. Auch das Kontextmenü einer Bitmap enthält diese Befehle, zusammen mit [[menu.bitmaps.centerline_trace]] und [[menu.bitmaps.outline_trace]].

## Das Dialogfeld Bitmap nachzeichnen {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] und
[[menu.bitmaps.centerline_trace]] listen Vorgaben auf; wählen Sie eine, um das
Dialogfeld zum Nachzeichnen zu öffnen.

- **Umrissnachzeichnung** füllt Farbflächen mit Formen:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] und [[menu.bitmaps.trace_high_quality]].
- **Mittellinien-Nachzeichnung** folgt Linien mit offenen Kurven:
  [[menu.bitmaps.trace_technical]] und [[menu.bitmaps.trace_line_drawing]].

![Das Dialogfeld Bitmap nachzeichnen](../../img/bitmaps-tracing.png)

Das Dialogfeld ist ein Fenster, das Sie vergrößern und maximieren können.
Die Vorschau füllt seine linke Seite, die Einstellungen liegen in einem
Bereich rechts, und die Nachzeichnung wird bei jeder Änderung einer
Einstellung neu erstellt: Sie müssen nichts drücken, um das Ergebnis zu
sehen. Unter der Vorschau zählt eine Zeile die **Kurven**, **Knoten** und
**Farben** der Nachzeichnung. Klicken Sie auf [[dialog.ok]], um die Formen
über der Bitmap zu platzieren, die dunkelsten zuerst, oder auf
[[dialog.cancel]], um die Bitmap zu lassen, wie sie ist.

## Die Vorschau {#preview}

Die Liste [[dialog.trace_preview_label]] wählt, was die Vorschau zeigt:

- [[dialog.trace_view_before_after]]: die Bitmap oben und ihre
  Nachzeichnung darunter, oder [[dialog.view_side_by_side]];
  [[dialog.before]] zeigt nur die Bitmap.
- [[dialog.trace_view_large]]: nur die Nachzeichnung, so groß wie die
  Vorschau.
- [[dialog.trace_view_wireframe]]: die Kurven der Nachzeichnung als dünne
  Linien über der Bitmap. Neben der Liste erscheint der Regler
  [[dialog.trace_overlay]]: Er legt fest, wie stark die Bitmap unter den
  Linien durchscheint, von 0 (nur die Linien) bis 100 (die Bitmap
  unverändert), anfangs 80.

Die Schaltflächen nach der Liste passen die Ansicht ein, verkleinern,
vergrößern und schwenken. Das Mausrad zoomt um den Zeiger, ein Ziehen
verschiebt das Bild; beide Felder bewegen sich gemeinsam. Transparente
Teile zeigen ein Schachbrett.

## Einstellungen {#settings}

Die Registerkarte [[dialog.trace_tab_settings]] enthält:

- [[dialog.trace_type]] Umriss oder Mittellinie, und
  [[dialog.trace_image_type]] die Vorgabe. Die Wahl setzt die Regler
  darunter auf die Werte, die zu dieser Art von Bild passen.
- [[dialog.detail]] (0 bis 100): mehr Detail behält kleinere Flächen und
  feinere Linien; weniger Detail glättet Flecken und Körnung weg.
- [[dialog.smoothing]] (0 bis 100, anfangs 25): glattere Kurven mit weniger
  Knoten.
- [[dialog.corner_smoothness]] (0 bis 100): rundere Ecken.
- [[dialog.trace_remove]]: was aus der Nachzeichnung wegbleibt.
  **Hintergrundfarbe** entfernt den Hintergrund dort, wo er den Bildrand
  erreicht, **Gesamte Farbe aus dem Bild** entfernt jede Fläche dieser
  Farbe, **Keine** zeichnet alles nach. [[dialog.trace_remove_auto]] findet
  die Hintergrundfarbe am Bildrand; [[dialog.trace_remove_specify]]
  verwendet die Farbe des Farbfelds, die Sie ändern oder mit der Pipette
  daneben aus der Vorschau aufnehmen können.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (anfangs ein)
  macht aus jeder Fläche einer Farbe eine Kurve; aus, wird jeder getrennte
  Fleck zu einer eigenen Kurve. [[dialog.trace_remove_overlap]] hält jede
  Form auf ihren eigenen Pixeln; aus, reichen Formen ein wenig unter die
  darüberliegenden, damit zwischen Nachbarn keine Lücken erscheinen.
  [[dialog.trace_group_by_color]] legt die Kurven jeder Farbe in eine
  eigene Gruppe; es ist verfügbar, wenn die Flächen einer Farbe nicht
  bereits zusammengeführt sind.
- [[dialog.delete_original]]: die Bitmap verschwindet, wenn die
  Nachzeichnung platziert wird.

Einstellungen, die für den aktuellen Nachzeichnungstyp nicht gelten, sind
ausgegraut: eine Mittellinien-Nachzeichnung liest nur Detail und Glättung.

## Farben {#colors}

Die Registerkarte [[dialog.trace_tab_colors]] legt den [[dialog.color_mode]]
(RGB, Graustufen oder Schwarzweiß) und die [[dialog.trace_number_of_colors]]
(2 bis 256) fest und listet die Farben der Nachzeichnung mit der Anzahl
ihrer Kurven auf. Haken Sie zwei oder mehr Farben an und klicken Sie auf
[[dialog.trace_merge_colors]], um sie in der ersten zusammenzuführen;
klicken Sie auf das Farbfeld einer Farbe, um ihre Kurven in einer anderen
Farbe zu zeichnen. Eine Änderung, die das Bild neu nachzeichnet, vergisst
diese Bearbeitungen, die zur vorigen Nachzeichnung gehörten.

## Zurücksetzen, Rückgängig und Wiederholen {#steps}

Unter dem Einstellungsbereich setzt [[dialog.reset]] die Einstellungen der
Vorgabe zurück, und die beiden Pfeile gehen durch jede im Dialogfeld
vorgenommene Änderung zurück und vor: jedes Loslassen eines Reglers, jede
Listenauswahl, jedes Kontrollkästchen, jede Farbbearbeitung und jedes
Zurücksetzen ist ein Schritt. Eine Änderung nach dem Rückgängigmachen
verwirft die rückgängig gemachten Schritte.

[[dialog.trace_options]] öffnet die Seite [[options.bitmap_tracing]] der
globalen Optionen, auf der Sie die Vorgabe für das schnelle Nachzeichnen
und die Startwerte jeder Nachzeichnung wählen, und kehrt dann zum
Dialogfeld zurück. Die Schaltfläche [[dialog.help_button]] öffnet diese
Seite.
