# Palettes de couleurs

La palette de couleurs en bas de la fenêtre est le moyen le plus rapide de
colorer des objets. En dessous, la **palette du document** rassemble les
couleurs utilisées par votre dessin. Vous pouvez ouvrir d'autres palettes,
créer les vôtres et les modifier.

## Utiliser la palette {#using}

| Action | Résultat |
|---|---|
| Cliquer sur une couleur | remplit la sélection (sans sélection, règle la surface des nouveaux objets) |
| Cliquer avec le bouton droit sur une couleur | colore le contour de la sélection |
| <kbd>Ctrl</kbd>+clic sur une couleur | mélange un dixième de cette couleur à la surface de la sélection |
| Cliquer longuement sur une couleur | affiche des nuances plus claires, plus foncées, plus ternes et plus vives au choix |
| Faire glisser une couleur sur un objet | remplit l'objet, ou colore son contour si elle est déposée sur le bord |
| Cliquer sur la case barrée | supprime la surface (clic droit : le contour) |

Les flèches aux extrémités font défiler la palette, et le bouton à
l'extrémité droite affiche toutes les couleurs à la fois. Survolez une
couleur pour voir son nom et ses valeurs. Le bouton de menu à l'extrémité
gauche ouvre le menu de la palette.

La palette peut être déplacée vers un autre bord de la fenêtre, ou rendue
flottante : faites-la glisser par sa poignée en pointillés.

De chaque côté, elle peut se placer au bord de la fenêtre ou contre le dessin, entre le dessin et les panneaux ou la boîte à outils de ce côté. Pendant que vous la faites glisser, une bande bleue montre où elle ira ; un clic droit sur sa poignée propose la liste des places.

## La palette du document {#document-palette}

Chaque couleur que vous appliquez à un objet rejoint la palette du document,
qui est enregistrée avec le dessin. Sa pipette ajoute la couleur située sous
le pointeur, et son menu propose **Mettre à jour automatiquement** (pour ne
plus ajouter les couleurs d'elles-mêmes), **Ajouter depuis la sélection**,
**Ajouter depuis le document**, **Supprimer la couleur** et **Réinitialiser
la palette** (ne garde que les couleurs que le dessin utilise encore). Vous
pouvez aussi y faire glisser des couleurs ou des objets. Pour l'afficher ou
la masquer, voir
[Options de personnalisation](../customize/customization.md#color-palette).

## Le panneau Palettes {#palettes-docker}

Le panneau [[docker.palettes]] liste vos palettes, avec le nombre de
couleurs de chacune. Ouvrez-le avec
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![Le panneau Palettes](../../img/color-palettes.png)

- Cochez une palette pour l'afficher sous la fenêtre de dessin.
- [[docker.edit]] l'ouvre dans l'éditeur de palette.
- [[menu.window.palette_open]] ouvre un fichier de palette (`.tdpal`,
  `.gpl`, `.ase` ou `.aco`).
- [[menu.window.palette_from_document]] et
  [[menu.window.palette_from_selection]] créent une palette des couleurs
  utilisées dans le dessin ou dans la sélection.
- Les couleurs de la palette du document s'affichent en bas ; cliquez sur
  l'une d'elles pour remplir la sélection, cliquez avec le bouton droit pour
  colorer le contour.

## Modifier une palette {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]]
ouvre l'éditeur de palette.

![L'éditeur de palette](../../img/color-palettes-palette-editor.png)

1. Choisissez la [[dialog.palette]] à modifier, ou cliquez sur
   [[dialog.new_palette]] pour en commencer une.
2. Tapez son [[dialog.name]].
3. Cliquez sur une couleur pour la sélectionner ; [[dialog.add_color]]
   ajoute la couleur actuelle et [[dialog.remove_color]] supprime la couleur
   sélectionnée.
4. Cliquez sur [[dialog.save_palette]] pour l'enregistrer dans un fichier.
5. Cliquez sur [[dialog.close]].

Les palettes intégrées ne peuvent pas être modifiées ; créez une nouvelle
palette ou modifiez une copie.
