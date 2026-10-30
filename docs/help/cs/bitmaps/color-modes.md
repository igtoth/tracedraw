# Změna barevného režimu bitmap

> *Strojový překlad: tato stránka může zaostávat za anglickou verzí.*

[[menu.bitmaps > menu.bitmaps.mode]] převede vybrané bitmapy do jiného
barevného režimu. Každá vybraná bitmapa se změní v jednom kroku zpět a
průhlednost se zachová. Režimy s nastavením otevřou dialogové okno s náhledy
před a po.

| Režim | Výsledek |
|---|---|
| [[menu.bitmaps.mode_bw]] | pouze černá a bílá (viz níže) |
| [[menu.bitmaps.mode_grayscale]] | odstíny šedi |
| [[menu.bitmaps.mode_duotone]] | jedna až čtyři barvy (viz níže) |
| [[menu.bitmaps.mode_paletted]] | nejvýše 256 barev (viz níže) |
| [[menu.bitmaps.mode_rgb]] | bitmapa přejde do režimu RGB; pixely zůstanou beze změny |
| [[menu.bitmaps.mode_lab]] | barvy Lab |
| [[menu.bitmaps.mode_cmyk]] | bitmapa přejde do režimu CMYK a její pixely ukážou barvy, které inkousty dokážou vytisknout, spočítané podle profilů správy barev |

Nabídka zaškrtne [[menu.bitmaps.mode_rgb]] nebo [[menu.bitmaps.mode_cmyk]], když jsou v tomto režimu všechny vybrané bitmapy, a stavový řádek uvádí režim vybrané bitmapy, například „Bitmap 600 x 400 px (CMYK)“. Obrázek CMYK ze souboru `.cdr` se otevře v režimu CMYK a soubory `.tdraw` režim uchovají. [[menu.tools > menu.tools.convert_colors]] nastaví režim každé převáděné bitmapy stejně (viz [Převod barev](../color/convert-colors.md)).

## Černobílý {#black-and-white}

![Dialogové okno Černobílý](../../img/bitmaps-color-modes-black-and-white.png)

Zvolte metodu převodu:

- **Perokresba**: bílá tam, kde je šeď na **prahu** (výchozí 128) nebo nad
  ním, jinde černá;
- **Uspořádané**: pravidelný vzor bodů;
- **Půltón** (výchozí): rastr kulatých, čárových nebo čtvercových bodů pod
  **úhlem** (45 stupňů) a s počtem **linek na palec** (60);
- **Rozdělení kardinality**: náhodně působící textura;
- **Jarvis**, **Stucki** a **Floyd-Steinberg**: rozptyl chyby, nejhladší
  výsledky u fotografií.

**Intenzita** (0 až 100, výchozí 50) výsledek zesvětlí nebo ztmaví.

## Duplex {#duotone}

![Dialogové okno Duplex](../../img/bitmaps-color-modes-duotone.png)

Duplex tiskne obrázek ve stupních šedi několika barvami, což dává bohaté tóny
s malým počtem barev.

1. Zvolte typ: Monotón, Duplex (výchozí), Triplex nebo Kvadruplex, pro jednu
   až čtyři barvy.
2. Kliknutím na barvu změníte její odstín.
3. Upravte její křivku: vodorovně je úroveň šedi (černá vlevo), svisle
   množství této barvy, které se vytiskne. Tažením v prázdném místě přidáte
   bod, tažením bodu jej posunete, dvojitým kliknutím na něj nebo jeho
   vytažením mimo mřížku jej odeberete. **Zobrazit vše** vykreslí i křivky
   ostatních barev.
4. **Uložit** a **Načíst** uchovávají barvy a křivky v souboru.

## S paletou {#paletted}

![Dialogové okno S paletou](../../img/bitmaps-color-modes-paletted.png)

- **Paleta**: Jednotné barvy, Standardní VGA (16 barev), Adaptivní a
  Optimalizovaná (vlastní barvy obrázku, zredukované na nastavený počet
  **barev**, nejvýše 256), Odstíny šedi, Systémová nebo Vlastní (paleta
  dokumentu).
- **Rozptylování**: Žádné, Uspořádané, Jarvis, Stucki nebo Floyd-Steinberg,
  s intenzitou rozptylu.

Výsledná paleta se zobrazí pod nastavením.

## Barvení černobílých bitmap {#monochrome}

Když jsou vybrány pouze dvoubarevné bitmapy, kliknutím na barvu v paletě
obarvíte jejich pozadí a kliknutím pravým tlačítkem jejich popředí.
