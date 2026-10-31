# Kopien mit Schritt und Wiederholung erstellen

Das Andockfenster [[docker.step_and_repeat]] erstellt mehrere Kopien der
Auswahl, jede um denselben Abstand von der vorigen verschoben: eine Reihe von
Schaltflächen, eine Spalte von Linien, ein Raster von Kacheln. Öffnen Sie es
mit [[menu.edit > menu.edit.step_and_repeat]] (<kbd>Ctrl+Shift+D</kbd>).

![Das Andockfenster Schritt und Wiederholung](../../img/objects-step-and-repeat.png)

## Kopien erstellen {#copies}

1. Wählen Sie die zu kopierenden Objekte aus.
2. Legen Sie die Anzahl der [[docker.copies]] fest (1 bis 999).
3. Wählen Sie unter [[docker.horizontal_settings]], wie die Kopien in der
   Breite verteilt werden:
   - [[docker.offset]]: der Abstand von einer Kopie zur nächsten, gemessen vom
     selben Punkt jeder Kopie;
   - [[docker.spacing]]: die Lücke zwischen einer Kopie und der nächsten,
     sodass sich die Kopien unabhängig von ihrer Größe nie überlappen.

   Geben Sie dann den Abstand ein. 0 hält die Kopien in einer Linie.
4. Gehen Sie unter [[docker.vertical_settings]] genauso vor.
5. Klicken Sie auf [[docker.apply]].

Während Sie im Andockfenster arbeiten (der Zeiger ist darauf, Sie tippen in
eines seiner Felder oder Ihr letzter Klick war darin), werden die Kopien, die
[[docker.apply]] anlegen würde, auf der Seite blau gestrichelt umrandet und
folgen jedem geänderten Wert. Nichts wird hinzugefügt, bis Sie auf
[[docker.apply]] klicken, und ein [[menu.edit.undo]] entfernt alle Kopien
wieder. Ein Klick an anderer Stelle oder das Schließen oder Ausblenden des
Andockfensters entfernt die Umrisse. Sehr viele Kopien eines detaillierten
Objekts werden als ihre Rahmen umrandet.

Die Kopien werden der aktiven Ebene hinzugefügt. Abstände werden in den
Einheiten der Zeichnung angegeben.

Um bei jedem Schritt mit einer Drehung oder Skalierung zu kopieren, verwenden
Sie das Feld [[dialog.copies]] des Andockfensters [[docker.transformations]];
siehe [Verschieben, Skalieren, Drehen und Neigen](transforming.md#transformations-docker).
