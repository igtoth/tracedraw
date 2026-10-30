# De kleurmodus van bitmaps wijzigen

> *Machinevertaling: deze pagina kan achterlopen op de Engelse versie.*

[[menu.bitmaps > menu.bitmaps.mode]] zet de geselecteerde bitmaps om naar een
andere kleurmodus. Elke geselecteerde bitmap verandert in één
ongedaan-makenstap en de transparantie blijft behouden. De modi met
instellingen openen een dialoogvenster met een voorbeeld vooraf en achteraf;
de instellingen staan in een paneel rechts en daaronder zet
[[dialog.reset]] ze terug, terwijl de twee pijlen de wijzigingen in het
dialoogvenster ongedaan maken en opnieuw uitvoeren.

| Modus | Resultaat |
|---|---|
| [[menu.bitmaps.mode_bw]] | alleen zwart en wit (zie hieronder) |
| [[menu.bitmaps.mode_grayscale]] | grijstinten |
| [[menu.bitmaps.mode_duotone]] | één tot vier inkten (zie hieronder) |
| [[menu.bitmaps.mode_paletted]] | maximaal 256 kleuren (zie hieronder) |
| [[menu.bitmaps.mode_rgb]] | de modus van de bitmap wordt RGB; de pixels blijven zoals ze zijn |
| [[menu.bitmaps.mode_lab]] | Lab-kleur |
| [[menu.bitmaps.mode_cmyk]] | de modus van de bitmap wordt CMYK en de pixels tonen de kleuren die de inkten kunnen drukken, berekend met de kleurbeheerprofielen |

Het menu vinkt [[menu.bitmaps.mode_rgb]] of [[menu.bitmaps.mode_cmyk]] aan wanneer alle geselecteerde bitmaps in die modus staan, en de statusbalk noemt de modus van een geselecteerde bitmap, zoals "Bitmap 600 x 400 px (CMYK)". Een CMYK-afbeelding uit een `.cdr`-bestand opent in de CMYK-modus, en `.tdraw`-bestanden bewaren de modus. [[menu.tools > menu.tools.convert_colors]] zet de modus van elke bitmap die het omzet op dezelfde manier (zie [Kleuren omzetten](../color/convert-colors.md)).

## Zwart-wit {#black-and-white}

![Het dialoogvenster Zwart-wit](../../img/bitmaps-color-modes-black-and-white.png)

Kies de omzettingsmethode:

- **Lijntekening**: wit waar het grijs op of boven de **drempel** ligt (in het
  begin 128), elders zwart;
- **Geordend**: een regelmatig patroon van stippen;
- **Halftoon** (in het begin): een raster van ronde, lijnvormige of vierkante
  stippen onder een **hoek** (45 graden) en met een aantal **lijnen per inch**
  (60);
- **Kardinaliteitsverdeling**: een willekeurig ogende textuur;
- **Jarvis**, **Stucki** en **Floyd-Steinberg**: foutdiffusie, de vloeiendste
  resultaten voor foto's.

**Intensiteit** (0 tot 100, in het begin 50) maakt het resultaat lichter of
donkerder.

## Duotoon {#duotone}

![Het dialoogvenster Duotoon](../../img/bitmaps-color-modes-duotone.png)

Een duotoon drukt een grijswaardenafbeelding af met enkele inkten, wat rijke
tinten geeft met weinig kleuren.

1. Kies het type: Monotoon, Duotoon (in het begin), Tritoon of Quadtoon, voor
   één tot vier inkten.
2. Klik op een inkt om de kleur ervan te wijzigen.
3. Bewerk de curve: horizontaal staat het grijsniveau (zwart links), verticaal
   hoeveel van die inkt wordt afgedrukt. Sleep op een lege plek om een punt toe
   te voegen, sleep een punt om het te verplaatsen, dubbelklik erop of sleep het
   van het raster om het te verwijderen. **Alles tonen** tekent ook de curven
   van de andere inkten.
4. **Opslaan** en **Laden** bewaren de inkten en curven in een bestand.

## Palet {#paletted}

![Het dialoogvenster Palet](../../img/bitmaps-color-modes-paletted.png)

- **Palet**: Uniforme kleuren, Standaard VGA (16 kleuren), Adaptief en
  Geoptimaliseerd (de eigen kleuren van de afbeelding, teruggebracht tot het
  aantal **kleuren** dat u instelt, maximaal 256), Grijswaarden, Systeem of
  Aangepast (het documentpalet).
- **Dithering**: Geen, Geordend, Jarvis, Stucki of Floyd-Steinberg, met de
  ditheringintensiteit.

Het resulterende palet verschijnt onder de instellingen.

## Zwart-witbitmaps kleuren {#monochrome}

Als alleen tweekleurige bitmaps geselecteerd zijn, klikt u op een paletkleur om
hun achtergrond te kleuren en klikt u met de rechtermuisknop om hun voorgrond
te kleuren.
