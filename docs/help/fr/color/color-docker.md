# Le panneau Couleur

Le panneau [[docker.color]] (<kbd>Shift+F11</kbd>) compose une couleur et
l'applique à la surface ou au contour de la sélection. L'outil
[[tool.outline_color]] (<kbd>Shift+F12</kbd>) l'ouvre aussi. Ouvrez-le
depuis [[menu.window > menu.window.dockers]].

![Le panneau Couleur](../../img/color-color-docker.png)

## Choisir une couleur {#choose}

1. Sélectionnez des objets. Le panneau part de la couleur de surface du
   premier.
2. Composez la nouvelle couleur dans l'une des vues :
   - [[docker.color_viewers]] : cliquez ou faites glisser dans le carré pour
     régler la saturation (de gauche à droite) et la luminosité (de bas en
     haut), et dans la bande à côté pour régler la teinte ;
   - [[docker.color_sliders]] : un curseur par composante du
     [[docker.model]] : RVB, CMJN (au départ), TSV (HSB), TSL (HSL), Lab, Niveaux
     de gris ou YIQ ;
   - [[docker.color_palettes]] : les échantillons d'une palette.

   Les trois boutons du haut affichent ou masquent chaque vue ; toute
   combinaison peut être affichée. Vous pouvez aussi taper une valeur
   `#rrggbb`.
3. Cliquez sur [[docker.fill]] ou [[docker.outline]] en bas du panneau pour
   appliquer la nouvelle couleur.
   Le verrou ([[docker.lock_colors]]) lie surface et contour, de sorte que l'un
   ou l'autre bouton applique la couleur aux deux. Le bouton "..."
   ([[docker.more_actions]]) déroule les autres actions : [[docker.no_fill]],
   [[docker.no_outline_action]], [[docker.swap_colors]],
   [[docker.add_to_document_palette]], [[docker.add_to_spot_colors]] et
   [[docker.add_to_color_styles]].

L'ancienne couleur et la nouvelle s'affichent côte à côte. Quand la nouvelle
couleur ne peut pas être imprimée exactement avec des encres CMJN, le
panneau affiche un avertissement.

## Prélever une couleur {#sample}

Cliquez sur [[docker.eyedropper]], puis n'importe où sur le dessin pour y
prendre la couleur. Cliquez ensuite sur [[docker.use_sampled]] pour en faire
la nouvelle couleur. Voir aussi [Pipettes](eyedroppers.md).

## Encres d'accompagnement {#spot-inks}

Sous les boutons, [[docker.spot_inks]] liste les encres d'accompagnement du
dessin ; voir [Couleurs d'accompagnement](spot-colors.md).
