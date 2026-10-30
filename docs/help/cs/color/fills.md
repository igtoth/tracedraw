# Výplně

Výplň barví vnitřek uzavřeného objektu. TraceDraw má šest druhů výplně:
[[docker.fill_none]], [[docker.fill_uniform]] (jedna plná barva),
[[docker.fill_fountain]] (barvy, které do sebe plynule přecházejí),
[[docker.fill_pattern]], [[docker.fill_texture]] a [[docker.fill_mesh]]
(barvy plynoucí mezi uzly mřížky).

## Způsoby vyplnění objektu {#ways}

- Kliknutím na barvu v paletě dáte výběru jednotnou výplň.
- Použijte oddíl [[docker.fill]] panelu [[docker.properties]]
  (<kbd>F11</kbd>): řadou obrázků zvolte druh výplně a poté její nastavení.
- Nástrojem [[tool.interactive_fill]] (<kbd>G</kbd>) nastavíte přechody
  přímo na objektu.
- Nástrojem [[tool.area_fill]] vyplníte oblast ohraničenou několika objekty.
- Nástrojem [[tool.mesh_fill]] (<kbd>M</kbd>) vytvoříte síťovou výplň.

Když není nic vybráno, zvolená výplň se stane výplní nových objektů.

## Jednotné výplně {#uniform}

Jednotná výplň je jedna barva v libovolném barevném modelu. Zvolte ji z
palety, z panelu [[docker.color]] (viz [Panel Barva](color-docker.md)) nebo
z oddílu výplně panelu [[docker.properties]].

## Přechodové výplně {#gradient}

Přechod prolíná dvě nebo více barev. Jeho nastavení v oddílu výplně panelu
[[docker.properties]]:

- Typ: [[fill.linear]] (podél přímky), [[fill.radial]] (od středu ven),
  [[fill.conical]] (kolem středu), [[fill.square]] (ve zvětšujících se
  čtvercích) nebo [[fill.shaped]] (podle obrysu objektu, od okraje dovnitř).
- [[docker.gradient_presets]]: hotové přechody, ze kterých lze vyjít.
- **Zarážky**: barvy a jejich místa podél přechodu, 2 až 64. Vyberte zarážku
  a změňte její barvu nebo polohu; [[docker.add_stop]] a
  [[docker.remove_stop]] zarážky přidávají a odebírají.
  [[docker.import_gradient]] převezme barvy ze souboru přechodu `.ggr`.
- **Úhel** lineárních, kuželových a čtvercových přechodů,
  [[docker.edge_pad]] (jak velká část každého konce je plnou barvou) a posun
  středu radiálních, kuželových a čtvercových přechodů.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]]
  (barvy běží znovu od začátku) nebo [[docker.repeat_mirror]] (běží tam a
  zpět), s počtem opakování [[docker.repeat_count]].
- [[docker.bilinear]]: barvy běží od první k poslední a zpět.
- [[docker.invert_colours]]: barvy běží v obráceném pořadí.

Pruh pod typem ukazuje celý průběh barev tak, jak se vykreslí.

## Dialog Upravit výplň {#edit-fill}

<kbd>F11</kbd> otevře dialog [[edit_fill.title]] s výplní výběru. U přechodu
ukazuje náhled, [[edit_fill.fill_picker]], typ, opakování a pás barev:
klepnutím uzel vyberete, tažením posunete, poklepáním uzel přidáte nebo
odstraníte. Pod pásem je barva, průhlednost, poloha a [[edit_fill.midpoint]]
uzlu a [[edit_fill.reverse]]. [[edit_fill.transform]] obsahuje otočení, střed
a okraj. **OK** použije, co jste změnili, i s [[edit_fill.overprint]]. Dokud je dialogové okno otevřené, výběr ukazuje výplň průběžně při změnách; **Zrušit** nebo <kbd>Esc</kbd> vrátí původní výplň.

![Dialog Upravit výplň](../../img/color-fills-edit-fill.png)

## Nástroj Interaktivní výplň {#interactive-fill}

Nástroj [[tool.interactive_fill]] (<kbd>G</kbd>) upravuje výplně přímo na
objektu.

1. Zvolte nástroj a táhněte přes objekt: vznikne lineární přechod od místa
   stisknutí k místu uvolnění. Podržením <kbd>Ctrl</kbd> omezíte úhel na
   kroky po 15 stupních. Kliknutí dá výchozí výplň.
2. Tažením uzlů na objektu přechod měníte: dva čtvercové uzly lineárního
   přechodu mění jeho směr a délku; uzel radiálního, kuželového nebo
   čtvercového přechodu posouvá jeho střed.
3. Přetažením barvy z palety na uzel jej přebarvíte, přetažením na čáru mezi
   uzly tam přidáte barvu.

Lišta vlastností obsahuje druh výplně a jeho nastavení: barvu jednotné
výplně; u přechodu typ, počáteční a koncovou barvu, úhel a okrajovou rezervu.

## Nástroj Výplň oblasti {#area-fill}

Nástroj [[tool.area_fill]] vyplní libovolnou uzavřenou oblast, i takovou,
kterou tvoří několik překrývajících se objektů, tím, že z této oblasti
vytvoří nový objekt. Okolní objekty zůstanou, jak jsou.

1. Zvolte nástroj [[tool.area_fill]].
2. Na liště vlastností zvolte výplň a obrys: **Určit** (barva a u obrysu
   vedle ní šířka), **Použít výchozí** (výplň a obrys nových objektů; je-li to
   žádná výplň, použije se barva, aby byla oblast vidět) nebo **Žádná**.
3. Klikněte dovnitř oblasti.

Nový objekt se umístí navrch aktivní vrstvy a je vybraný. Kliknutí na obrys
nebo do oblasti, kterou nic neohraničuje, nic nevytvoří.

## Síťové výplně {#mesh-fill}

Síťová výplň je mřížka uzlů přes objekt, každý uzel s vlastní barvou a
průhledností; barvy mezi uzly plynule přecházejí. Hodí se pro realistické
stínování.

1. Zvolte nástroj [[tool.mesh_fill]] (<kbd>M</kbd>) a na liště vlastností
   nastavte řádky a sloupce (výchozí 2 x 2, nejvýše 50).
2. Klikněte na objekt. Dostane síť v barvě své výplně.
3. Kliknutím na uzel jej vyberete, kliknutím na barvu v paletě jej obarvíte.
   Tažením uzlů ohýbáte tok barev.
4. Dvojitým kliknutím uvnitř sítě přidáte řádek nebo sloupec procházející
   nejbližším uzlem; vyberte uzel a stisknutím <kbd>Delete</kbd> odeberete
   jeho řádek a sloupec.

Lišta vlastností obsahuje také **Vyhlazenou barvu**, která barvy prolíná
jemněji, **průhlednost** vybraného uzlu a **Zrušit síť**, které síťovou
výplň odebere. Změna řádků a sloupců sítě zachová její barvy.

## Vzorkové a texturové výplně {#pattern-texture}

- Výplň [[docker.fill_pattern]] opakuje dlaždici: dvoubarevnou, vektorový
  vzorek nebo bitmapu. Viz [Vzorkové výplně](patterns.md).
- Výplň [[docker.fill_texture]] je generovaná, přirozeně působící textura:
  [[texture.clouds]], [[texture.marble]], [[texture.noise]] nebo
  [[texture.wood]], s vlastními barvami a měřítkem.

## Více výplní na jednom objektu {#several-fills}

V oddílu výplně panelu [[docker.properties]] přidá [[docker.add_fill]] přes
vlastní výplň objektu další výplň, která začíná jako kopie k úpravě. Další
výplně se zobrazují jako „Výplň 2“, „Výplň 3“ a tak dále, každá s vlastním
editorem a tlačítkem pro odebrání. Oddíl obrysu může stejným způsobem
přidávat další obrysy.

Výplně lze malovat i na obrysy; viz
[Nastavení obrysů](../lines-shapes/outlines.md#outline-pen).
