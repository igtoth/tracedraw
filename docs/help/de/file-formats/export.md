# Exportieren

[[menu.file > menu.file.export]] (<kbd>Ctrl+E</kbd>) schreibt die Zeichnung,
eine Seite oder die Auswahl in einem anderen Format. Das Dialogfeld Exportieren
zeigt links eine Vorschau und rechts die Einstellungen.

![Das Dialogfeld Exportieren](../../img/file-formats-export.png)

## Exportieren {#export}

1. Wählen Sie [[menu.file > menu.file.export]].
2. Wählen Sie das **Format**. Die gängigen (JPG, PDF, AI, PNG) stehen zuerst.
3. Stellen Sie die Optionen des Formats ein (siehe unten).
4. Aktivieren Sie **Nur Auswahl**, um nur die Auswahl zu exportieren, oder
   **Alle Seiten**, um jede Seite zu exportieren.
5. Klicken Sie auf [[dialog.ok]] und wählen Sie den Speicherort. Im Browser
   wird die Datei heruntergeladen.

Das Dialogfeld schätzt die Größe der Datei. Ist **Filterdialog nicht anzeigen**
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

Bei Bitmap-Formaten können Sie Breite oder Höhe in Pixeln eingeben; die
Auflösung folgt. PostScript- und PRN-Dateien enthalten jede exportierte Seite.

Die Anfangswerte werden auf der Seite [[options.file_formats]] der globalen
Optionen festgelegt. [[menu.file.export_for]] hat Vorgaben für das Web und für
Office-Dokumente; siehe [Für das Web exportieren](../web-graphics/export-for-web.md).
Für PDF-Dateien mit Lesezeichen und Sicherheit verwenden Sie
[Als PDF veröffentlichen](publish-pdf.md).
