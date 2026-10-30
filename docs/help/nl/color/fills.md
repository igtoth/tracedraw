# Vullingen

Een vulling kleurt de binnenkant van een gesloten object. TraceDraw kent zes
soorten vulling: [[docker.fill_none]], [[docker.fill_uniform]] (één egale
kleur), [[docker.fill_fountain]] (kleuren die in elkaar overlopen),
[[docker.fill_pattern]], [[docker.fill_texture]] en [[docker.fill_mesh]]
(kleuren die tussen de knooppunten van een raster vloeien).

## Manieren om een object te vullen {#ways}

- Klik op een kleur van het palet om de selectie een uniforme vulling te geven.
- Gebruik het onderdeel [[docker.fill]] van het paneel [[docker.properties]]
  (<kbd>F11</kbd>): kies de soort vulling met de rij afbeeldingen en daarna de
  instellingen ervan.
- Gebruik het gereedschap [[tool.interactive_fill]] (<kbd>G</kbd>) om
  verlopen op het object zelf in te stellen.
- Gebruik het gereedschap [[tool.area_fill]] om een gebied te vullen dat door
  meerdere objecten wordt omsloten.
- Gebruik het gereedschap [[tool.mesh_fill]] (<kbd>M</kbd>) voor een
  meshvulling.

Als er niets geselecteerd is, wordt de vulling die u kiest de vulling van
nieuwe objecten.

## Uniforme vullingen {#uniform}

Een uniforme vulling is één kleur, in elk kleurmodel. Kies haar in het palet,
in het paneel [[docker.color]] (zie [Het paneel Kleur](color-docker.md)) of in
het vulonderdeel van het paneel [[docker.properties]].

## Verloopvullingen {#gradient}

Een verloop laat twee of meer kleuren in elkaar overlopen. De instellingen
ervan, in het vulonderdeel van het paneel [[docker.properties]]:

- Het type: [[fill.linear]] (langs een lijn), [[fill.radial]] (vanuit het
  midden naar buiten), [[fill.conical]] (rond het midden), [[fill.square]] (in
  groeiende vierkanten) of [[fill.shaped]] (de omtrek van het object volgend,
  van de rand naar binnen).
- [[docker.gradient_presets]]: kant-en-klare verlopen om mee te beginnen.
- De **kleurstops**: de kleuren en hun plaats langs het verloop, 2 tot 64.
  Selecteer een kleurstop om de kleur of positie ervan te wijzigen;
  [[docker.add_stop]] en [[docker.remove_stop]] voegen kleurstops toe en
  verwijderen ze. [[docker.import_gradient]] neemt de kleuren over uit een
  verloopbestand `.ggr`.
- De **hoek** van lineaire, conische en vierkante verlopen, de
  [[docker.edge_pad]] (hoeveel van elk uiteinde egale kleur is) en de
  verschuiving van het middelpunt van radiale, conische en vierkante verlopen.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]] (de
  kleuren beginnen opnieuw vanaf het begin) of [[docker.repeat_mirror]] (ze
  lopen heen en weer), met het aantal [[docker.repeat_count]].
- [[docker.bilinear]]: de kleuren lopen van de eerste naar de laatste en terug.
- [[docker.invert_colours]]: de kleuren lopen in omgekeerde volgorde.

Een strook onder het type toont het hele kleurverloop zoals het getekend wordt.

## Het dialoogvenster Vulling bewerken {#edit-fill}

<kbd>F11</kbd> opent het dialoogvenster [[edit_fill.title]] op de vulling van
de selectie. Voor een kleurverloop toont het een voorbeeld, de
[[edit_fill.fill_picker]], het type, de herhaling en de kleurenband: klik op
een knooppunt om het te kiezen, sleep om het te verplaatsen, dubbelklik om er
een toe te voegen of te verwijderen. Onder de band: kleur, transparantie,
positie en [[edit_fill.midpoint]] van het knooppunt, en [[edit_fill.reverse]].
[[edit_fill.transform]] bevat draaiing, midden en randmarge. **OK** past toe
wat u veranderde, met [[edit_fill.overprint]]. Zolang het dialoogvenster open is, toont de selectie de vulling terwijl u die wijzigt; **Annuleren** of <kbd>Esc</kbd> zet de oude vulling terug.

![Het dialoogvenster Vulling bewerken](../../img/color-fills-edit-fill.png)

## Het gereedschap Interactieve vulling {#interactive-fill}

Het gereedschap [[tool.interactive_fill]] (<kbd>G</kbd>) bewerkt vullingen op
het object.

1. Kies het gereedschap en sleep over een object: u krijgt een lineair verloop
   van waar u indrukt tot waar u loslaat. Houd <kbd>Ctrl</kbd> ingedrukt om de
   hoek in stappen van 15 graden te houden. Een klik geeft de standaardvulling.
2. Sleep de knooppunten op het object om het verloop te wijzigen: de twee
   vierkante knooppunten van een lineair verloop veranderen de richting en
   lengte ervan; het knooppunt van een radiaal, conisch of vierkant verloop
   verplaatst het middelpunt.
3. Sleep een paletkleur op een knooppunt om het een andere kleur te geven, of
   op de lijn tussen de knooppunten om daar een kleur toe te voegen.

De eigenschappenbalk bevat de soort vulling en de instellingen ervan: de kleur
van een uniforme vulling; voor een verloop het type, de begin- en eindkleur, de
hoek en de randopvulling.

## Het gereedschap Gebiedsvulling {#area-fill}

Het gereedschap [[tool.area_fill]] vult elk gesloten gebied, zelfs een gebied
dat door meerdere overlappende objecten wordt gevormd, door van dat gebied een
nieuw object te maken. De objecten eromheen blijven zoals ze zijn.

1. Kies het gereedschap [[tool.area_fill]].
2. Kies de vulling en omtrek op de eigenschappenbalk: **Opgeven** (de kleur,
   en voor de omtrek de breedte, ernaast), **Standaard gebruiken** (de vulling
   en omtrek van nieuwe objecten; een kleur als dat geen vulling is, zodat het
   gebied zichtbaar wordt) of **Geen**.
3. Klik binnen het gebied.

Het nieuwe object komt bovenaan de actieve laag en wordt geselecteerd. Een klik
op een omtrek, of in een gebied dat door niets wordt omsloten, maakt niets.

## Meshvullingen {#mesh-fill}

Een meshvulling is een raster van knooppunten over een object, elk knooppunt
met een eigen kleur en transparantie; de kleuren vloeien vloeiend tussen de
knooppunten. Het is geschikt voor realistische schaduwwerking.

1. Kies het gereedschap [[tool.mesh_fill]] (<kbd>M</kbd>) en stel de rijen en
   kolommen in op de eigenschappenbalk (in het begin 2 bij 2, tot 50).
2. Klik op een object. Het krijgt een mesh in de kleur van zijn vulling.
3. Klik op een knooppunt om het te selecteren en klik daarna op een kleur van
   het palet om het te kleuren. Sleep knooppunten om de kleurstroom te buigen.
4. Dubbelklik binnen de mesh om een rij of kolom toe te voegen door het
   dichtstbijzijnde knooppunt; selecteer een knooppunt en druk op
   <kbd>Delete</kbd> om de rij en kolom ervan te verwijderen.

De eigenschappenbalk heeft ook **Vloeiende kleur**, die de kleuren zachter laat
overlopen, de **transparantie** van het geselecteerde knooppunt, en **Mesh
wissen**, die de meshvulling verwijdert. Het wijzigen van de rijen en kolommen
van een mesh behoudt de kleuren.

## Patroon- en textuurvullingen {#pattern-texture}

- Een vulling [[docker.fill_pattern]] herhaalt een tegel: twee kleuren, een
  vectorpatroon of een bitmap. Zie [Patroonvullingen](patterns.md).
- Een vulling [[docker.fill_texture]] is een gegenereerde, natuurlijk ogende
  textuur: [[texture.clouds]], [[texture.marble]], [[texture.noise]] of
  [[texture.wood]], met de kleuren en schaal ervan.

## Meerdere vullingen op één object {#several-fills}

In het vulonderdeel van het paneel [[docker.properties]] kleurt
[[docker.add_fill]] een extra vulling over de eigen vulling van het object,
beginnend als een kopie om te wijzigen. Verdere vullingen verschijnen als
"Vulling 2", "Vulling 3" enzovoort, elk met een eigen editor en een knop om
haar te verwijderen. In het omtrekonderdeel kunt u op dezelfde manier extra
omtrekken toevoegen.

Vullingen kunnen ook op omtrekken worden aangebracht; zie
[Omtrekken instellen](../lines-shapes/outlines.md#outline-pen).
