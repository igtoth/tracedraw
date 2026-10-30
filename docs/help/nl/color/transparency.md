# Transparantie

Met transparantie is te zien wat onder een object ligt. Ze kan overal gelijk
zijn, als een verloop over het object veranderen, of een patroon of textuur
volgen.

## Transparantie toepassen {#apply}

- Met het gereedschap [[tool.transparency]]: klik op een object en kies daarna
  een soort transparantie op de eigenschappenbalk, of sleep over het object voor
  een verlooptransparantie van dekkend (waar u indrukt) naar transparant (waar u
  loslaat). De knooppunten van het verloop verschijnen dan op het object, zoals bij het gereedschap [[tool.interactive_fill]]: sleep het begin, het eind of het ronde schuinknooppunt; sleep de balk tussen twee knooppunten (het middelpunt) om te verplaatsen waar de overgang halverwege is; dubbelklik op de lijn om een knooppunt toe te voegen, op een knooppunt om het te verwijderen.
- In het onderdeel [[docker.transparency]] van het paneel
  [[docker.properties]].

## Instellingen {#settings}

- De soort: [[transparency.none]], [[transparency.uniform]] (in het begin),
  [[transparency.fountain]] (lineair, radiaal, conisch, vierkant of vormgevolgd,
  zoals bij verloopvullingen), [[transparency.pattern]] of
  [[transparency.texture]].
- De hoeveelheid: 0 tot 100% (in het begin 50%). Bij een verloop zijn de
  kleurstops grijstinten: wit is dekkend, zwart is volledig transparant.
- [[transparency.merge_mode]]: hoe de kleuren zich mengen met wat eronder ligt.
  Normaal (in het begin) laat het eenvoudig doorschijnen; de andere, zoals
  Optellen, Aftrekken, Verschil, Vermenigvuldigen, Delen, Indien lichter,
  Indien donkerder, Tint, Verzadiging, Kleur en Omkeren, combineren de kleuren
  op andere manieren.
- Of ze geldt voor de [[transparency.target_fill]], de
  [[transparency.target_outline]] of [[transparency.target_all]].
- **Bevriezen**: houdt wat doorschijnt zoals het nu is, ook als de objecten
  eronder verplaatst worden.

Klik om ze te verwijderen op [[toolbar.clear_transparency]] aan het eind van de eigenschappenbalk (of kies [[transparency.none]] als soort).

Om de transparantie van een ander object te kopiëren, gebruikt u
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
Voor doorzichtige gebieden die door de helderheid van een object worden
bepaald, zie [Dekkingsmaskers](../objects/clip-frames.md#opacity-mask).
