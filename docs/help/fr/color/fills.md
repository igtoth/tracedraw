# Surfaces

Une surface peint l'intérieur d'un objet fermé. TraceDraw propose six sortes
de surface : [[docker.fill_none]], [[docker.fill_uniform]] (une seule couleur
unie), [[docker.fill_fountain]] (des couleurs qui se fondent les unes dans
les autres), [[docker.fill_pattern]], [[docker.fill_texture]] et
[[docker.fill_mesh]] (des couleurs qui s'écoulent entre les nœuds d'une
grille).

## Façons de remplir un objet {#ways}

- Cliquez sur une couleur de la palette pour donner à la sélection une
  surface uniforme.
- Utilisez la section [[docker.fill]] du panneau [[docker.properties]]
  (<kbd>F11</kbd>) : choisissez la sorte de surface dans la rangée d'images,
  puis ses réglages.
- Utilisez l'outil [[tool.interactive_fill]] (<kbd>G</kbd>) pour régler des
  dégradés directement sur l'objet.
- Utilisez l'outil [[tool.area_fill]] pour remplir une zone fermée par
  plusieurs objets.
- Utilisez l'outil [[tool.mesh_fill]] (<kbd>M</kbd>) pour une surface maille.

Quand rien n'est sélectionné, la surface choisie devient celle des nouveaux
objets.

## Surfaces uniformes {#uniform}

Une surface uniforme est une seule couleur, dans n'importe quel modèle de
couleur. Choisissez-la dans la palette, dans le panneau [[docker.color]]
(voir [Le panneau Couleur](color-docker.md)) ou dans la section surface du
panneau [[docker.properties]].

## Surfaces en dégradé {#gradient}

Un dégradé fond deux couleurs ou plus. Ses réglages, dans la section
surface du panneau [[docker.properties]] :

- Le type : [[fill.linear]] (le long d'une ligne), [[fill.radial]] (du
  centre vers l'extérieur), [[fill.conical]] (autour du centre),
  [[fill.square]] (en carrés croissants) ou [[fill.shaped]] (en suivant le
  contour de l'objet, du bord vers l'intérieur).
- [[docker.gradient_presets]] : des dégradés prêts à l'emploi comme point de
  départ.
- Les **arrêts** : les couleurs et leur position le long du dégradé, de 2 à
  64. Sélectionnez un arrêt pour changer sa couleur ou sa position ;
  [[docker.add_stop]] et [[docker.remove_stop]] ajoutent et suppriment des
  arrêts. [[docker.import_gradient]] reprend les couleurs d'un fichier de
  dégradé `.ggr`.
- L'**angle** des dégradés linéaires, coniques et carrés, le
  [[docker.edge_pad]] (la part de chaque extrémité en couleur unie) et le
  décalage du centre des dégradés radiaux, coniques et carrés.
- [[docker.repeat]] : [[docker.repeat_none]], [[docker.repeat_sawtooth]]
  (les couleurs repartent du début) ou [[docker.repeat_mirror]] (elles font
  l'aller-retour), avec le nombre de [[docker.repeat_count]].
- [[docker.bilinear]] : les couleurs vont de la première à la dernière, puis
  reviennent.
- [[docker.invert_colours]] : les couleurs vont dans l'ordre inverse.

Une bande sous le type montre toute la suite de couleurs telle qu'elle sera
dessinée.

## La boîte de dialogue Modifier le remplissage {#edit-fill}

<kbd>F11</kbd> ouvre la boîte de dialogue [[edit_fill.title]] sur la surface
de la sélection. Pour un dégradé, elle montre un aperçu, le
[[edit_fill.fill_picker]], le type, la répétition et la bande de couleurs :
cliquez sur un nœud pour le choisir, faites-le glisser pour le déplacer,
double-cliquez pour en ajouter ou en retirer un. Sous la bande : la couleur,
la transparence, la position et le [[edit_fill.midpoint]] du nœud, et
[[edit_fill.reverse]]. [[edit_fill.transform]] contient la rotation, le centre
et la marge du bord. **OK** applique ce que vous avez changé, avec
[[edit_fill.overprint]]. Tant que la boîte de dialogue est ouverte, la sélection montre le remplissage au fur et à mesure que vous le modifiez ; **Annuler** ou <kbd>Échap</kbd> rétablit l'ancien remplissage.

![La boîte de dialogue Modifier le remplissage](../../img/color-fills-edit-fill.png)

## L'outil Surface interactive {#interactive-fill}

L'outil [[tool.interactive_fill]] (<kbd>G</kbd>) modifie les surfaces
directement sur l'objet.

1. Choisissez l'outil et faites glisser sur un objet : vous obtenez un
   dégradé linéaire depuis l'endroit où vous appuyez jusqu'à celui où vous
   relâchez. Maintenez <kbd>Ctrl</kbd> pour contraindre l'angle par pas de
   15 degrés. Un clic donne la surface par défaut.
2. Faites glisser les nœuds sur l'objet pour modifier le dégradé : les deux
   nœuds carrés d'un dégradé linéaire changent sa direction et sa longueur ;
   le nœud d'un dégradé radial, conique ou carré déplace son centre.
3. Faites glisser une couleur de la palette sur un nœud pour le recolorer, ou
   sur la ligne entre les nœuds pour y ajouter une couleur.

La barre de propriétés contient la sorte de surface et ses réglages : la
couleur d'une surface uniforme ; pour un dégradé, le type, les couleurs de
début et de fin, l'angle et le remplissage des bords.

## L'outil Remplissage de zone {#area-fill}

L'outil [[tool.area_fill]] remplit n'importe quelle zone fermée, même formée
de plusieurs objets qui se chevauchent, en créant un nouvel objet de cette
zone. Les objets qui l'entourent restent tels quels.

1. Choisissez l'outil [[tool.area_fill]].
2. Choisissez la surface et le contour dans la barre de propriétés :
   **Spécifier** (la couleur, et pour le contour l'épaisseur, à côté),
   **Utiliser la valeur par défaut** (la surface et le contour des nouveaux
   objets ; une couleur quand ce serait aucune surface, afin que la zone soit
   visible) ou **Aucun**.
3. Cliquez à l'intérieur de la zone.

Le nouvel objet est placé au-dessus du calque actif et sélectionné. Un clic
sur un contour, ou dans une zone que rien ne ferme, ne crée rien.

## Surfaces maille {#mesh-fill}

Une surface maille est une grille de nœuds posée sur un objet, chaque nœud
ayant sa propre couleur et sa propre transparence ; les couleurs s'écoulent
en douceur entre les nœuds. Elle convient aux ombrages réalistes.

1. Choisissez l'outil [[tool.mesh_fill]] (<kbd>M</kbd>) et réglez les lignes
   et les colonnes dans la barre de propriétés (2 sur 2 au départ, jusqu'à
   50).
2. Cliquez sur un objet. Il reçoit une maille de la couleur de sa surface.
3. Cliquez sur un nœud pour le sélectionner, puis sur une couleur de la
   palette pour le colorer. Faites glisser des nœuds pour courber le flux des
   couleurs.
4. Double-cliquez dans la maille pour ajouter une ligne ou une colonne
   passant par le nœud le plus proche ; sélectionnez un nœud et appuyez sur
   <kbd>Delete</kbd> pour supprimer sa ligne et sa colonne.

La barre de propriétés propose aussi **Lisser la couleur**, qui fond les
couleurs plus doucement, la **transparence** du nœud sélectionné et
**Effacer la maille**, qui supprime la surface maille. Changer le nombre de
lignes et de colonnes d'une maille conserve ses couleurs.

## Surfaces motif et texture {#pattern-texture}

- Une surface [[docker.fill_pattern]] répète une vignette : deux couleurs,
  un motif vectoriel ou une image bitmap. Voir
  [Surfaces motif](patterns.md).
- Une surface [[docker.fill_texture]] est une texture générée, d'aspect
  naturel : [[texture.clouds]], [[texture.marble]], [[texture.noise]] ou
  [[texture.wood]], avec ses couleurs et son échelle.

## Plusieurs surfaces sur un objet {#several-fills}

Dans la section surface du panneau [[docker.properties]],
[[docker.add_fill]] peint une autre surface par-dessus celle de l'objet, en
commençant par une copie à modifier. Les surfaces suivantes s'affichent
comme "Surface 2", "Surface 3" et ainsi de suite, chacune avec son propre
éditeur et un bouton pour la supprimer. La section contour permet d'ajouter
d'autres contours de la même façon.

Des surfaces peuvent aussi être peintes sur les contours ; voir
[Régler les contours](../lines-shapes/outlines.md#outline-pen).
