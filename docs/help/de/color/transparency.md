# Transparenz

Transparenz lässt durchscheinen, was unter einem Objekt liegt. Sie kann überall
gleich sein, sich wie ein Verlauf über das Objekt ändern oder einem Muster oder
einer Textur folgen.

## Transparenz anwenden {#apply}

- Mit dem Werkzeug [[tool.transparency]]: Klicken Sie auf ein Objekt und wählen
  Sie dann in der Eigenschaftsleiste eine Art von Transparenz, oder ziehen Sie
  über das Objekt, um eine Verlaufstransparenz von deckend (wo Sie drücken) bis
  transparent (wo Sie loslassen) zu erhalten. Die Knoten des Verlaufs erscheinen dann am Objekt wie beim Werkzeug [[tool.interactive_fill]]: Ziehen Sie den Anfang, das Ende oder den runden Neigungsknoten; ziehen Sie den Balken zwischen zwei Knoten (den Mittelpunkt), um zu verschieben, wo der Übergang zur Hälfte erreicht ist; ein Doppelklick auf die Linie fügt einen Knoten hinzu, auf einen Knoten entfernt ihn.
- Im Bereich [[docker.transparency]] des Andockfensters [[docker.properties]].

## Einstellungen {#settings}

- Die Art: [[transparency.none]], [[transparency.uniform]] (anfangs),
  [[transparency.fountain]] (linear, radial, konisch, quadratisch oder
  formbezogen, wie bei Verlaufsfüllungen), [[transparency.pattern]] oder
  [[transparency.texture]].
- Die Stärke: 0 bis 100 % (anfangs 50 %). Bei einem Verlauf sind die
  Farbstopps Grautöne: Weiß ist deckend, Schwarz vollständig transparent.
- [[transparency.merge_mode]]: wie sich die Farben mit dem Darunterliegenden
  mischen. Normal (anfangs) lässt einfach durchscheinen; die anderen, etwa
  Addieren, Subtrahieren, Differenz, Multiplizieren, Dividieren, Wenn heller,
  Wenn dunkler, Farbton, Sättigung, Farbe und Umkehren, kombinieren die Farben
  auf andere Weise.
- Ob sie für [[transparency.target_fill]], [[transparency.target_outline]] oder
  [[transparency.target_all]] gilt.
- **Fixieren**: behält das Durchscheinende so, wie es jetzt ist, auch wenn sich
  die Objekte darunter bewegen.

Zum Entfernen klicken Sie am Ende der Eigenschaftsleiste auf [[toolbar.clear_transparency]] (oder wählen Sie [[transparency.none]] als Art).

Um die Transparenz eines anderen Objekts zu kopieren, verwenden Sie
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
Für durchsichtige Bereiche, die durch die Helligkeit eines Objekts bestimmt
werden, siehe [Deckkraftmasken](../objects/clip-frames.md#opacity-mask).
