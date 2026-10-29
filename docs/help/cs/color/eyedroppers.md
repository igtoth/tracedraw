# Kopírování barev a atributů kapátky

Nástroje kapátka naberou barvu nebo celou sadu atributů z jednoho objektu a
předají je jiným. Oba pracují ve dvou krocích, zobrazených na liště
vlastností: **vybrat** (nabrat vzorek) a **použít** (malovat jím). Po nabrání
vzorku se nástroj přepne na použití; dalším vzorkem začnete kliknutím na
tlačítko výběru. <kbd>Esc</kbd> vzorek zahodí a vrátí se k nástroji
[[tool.pick]].

Ukazatel je kapátko, když nástroj odebírá vzorek, a kbelík s barvou, když ho
používá. Obrázky dávají svou barvu i ve skupině nebo v rámečku.

## Kapátko barev {#color-eyedropper}

1. Zvolte nástroj [[tool.color_eyedropper]].
2. Kliknutím na objekt naberete jeho barvu: barvu jednotné výplně, první
   barvu přechodu nebo barvu obrysu. Na bitmapě nabere barvu pixelů pod
   ukazatelem, zprůměrovanou přes velikost vzorku nastavenou na liště
   vlastností (1 x 1, 2 x 2 nebo 5 x 5 pixelů). <kbd>Shift</kbd>+kliknutí
   nabere přednostně barvu obrysu.
3. Kliknutím na objekty je vyplníte touto barvou; <kbd>Shift</kbd>+kliknutím
   obarvíte jejich obrys (objekt bez obrysu dostane tenký obrys). Kliknutím
   dovnitř prázdného uzavřeného tvaru jej vyplníte.

Lišta vlastností ukazuje nabranou barvu.

Když pohybujete ukazatelem, abyste vzali barvu, malé pole vedle něj ukazuje
barvu pod ním: vzorek, název barvy, je-li to barva palety nebo přímá barva,
její hodnoty v barevném režimu kresby (CMYK nebo RGB) a její hexadecimální
hodnotu. Kapátko výběru barvy ukazuje totéž pole.

## Kapátko atributů {#attributes-eyedropper}

Nástroj [[tool.attributes_eyedropper]] kopíruje několik vlastností najednou z
jednoho objektu na jiné.

1. Zvolte nástroj [[tool.attributes_eyedropper]].
2. Na liště vlastností zvolte, co se má kopírovat:
   - **Vlastnosti**: obrys, výplň a text (písmo, velikost a styl; samotný
     text zůstane). Výchozí vše zapnuto.
   - **Transformace**: velikost, otočení a poloha. Výchozí vypnuto.
   - **Efekty**: průhlednost, vržený stín a živé efekty. Výchozí vypnuto.
3. Klikněte na objekt, ze kterého se kopíruje.
4. Klikněte na každý objekt, na který se kopíruje. <kbd>Shift</kbd>+kliknutí
   místo toho nabere vzorek z jiného objektu.

Každé kliknutí je jeden krok zpět.
