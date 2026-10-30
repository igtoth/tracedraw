# Vektorizace bitmap

Vektorizace promění bitmapu ve vektorové tvary, které můžete upravovat,
přebarvovat a měnit jejich velikost bez ztráty kvality: naskenované logo,
skicu, fotografii.

## Rychlá vektorizace {#quick-trace}

Vyberte bitmapu a zvolte [[menu.bitmaps > menu.bitmaps.quick_trace]]. Použije
se předvolba zvolená na stránce [[options.bitmap_tracing]] v globálních
možnostech (výchozí Clipart).

[[menu.bitmaps.repeat_last_trace]] obtáhne vybranou bitmapu znovu jako poslední obtažení, bez dialogového okna, a [[menu.bitmaps.last_trace_settings]] otevře dialogové okno s nastavením posledního obtažení. Tyto příkazy má i místní nabídka bitmapy, spolu s [[menu.bitmaps.centerline_trace]] a [[menu.bitmaps.outline_trace]].

## Dialogové okno vektorizace bitmapy {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] a
[[menu.bitmaps.centerline_trace]] obsahují seznam předvoleb; výběrem jedné
otevřete dialogové okno vektorizace.

- **Vektorizace obrysu** vyplní barevné oblasti tvary:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] a [[menu.bitmaps.trace_high_quality]].
- **Vektorizace středové čáry** sleduje čáry otevřenými křivkami:
  [[menu.bitmaps.trace_technical]] a [[menu.bitmaps.trace_line_drawing]].

![Dialogové okno vektorizace bitmapy](../../img/bitmaps-tracing.png)

Dialogové okno je okno, jehož velikost můžete měnit a maximalizovat. Náhled
vyplňuje jeho levou část, nastavení sedí v panelu vpravo a vektorizace se
provede znovu, kdykoli změníte nastavení: pro zobrazení výsledku není třeba
nic stisknout. Pod náhledem řádek počítá **křivky**, **uzly** a **barvy**
vektorizace. Kliknutím na [[dialog.ok]] umístíte tvary přes bitmapu,
nejtmavší jako první; [[dialog.cancel]] ponechá bitmapu, jak je.

## Náhled {#preview}

Seznam [[dialog.trace_preview_label]] volí, co náhled ukazuje:

- [[dialog.trace_view_before_after]]: bitmapa nahoře a její vektorizace dole,
  nebo [[dialog.view_side_by_side]]; [[dialog.before]] ukazuje samotnou
  bitmapu.
- [[dialog.trace_view_large]]: samotná vektorizace, tak velká jako náhled.
- [[dialog.trace_view_wireframe]]: křivky vektorizace nakreslené tenkými
  čarami přes bitmapu. Vedle seznamu se objeví posuvník
  [[dialog.trace_overlay]]: určuje, jak silně bitmapa pod čarami prosvítá, od
  0 (jen čáry) do 100 (bitmapa, jak je), výchozí 80.

Tlačítka za seznamem přizpůsobí zobrazení, oddálí, přiblíží a posouvají.
Kolečko zvětšuje kolem ukazatele a tažením obrázek posunete; obě části se
pohybují společně. Průhledná místa ukazují šachovnici.

## Nastavení {#settings}

Karta [[dialog.trace_tab_settings]] obsahuje:

- [[dialog.trace_type]] Obrys nebo Středová čára a [[dialog.trace_image_type]]
  předvolbu. Výběr předvolby nastaví posuvníky níže na hodnoty vhodné pro
  daný druh obrázku.
- [[dialog.detail]] (0 až 100): více detailů zachová menší oblasti a jemnější
  čáry; méně detailů vyhladí skvrnky a zrno.
- [[dialog.smoothing]] (0 až 100, výchozí 25): hladší křivky s méně uzly.
- [[dialog.corner_smoothness]] (0 až 100): kulatější rohy.
- [[dialog.trace_remove]]: co z vektorizace vynechat. **Barva pozadí**
  odstraní pozadí tam, kde sahá k okraji obrázku, **Celá barva z obrázku**
  odstraní každou oblast této barvy, **Žádné** vektorizuje vše.
  [[dialog.trace_remove_auto]] najde barvu pozadí podél okraje obrázku;
  [[dialog.trace_remove_specify]] použije barvu vzorku, kterou můžete změnit
  nebo nabrat z náhledu kapátkem vedle něj.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (výchozí zapnuto)
  udělá z každé oblasti jedné barvy jednu křivku; vypnuto, každá samostatná
  plocha je vlastní křivkou. [[dialog.trace_remove_overlap]] drží každý tvar u
  jeho vlastních pixelů; vypnuto, tvary sahají kousek pod tvary nad nimi, aby
  mezi sousedy nebyly mezery. [[dialog.trace_group_by_color]] dá křivky každé
  barvy do vlastní skupiny; je dostupné, když oblasti jedné barvy nejsou už
  sloučeny.
- [[dialog.delete_original]]: bitmapa po umístění vektorizace zmizí.

Nastavení, která se na aktuální typ vektorizace nevztahují, jsou zašedlá:
vektorizace středovou čarou čte jen detail a vyhlazení.

## Barvy {#colors}

Karta [[dialog.trace_tab_colors]] nastavuje [[dialog.color_mode]] (RGB, odstíny
šedi nebo černobílá) a [[dialog.trace_number_of_colors]] (2 až 256) a
vypisuje barvy vektorizace s počtem křivek každé z nich. Zaškrtněte dvě nebo
více barev a klikněte na [[dialog.trace_merge_colors]], abyste je sloučili do
první; kliknutím na vzorek barvy nakreslíte její křivky jinou barvou. Změna
nastavení, která obrázek vektorizuje znovu, tyto úpravy zapomene, protože
patřily k předchozí vektorizaci.

## Obnovit, zpět a znovu {#steps}

Pod panelem nastavení [[dialog.reset]] vrátí nastavení předvolby a dvě šipky
procházejí zpět a vpřed každou změnu provedenou v dialogovém okně: každé
puštění posuvníku, volba ze seznamu, zaškrtávací políčko, úprava barvy a
Obnovit je jeden krok. Změna provedená po kroku zpět zahodí kroky, které
byly vráceny.

[[dialog.trace_options]] otevře stránku [[options.bitmap_tracing]] globálních
voleb, kde zvolíte předvolbu rychlé vektorizace a volby, se kterými každá
vektorizace začíná, a pak se vrátí do dialogového okna. Tlačítko
[[dialog.help_button]] otevře tuto stránku.
