# Barevné palety

Barevná paleta podél spodního okraje okna je nejrychlejší způsob, jak
obarvovat objekty. Pod ní **paleta dokumentu** shromažďuje barvy, které kresba
používá. Můžete otevřít další palety, vytvářet vlastní a upravovat je.

## Používání palety {#using}

| Akce | Výsledek |
|---|---|
| Kliknutí na barvu | vyplní výběr (když není nic vybráno, nastaví výplň nových objektů) |
| Kliknutí pravým tlačítkem na barvu | obarví obrys výběru |
| <kbd>Ctrl</kbd>+kliknutí na barvu | přimíchá desetinu barvy do výplně výběru |
| Kliknutí a podržení barvy | nabídne světlejší, tmavší, matnější a jasnější odstíny |
| Přetažení barvy na objekt | vyplní objekt, nebo obarví jeho obrys, je-li puštěna na okraj |
| Kliknutí na přeškrtnuté pole | odebere výplň (pravým tlačítkem: obrys) |

Šipky na koncích paletu posouvají a tlačítko na pravém konci zobrazí všechny
barvy najednou. Když na barvu najedete ukazatelem, uvidíte její název a
hodnoty. Tlačítko nabídky palety na levém konci otevře nabídku palety.

Paletu lze přesunout k jinému okraji okna nebo ji nechat plovoucí: táhněte ji
za tečkovaný úchyt.

Na každé straně může ležet u okraje okna nebo vedle kresby, mezi kresbou a panely nebo panelem nástrojů na té straně. Při tažení ukazuje modrý pruh, kam se umístí; klepnutím pravým tlačítkem na úchyt vyberete místo ze seznamu.

## Paleta dokumentu {#document-palette}

Každá barva, kterou použijete na objekt, se přidá do palety dokumentu, která
se ukládá s kresbou. Její kapátko přidá barvu pod ukazatelem a její nabídka
obsahuje **Automaticky aktualizovat** (pro vypnutí samočinného přidávání
barev), **Přidat z výběru**, **Přidat z dokumentu**, **Odstranit barvu** a
**Obnovit paletu** (ponechá jen barvy, které kresba stále používá). Můžete na
ni také přetahovat barvy nebo objekty. Jak ji zobrazit nebo skrýt, viz
[Možnosti přizpůsobení](../customize/customization.md#color-palette).

## Panel Palety {#palettes-docker}

Panel [[docker.palettes]] uvádí palety, které máte, s počtem barev v každé.
Otevřete jej příkazem
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![Panel Palety](../../img/color-palettes.png)

- Zaškrtnutím paletu zobrazíte pod oknem kresby.
- [[docker.edit]] ji otevře v editoru palet.
- [[menu.window.palette_open]] otevře soubor palety (`.tdpal`, `.gpl`,
  `.ase` nebo `.aco`).
- [[menu.window.palette_from_document]] a
  [[menu.window.palette_from_selection]] vytvoří paletu z barev použitých v
  kresbě nebo ve výběru.
- Barvy palety dokumentu jsou zobrazeny dole; kliknutím na jednu vyplníte
  výběr, kliknutím pravým tlačítkem obarvíte obrys.

## Úprava palety {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]]
otevře editor palet.

![Editor palet](../../img/color-palettes-palette-editor.png)

1. Zvolte paletu k úpravě v poli [[dialog.palette]], nebo kliknutím na
   [[dialog.new_palette]] začněte novou.
2. Zadejte její [[dialog.name]].
3. Kliknutím na barvu ji vyberete; [[dialog.add_color]] přidá aktuální barvu
   a [[dialog.remove_color]] odebere vybranou.
4. Kliknutím na [[dialog.save_palette]] ji uložíte jako soubor.
5. Klikněte na [[dialog.close]].

Vestavěné palety nelze upravovat; vytvořte novou paletu nebo upravte kopii.
