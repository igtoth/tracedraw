# Suchen und Ersetzen

Das Andockfenster [[docker.find_replace]] (<kbd>Ctrl+F</kbd>) sucht Text in der
Zeichnung und ersetzt ihn, sucht Objekte nach Namen oder nach Füllung,
Umriss, Umrissbreite, Schrift oder Typ und ersetzt diese. Öffnen Sie es mit
[[menu.edit > menu.edit.find_and_replace]].

![Das Andockfenster Suchen und Ersetzen](../../img/objects-find-replace.png)

## Text suchen und ersetzen {#text}

1. Wählen Sie [[dialog.find_text]].
2. Geben Sie den Suchbegriff in [[dialog.find]] ein und den neuen Text in
   [[dialog.replace_with]].
3. Aktivieren Sie [[dialog.match_case]], um Groß- und Kleinbuchstaben zu
   unterscheiden, und [[dialog.whole_word]], um den Text zu überspringen, wenn
   er Teil eines längeren Wortes ist.
4. Klicken Sie auf [[dialog.find_next]], um das nächste Textobjekt
   auszuwählen, das ihn enthält, oder auf [[dialog.find_all]], um alle
   auszuwählen.
5. Klicken Sie auf [[dialog.replace]], um ihn im ausgewählten Text zu
   ersetzen, oder auf [[dialog.replace_all]], um ihn überall zu ersetzen.

Das Andockfenster zeigt an, wie viele Treffer gefunden wurden.

## Objekte suchen {#objects}

Wählen Sie [[dialog.find_objects]], geben Sie einen Teil eines Namens oder eine
Objektart wie „Rectangle“ ein und klicken Sie auf [[dialog.find_next]] oder
[[dialog.find_all]]. Die Suche umfasst die aktuelle Seite. Wie Sie Objekte
benennen, lesen Sie unter
[Objekte und Ebenen umbenennen](objects-and-layers.md#rename).

## Nach Eigenschaften suchen und ersetzen {#properties}

1. Wählen Sie [[find.properties]].
2. Wählen Sie unter [[find.look_in]] [[find.this_page]] oder [[find.whole_drawing]].
3. Kreuzen Sie unter [[find.find_objects_with]] an, wonach gesucht wird:
   [[find.object_type]], [[find.fill]], [[find.outline]] (eine Farbe oder
   [[find.none]]), [[find.outline_width]] oder [[find.font]]. Gefunden wird ein
   Objekt, das alles Angekreuzte hat. [[find.from_selection]] übernimmt diese
   Werte vom ausgewählten Objekt, etwa um alle Objekte mit seiner Füllfarbe zu
   finden.
4. Klicken Sie auf [[find.previous]] oder [[dialog.find_next]], um die Objekte
   nacheinander auszuwählen (auch auf anderen Seiten), oder auf
   [[dialog.find_all]], um die auf dieser Seite auszuwählen.
5. Zum Ändern kreuzen Sie unter [[find.replace_with]] die neue Füllfarbe,
   Umrissfarbe, Umrissbreite oder Schrift an und klicken auf
   [[dialog.replace_all]]. Alle Änderungen lassen sich in einem Schritt
   rückgängig machen.

Eine gesuchte Füllfarbe wird überall ersetzt, wo sie vorkommt, auch in
Verläufen. Gesperrte Objekte bleiben unverändert.
