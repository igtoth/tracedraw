# Průhlednost

Průhlednost nechá prosvítat to, co leží pod objektem. Může být všude stejná,
měnit se napříč objektem jako přechod, nebo sledovat vzorek či texturu.

## Použití průhlednosti {#apply}

- Nástrojem [[tool.transparency]]: klikněte na objekt a pak na liště
  vlastností zvolte druh průhlednosti, nebo táhněte přes objekt a vznikne
  přechodová průhlednost od neprůhledné (kde stisknete) po průhlednou (kde
  uvolníte). Uzly přechodu se pak ukážou na objektu jako u nástroje [[tool.interactive_fill]]: táhněte začátek, konec nebo kulatý uzel zkosení; tažením příčky mezi dvěma uzly (středový bod) posunete místo, kde je přechod v polovině; dvojklik na čáru přidá uzel, na uzel ho odebere.
- V oddílu [[docker.transparency]] panelu [[docker.properties]].

## Nastavení {#settings}

- Druh: [[transparency.none]], [[transparency.uniform]] (výchozí),
  [[transparency.fountain]] (lineární, radiální, kuželový, čtvercový nebo
  tvarovaný, jako u přechodových výplní), [[transparency.pattern]] nebo
  [[transparency.texture]].
- Míra: 0 až 100 % (výchozí 50 %). U přechodu jsou zarážky odstíny šedi:
  bílá je neprůhledná, černá zcela průhledná.
- [[transparency.merge_mode]]: jak se barvy mísí s tím, co leží pod nimi.
  Normální (výchozí) jednoduše prosvítá; ostatní, například Přidat, Odečíst,
  Rozdíl, Násobit, Dělit, Je-li světlejší, Je-li tmavší, Odstín, Sytost,
  Barva a Invertovat, kombinují barvy jinými způsoby.
- Zda se týká možnosti [[transparency.target_fill]],
  [[transparency.target_outline]] nebo [[transparency.target_all]].
- **Zmrazit**: zachová to, co prosvítá, v současné podobě, i když se objekty
  pod ním posunou.

Chcete-li ji odstranit, klikněte na [[toolbar.clear_transparency]] na konci panelu vlastností (nebo jako druh zvolte [[transparency.none]]).

Chcete-li zkopírovat průhlednost jiného objektu, použijte
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
O průhledných oblastech určených jasem objektu viz
[Masky neprůhlednosti](../objects/clip-frames.md#opacity-mask).
