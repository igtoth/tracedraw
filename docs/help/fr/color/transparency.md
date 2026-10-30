# Transparence

La transparence laisse voir à travers un objet ce qui se trouve en dessous.
Elle peut être la même partout, varier sur l'objet comme un dégradé, ou
suivre un motif ou une texture.

## Appliquer une transparence {#apply}

- Avec l'outil [[tool.transparency]] : cliquez sur un objet, puis choisissez
  une sorte de transparence dans la barre de propriétés, ou faites glisser
  sur l'objet pour une transparence en dégradé, opaque là où vous appuyez et
  transparente là où vous relâchez. Les nœuds du dégradé s'affichent alors sur l'objet, comme avec l'outil [[tool.interactive_fill]] : faites glisser le début, la fin ou le nœud rond d'inclinaison ; faites glisser la barre entre deux nœuds (le point milieu) pour déplacer l'endroit où le mélange est à moitié ; double-cliquez sur la ligne pour ajouter un nœud, sur un nœud pour le supprimer.
- Dans la section [[docker.transparency]] du panneau [[docker.properties]].

## Réglages {#settings}

- La sorte : [[transparency.none]], [[transparency.uniform]] (au départ),
  [[transparency.fountain]] (linéaire, radiale, conique, carrée ou formée,
  comme les surfaces en dégradé), [[transparency.pattern]] ou
  [[transparency.texture]].
- La quantité : de 0 à 100 % (50 % au départ). Pour un dégradé, les arrêts
  sont des niveaux de gris : le blanc est opaque, le noir totalement
  transparent.
- [[transparency.merge_mode]] : comment les couleurs se mélangent avec ce
  qui se trouve en dessous. Normal (au départ) laisse simplement voir à
  travers ; les autres, comme Ajouter, Soustraire, Différence, Multiplier,
  Diviser, Si plus clair, Si plus foncé, Teinte, Saturation, Couleur et
  Inverser, combinent les couleurs d'autres façons.
- Si elle s'applique à la [[transparency.target_fill]], au
  [[transparency.target_outline]] ou à [[transparency.target_all]].
- **Figer** : garde ce qui transparaît tel qu'il est maintenant, même si les
  objets situés en dessous se déplacent.

Pour la retirer, cliquez sur [[toolbar.clear_transparency]] au bout de la barre de propriétés (ou choisissez [[transparency.none]] comme type).

Pour copier la transparence d'un autre objet, utilisez
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
Pour des zones transparentes définies par la luminosité d'un objet, voir
[Masques d'opacité](../objects/clip-frames.md#opacity-mask).
