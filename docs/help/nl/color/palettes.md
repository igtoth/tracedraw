# Kleurenpaletten

Het kleurenpalet onder in het venster is de snelste manier om objecten te
kleuren. Daaronder verzamelt het **documentpalet** de kleuren die uw tekening
gebruikt. U kunt meer paletten openen, eigen paletten maken en ze bewerken.

## Het palet gebruiken {#using}

| Handeling | Resultaat |
|---|---|
| Op een kleur klikken | vult de selectie (als er niets geselecteerd is, stelt het de vulling van nieuwe objecten in) |
| Met de rechtermuisknop op een kleur klikken | kleurt de omtrek van de selectie |
| <kbd>Ctrl</kbd>+klik op een kleur | mengt een tiende ervan in de vulling van de selectie |
| Een kleur ingedrukt houden | toont lichtere, donkerdere, doffere en helderdere tinten ervan om uit te kiezen |
| Een kleur op een object slepen | vult het object, of kleurt de omtrek als u op de rand loslaat |
| Op het doorgestreepte vak klikken | verwijdert de vulling (rechtermuisknop: de omtrek) |

De pijlen aan de uiteinden schuiven door het palet en de knop helemaal rechts
toont alle kleuren tegelijk. Houd de aanwijzer boven een kleur om de naam en
waarden ervan te zien. De paletmenuknop helemaal links opent het paletmenu.

Het palet kan naar een andere rand van het venster worden verplaatst of zwevend
worden gemaakt: sleep het aan de gestippelde greep.

Aan elke kant kan het aan de rand van het venster liggen of naast de tekening, tussen de tekening en de panelen of de gereedschapskist aan die kant. Tijdens het slepen toont een blauwe band waar het komt; klik met rechts op de greep om een plaats uit een lijst te kiezen.

## Het documentpalet {#document-palette}

Elke kleur die u op een object toepast, komt in het documentpalet, dat met de
tekening wordt opgeslagen. De pipet ervan voegt de kleur onder de aanwijzer
toe, en het menu ervan bevat **Automatisch bijwerken** (om te stoppen met het
vanzelf toevoegen van kleuren), **Toevoegen uit selectie**, **Toevoegen uit
document**, **Kleur verwijderen** en **Palet herstellen** (behoudt alleen de
kleuren die de tekening nog gebruikt). U kunt ook kleuren of objecten erop
slepen. Zie [Aanpassingsopties](../customize/customization.md#color-palette)
om het te tonen of te verbergen.

## Het paneel Paletten {#palettes-docker}

Het paneel [[docker.palettes]] toont uw paletten, met het aantal kleuren in
elk. Open het met
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![Het paneel Paletten](../../img/color-palettes.png)

- Vink een palet aan om het onder het tekenvenster te tonen.
- [[docker.edit]] opent het in de paleteditor.
- [[menu.window.palette_open]] opent een paletbestand (`.tdpal`, `.gpl`,
  `.ase` of `.aco`).
- [[menu.window.palette_from_document]] en
  [[menu.window.palette_from_selection]] maken een palet van de kleuren die in
  de tekening of in de selectie gebruikt worden.
- De kleuren van het documentpalet staan onderaan; klik op een kleur om de
  selectie te vullen, klik met de rechtermuisknop om de omtrek te kleuren.

## Een palet bewerken {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]] opent
de paleteditor.

![De paleteditor](../../img/color-palettes-palette-editor.png)

1. Kies het [[dialog.palette]] dat u wilt bewerken, of klik op
   [[dialog.new_palette]] om er een te beginnen.
2. Typ de [[dialog.name]].
3. Klik op een kleur om die te selecteren; [[dialog.add_color]] voegt de
   huidige kleur toe en [[dialog.remove_color]] verwijdert de geselecteerde.
4. Klik op [[dialog.save_palette]] om het als bestand op te slaan.
5. Klik op [[dialog.close]].

Ingebouwde paletten kunnen niet worden bewerkt; maak een nieuw palet of bewerk
een kopie.
