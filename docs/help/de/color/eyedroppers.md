# Farben und Attribute mit den Pipetten kopieren

Die Pipettenwerkzeuge nehmen eine Farbe oder einen ganzen Satz von Attributen
von einem Objekt auf und übertragen sie auf andere. Beide arbeiten in zwei
Schritten, die in der Eigenschaftsleiste angezeigt werden: **Auswählen** (eine
Probe nehmen) und **Anwenden** (damit malen). Nach einer Probe wechselt das
Werkzeug zu Anwenden; klicken Sie auf die Schaltfläche Auswählen, um erneut
eine Probe zu nehmen. <kbd>Esc</kbd> verwirft die Probe und kehrt zum Werkzeug
[[tool.pick]] zurück.

Der Zeiger ist eine Pipette, während das Werkzeug eine Probe nimmt, und ein
Farbeimer, während es anwendet. Bilder geben ihre Farbe auch in einer Gruppe
oder einem Rahmen.

## Farbpipette {#color-eyedropper}

1. Wählen Sie das Werkzeug [[tool.color_eyedropper]].
2. Klicken Sie auf ein Objekt, um seine Farbe aufzunehmen: die Füllfarbe einer
   einheitlichen Füllung, die erste Farbe eines Verlaufs oder die Umrissfarbe.
   Auf einer Bitmap nimmt sie die Farbe der Pixel unter dem Zeiger auf,
   gemittelt über die in der Eigenschaftsleiste eingestellte Probengröße
   (1 mal 1, 2 mal 2 oder 5 mal 5 Pixel). <kbd>Shift</kbd>+Klick nimmt zuerst
   die Umrissfarbe auf.
3. Klicken Sie auf Objekte, um sie mit dieser Farbe zu füllen;
   <kbd>Shift</kbd>+Klick färbt ihren Umriss (ein Objekt ohne Umriss erhält
   einen dünnen). Ein Klick in eine leere geschlossene Form füllt sie.

Die Eigenschaftsleiste zeigt die aufgenommene Farbe.

Während Sie den Zeiger bewegen, um eine Farbe aufzunehmen, zeigt ein kleines
Feld daneben die Farbe darunter: ein Farbfeld, den Namen der Farbe, wenn sie
eine Palettenfarbe oder eine Schmuckfarbe ist, ihre Werte im Farbmodus der
Zeichnung (CMYK oder RGB) und ihren Hex-Wert. Die Pipette einer Farbauswahl
zeigt dasselbe Feld.

## Attributpipette {#attributes-eyedropper}

Das Werkzeug [[tool.attributes_eyedropper]] kopiert mehrere Eigenschaften auf
einmal von einem Objekt auf andere.

1. Wählen Sie das Werkzeug [[tool.attributes_eyedropper]].
2. Wählen Sie in der Eigenschaftsleiste, was kopiert werden soll:
   - **Eigenschaften**: Umriss, Füllung und Text (Schrift, Größe und Stil; der
     Text selbst bleibt erhalten). Anfangs alle ein.
   - **Transformationen**: Größe, Drehung und Position. Anfangs aus.
   - **Effekte**: Transparenz, hinterlegter Schatten und Live-Effekte.
     Anfangs aus.
3. Klicken Sie auf das Objekt, von dem kopiert werden soll.
4. Klicken Sie auf jedes Objekt, auf das kopiert werden soll.
   <kbd>Shift</kbd>+Klick nimmt stattdessen eine Probe von einem anderen
   Objekt.

Jeder Klick ist ein Rückgängig-Schritt.
