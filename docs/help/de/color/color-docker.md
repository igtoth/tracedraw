# Das Andockfenster Farbe

Das Andockfenster [[docker.color]] (<kbd>Shift+F11</kbd>) mischt eine Farbe und
weist sie der Füllung oder dem Umriss der Auswahl zu. Auch das Werkzeug
[[tool.outline_color]] (<kbd>Shift+F12</kbd>) öffnet es. Öffnen Sie es über
[[menu.window > menu.window.dockers]].

![Das Andockfenster Farbe](../../img/color-color-docker.png)

## Eine Farbe wählen {#choose}

1. Wählen Sie Objekte aus. Das Andockfenster beginnt mit der Füllfarbe des
   ersten.
2. Mischen Sie die neue Farbe in einer der Ansichten:
   - [[docker.color_viewers]]: Klicken oder ziehen Sie im Quadrat, um die
     Sättigung (von links nach rechts) und die Helligkeit (von unten nach oben)
     einzustellen, und im Streifen daneben, um den Farbton einzustellen;
   - [[docker.color_sliders]]: ein Regler pro Komponente des
     [[docker.model]]: RGB, CMYK (anfangs), HSB, HSL, Lab, Graustufen oder
     YIQ;
   - [[docker.color_palettes]]: die Farbfelder einer Palette.

   Die drei Schaltflächen oben blenden jede Ansicht ein oder aus; jede
   Kombination ist möglich. Sie können auch einen Wert `#rrggbb` eingeben.
3. Klicken Sie unten im Andockfenster auf [[docker.fill]] oder
   [[docker.outline]], um die neue Farbe zuzuweisen.
   Das Schloss ([[docker.lock_colors]]) verbindet Füllung und Umriss, sodass
   jede der beiden Schaltflächen die Farbe beiden zuweist. Die Schaltfläche
   "..." ([[docker.more_actions]]) klappt die weiteren Aktionen auf:
   [[docker.no_fill]], [[docker.no_outline_action]], [[docker.swap_colors]],
   [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]] und
   [[docker.add_to_color_styles]].

Die alte und die neue Farbe werden nebeneinander angezeigt. Kann die neue Farbe
mit CMYK-Druckfarben nicht exakt gedruckt werden, zeigt das Andockfenster eine
Warnung.

## Eine Farbe aufnehmen {#sample}

Klicken Sie auf [[docker.eyedropper]] und dann auf eine beliebige Stelle der
Zeichnung, um die Farbe dort aufzunehmen. Klicken Sie dann auf
[[docker.use_sampled]], um sie zur neuen Farbe zu machen. Siehe auch
[Pipetten](eyedroppers.md).

## Volltonfarben {#spot-inks}

Unter den Schaltflächen listet [[docker.spot_inks]] die Volltonfarben der
Zeichnung auf; siehe [Volltonfarben](spot-colors.md).
