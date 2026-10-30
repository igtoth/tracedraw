# Exportieren

> *Maschinelle Übersetzung: Diese Seite kann hinter der englischen Fassung zurückliegen.*

[[menu.file > menu.file.export]] (<kbd>Ctrl+E</kbd>) schreibt die Zeichnung,
eine Seite oder die Auswahl in einem anderen Format. Das Dialogfeld Exportieren
zeigt links eine Vorschau und rechts die Einstellungen.

![Das Dialogfeld Exportieren](../../img/file-formats-export.png)

## Exportieren {#export}

1. Wählen Sie [[menu.file > menu.file.export]].
2. Wählen Sie eine **Vorgabe**, um von gespeicherten Einstellungen
   auszugehen, oder das **Format**. Die gängigen Formate (JPG, PDF, AI, PNG)
   stehen zuerst. Die Wahl eines Formats holt die Einstellungen zurück, mit
   denen es zuletzt exportiert wurde.
3. Stellen Sie die Optionen des Formats ein (siehe unten).
4. Aktivieren Sie **Nur Auswahl**, um nur die Auswahl zu exportieren, oder
   **Alle Seiten**, um jede Seite zu exportieren.
5. Klicken Sie auf [[dialog.ok]] und wählen Sie den Speicherort. Im Browser
   wird die Datei heruntergeladen.

Das Dialogfeld schätzt die Größe der Datei und nennt bei Bitmap-Formaten, was
herauskommt, etwa „1240 x 1754 pixels, 150 dpi, sRGB“. Ist **Filterdialog nicht anzeigen**
aktiviert, geht der nächste Export mit denselben Einstellungen direkt in die
Datei.

## Formatoptionen {#options}

| Format | Optionen |
|---|---|
| PNG, WebP, GIF | Auflösung (oder Größe in Pixeln), transparenter Hintergrund; bei WebP auch Qualität |
| JPEG | Auflösung, Qualität (1 bis 100) |
| BMP, TIFF, PSD | Auflösung |
| PDF | Standard (PDF 1.4, PDF/X-1a, PDF/X-3 oder PDF/X-4), Beschnittzugabe, Ausgabebedingung für PDF/X |
| PDF, AI | Titel und Schlagwörter |
| SVG, EPS, AI, DXF, HTML, EMF, WMF, PLT, PS, PRN | keine |

## Größe von Bitmaps {#size}

Bei Bitmap-Formaten hat **Größe** eine Breite, eine Höhe und deren Einheit
(px, mm, cm oder in); die andere Seite folgt den Proportionen der Seite.

- In Pixeln setzt eine eingegebene Breite oder Höhe die Auflösung, die sie
  ergibt; so geben Sie die Größe eines Symbols (32 x 32 Pixel) direkt ein.
- In einer Länge ist die Größe die Druckgröße: Eine eingegebene Breite oder
  Höhe skaliert die Grafik bei der eingestellten Auflösung, von 1 % bis
  10000 %. Eine Schaltfläche mit dem Maßstab (etwa „200 %“) erscheint, wenn er
  nicht 100 % ist, und setzt ihn zurück.

PNG-, JPEG- und BMP-Dateien speichern ihre Auflösung und öffnen daher in der
gewählten Größe. PostScript- und PRN-Dateien enthalten jede exportierte Seite.

## Vorgaben {#presets}

Die Liste **Vorgabe** enthält jede gespeicherte Vorgabe, jeweils mit ihrem
Format in Klammern, etwa „Web PNG (PNG)“. Eine Vorgabe eines anderen Formats
schaltet das Dialogfeld auf dieses Format um; ein Formatwechsel lässt keine
Vorgabe gewählt. **+** speichert die aktuellen Einstellungen unter einem Namen
(derselbe Name ersetzt diese Vorgabe), der Stift benennt die gewählte Vorgabe
um und **-** löscht sie.

## Anfangswerte {#defaults}

Die Anfangswerte werden auf der Seite [[options.file_formats]] der globalen
Optionen festgelegt; dort lässt **Die Einstellungen des letzten Exports
merken** (anfangs an) das Dialogfeld mit Format, Auflösung und Qualität des
letzten Exports beginnen, auch in der nächsten Sitzung. [[menu.file.export_for]] hat Vorgaben für das Web und für
Office-Dokumente; siehe [Für das Web exportieren](../web-graphics/export-for-web.md).
Für PDF-Dateien mit Lesezeichen und Sicherheit verwenden Sie
[Als PDF veröffentlichen](publish-pdf.md).
